//! Settings files the Python services can read and write too: `settings-<VERSION>.json` with a `VERSION` field,
//! highest version first, migrations from older ones, files from the future refused, and an atomic write.

use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use blueos_idl::{
    blueos_msgs::{CommandAck, SettingsEnvelope, UpdateSettingsFeedback, UpdateSettingsGoal, UpdateSettingsResult},
    cdr,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use tokio::sync::watch;
use zenoh::query::Query;

use crate::{
    Error, Service, State, jobs, keys,
    routes::{self, action_schema},
};

const PREFIX: &str = "settings-";
const UPDATE_SETTINGS: &str = "UpdateSettings";

pub trait Settings: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static {
    const VERSION: u64;

    /// Moves a document of an older `VERSION` to this one, updating its `VERSION`, as the Python `migrate` does.
    fn migrate(_document: &mut Map<String, Value>) -> Result<(), SettingsError> {
        Err(SettingsError::MigrationFail)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings file has no VERSION")]
    BadSettingsFile,
    #[error("settings file has an invalid VERSION")]
    BadAttributes,
    #[error("settings file comes from a future version {0}")]
    FromTheFuture(u64),
    #[error("migration did not reach the current version")]
    MigrationFail,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn path<S: Settings>(folder: &Path) -> PathBuf {
    folder.join(format!("{PREFIX}{}.json", S::VERSION))
}

/// Loads the newest valid file, or saves and returns the default when there is none.
pub fn load<S: Settings>(folder: &Path) -> Result<S, SettingsError> {
    std::fs::create_dir_all(folder)?;
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "tmp") {
            std::fs::remove_file(&path)?;
            continue;
        }
        let version = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(PREFIX)?.strip_suffix(".json")?.parse::<u64>().ok());
        if let Some(version) = version {
            candidates.push((version, path));
        }
    }
    candidates.sort_unstable_by_key(|(version, _)| std::cmp::Reverse(*version));
    for (_, candidate) in candidates {
        match load_file::<S>(&candidate) {
            Ok(settings) => return Ok(settings),
            Err(error) => tracing::debug!(%error, path = %candidate.display(), "Invalid settings file"),
        }
    }
    let settings = S::default();
    save(folder, &settings)?;
    Ok(settings)
}

pub fn load_file<S: Settings>(path: &Path) -> Result<S, SettingsError> {
    let Value::Object(document) = serde_json::from_reader(File::open(path)?)? else {
        return Err(SettingsError::BadSettingsFile);
    };
    from_document(document)
}

/// Parses a document with its `VERSION`, migrating it when it is older.
pub fn from_document<S: Settings>(mut document: Map<String, Value>) -> Result<S, SettingsError> {
    let version = document
        .get("VERSION")
        .ok_or(SettingsError::BadSettingsFile)?
        .as_u64()
        .filter(|version| *version > 0)
        .ok_or(SettingsError::BadAttributes)?;
    if version > S::VERSION {
        return Err(SettingsError::FromTheFuture(version));
    }
    if version < S::VERSION {
        S::migrate(&mut document)?;
    }
    if document.remove("VERSION").and_then(|version| version.as_u64()) != Some(S::VERSION) {
        return Err(SettingsError::MigrationFail);
    }
    Ok(serde_json::from_value(Value::Object(document))?)
}

/// The document as the file holds it: `VERSION` first, then the fields.
pub fn to_document<S: Settings>(settings: &S) -> Result<Map<String, Value>, SettingsError> {
    let mut document = Map::from_iter([("VERSION".to_owned(), Value::from(S::VERSION))]);
    if let Value::Object(fields) = serde_json::to_value(settings)? {
        document.extend(fields);
    }
    Ok(document)
}

/// Writes a temporary file and renames it over the settings file, so a power loss never leaves half a file.
pub fn save<S: Settings>(folder: &Path, settings: &S) -> Result<(), SettingsError> {
    let path = path::<S>(folder);
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)?;
    let mut serializer =
        serde_json::Serializer::with_formatter(&mut file, serde_json::ser::PrettyFormatter::with_indent(b"    "));
    to_document(settings)?.serialize(&mut serializer)?;
    file.flush()?;
    file.sync_all()?;
    std::fs::rename(&temporary, &path)?;
    File::open(folder)?.sync_all()?;
    Ok(())
}

impl<Arguments> Service<Arguments> {
    /// Loads the settings, publishes them on `blueos/v1/<service>/settings`, and serves the `UpdateSettings`
    /// Command. The receiver sees every update that was saved.
    pub async fn settings<S: Settings>(&self) -> Result<watch::Receiver<S>, Error> {
        let folder = self.settings_folder.clone();
        let settings = load::<S>(&folder)?;
        let state = State::<SettingsEnvelope>::declare(&self.session, keys::settings(self.name)).await?;
        state.put_if_changed(&envelope(&settings)?).await?;
        let (sender, receiver) = watch::channel(settings);

        let schema = action_schema::<UpdateSettingsGoal, UpdateSettingsResult, UpdateSettingsFeedback>();
        self.add_endpoint("job", UPDATE_SETTINGS, "blueos_msgs/action/UpdateSettings", schema);
        let key = keys::command(self.name, UPDATE_SETTINGS);
        let queryable = self.session.declare_queryable(&key).await?;
        let shutdown = self.shutdown.clone();
        self.tasks.spawn(async move {
            loop {
                let query = tokio::select! {
                    query = queryable.recv_async() => match query {
                        Ok(query) => query,
                        Err(_) => break,
                    },
                    () = shutdown.cancelled() => break,
                };
                let ack = update(&query, &folder, &sender, &state).await;
                routes::reply(&query, &key, &ack).await;
            }
        });
        Ok(receiver)
    }
}

/// Applies an `UpdateSettings` Goal. Its document must have this `VERSION`, and it is saved before it is used, so
/// a failed write leaves the old file and the old value in place.
async fn update<S: Settings>(
    query: &Query,
    folder: &Path,
    sender: &watch::Sender<S>,
    state: &State<SettingsEnvelope>,
) -> CommandAck {
    let Some(job_id) = routes::job_id(query) else {
        return jobs::rejected(None, "the attachment must be the Job id, a UUID");
    };
    let parsed = query
        .payload()
        .ok_or_else(|| "missing goal".to_owned())
        .and_then(|payload| cdr::decode::<UpdateSettingsGoal>(&payload.to_bytes()).map_err(|error| error.to_string()))
        .and_then(|goal| match serde_json::from_str(&goal.envelope.document_json) {
            Ok(Value::Object(document)) => Ok(document),
            Ok(_) => Err("the document is not a JSON object".to_owned()),
            Err(error) => Err(error.to_string()),
        })
        .and_then(|document| {
            if document.get("VERSION").and_then(Value::as_u64) != Some(S::VERSION) {
                return Err(format!("the document VERSION must be {}", S::VERSION));
            }
            from_document::<S>(document).map_err(|error| error.to_string())
        })
        .and_then(|settings| {
            save(folder, &settings).map_err(|error| error.to_string())?;
            Ok(settings)
        });
    let settings = match parsed {
        Ok(settings) => settings,
        Err(reason) => return jobs::rejected(Some(&job_id), &reason),
    };
    match envelope(&settings) {
        Ok(published) => {
            if let Err(error) = state.put_if_changed(&published).await {
                tracing::warn!(%error, "Failed to publish the settings");
            }
        }
        Err(error) => tracing::warn!(%error, "Failed to encode the settings"),
    }
    sender.send_replace(settings);
    CommandAck {
        accepted: true,
        job_id: job_id.as_str().to_owned(),
        status: CommandAck::STATUS_SUCCEEDED,
        reason: String::new(),
    }
}

fn envelope<S: Settings>(settings: &S) -> Result<SettingsEnvelope, SettingsError> {
    Ok(SettingsEnvelope {
        document_json: serde_json::to_string(&to_document(settings)?)?,
        fields: Vec::new(),
    })
}
