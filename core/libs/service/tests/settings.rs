//! Settings files written by the Python settings manager (`fixtures/generate.py`) load, migrate and save back to
//! the same bytes.
#![allow(clippy::unwrap_used, reason = "test helpers")]

use std::path::Path;

use blueos_service::settings::{self, Settings, SettingsError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Animal {
    name: String,
    animal_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct GoldenSettingsV2 {
    first_variable: i64,
    new_animal: Animal,
}

impl Default for GoldenSettingsV2 {
    fn default() -> Self {
        Self {
            first_variable: 66,
            new_animal: Animal {
                name: "bilica".into(),
                animal_type: "dog".into(),
            },
        }
    }
}

impl Settings for GoldenSettingsV2 {
    const VERSION: u64 = 2;

    fn migrate(document: &mut Map<String, Value>) -> Result<(), SettingsError> {
        if let Some(animal) = document.remove("animal") {
            document.insert("new_animal".into(), animal);
        }
        document.insert("VERSION".into(), 2.into());
        Ok(())
    }
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn saved(folder: &Path) -> String {
    std::fs::read_to_string(settings::path::<GoldenSettingsV2>(folder)).unwrap()
}

#[test]
fn a_python_v1_file_migrates_and_saves_like_python() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::copy(
        fixture("golden_settings_v1.json"),
        folder.path().join("settings-1.json"),
    )
    .unwrap();
    let loaded = settings::load::<GoldenSettingsV2>(folder.path()).unwrap();
    assert_eq!(loaded.first_variable, 100);
    settings::save(folder.path(), &loaded).unwrap();
    assert_eq!(
        saved(folder.path()),
        std::fs::read_to_string(fixture("golden_settings_v2_migrated_save.json")).unwrap()
    );
}

#[test]
fn the_defaults_save_like_python() {
    let folder = tempfile::tempdir().unwrap();
    let loaded = settings::load::<GoldenSettingsV2>(folder.path()).unwrap();
    assert_eq!(loaded, GoldenSettingsV2::default());
    assert_eq!(
        saved(folder.path()),
        std::fs::read_to_string(fixture("golden_settings_v2_direct_save.json")).unwrap()
    );
}

#[test]
fn a_file_from_the_future_is_refused_and_an_older_one_is_used() {
    assert!(matches!(
        settings::load_file::<GoldenSettingsV2>(&fixture("golden_settings_future_v9.json")),
        Err(SettingsError::FromTheFuture(9))
    ));
    let folder = tempfile::tempdir().unwrap();
    std::fs::copy(
        fixture("golden_settings_future_v9.json"),
        folder.path().join("settings-9.json"),
    )
    .unwrap();
    std::fs::copy(
        fixture("golden_settings_v1.json"),
        folder.path().join("settings-1.json"),
    )
    .unwrap();
    assert_eq!(
        settings::load::<GoldenSettingsV2>(folder.path())
            .unwrap()
            .first_variable,
        100
    );
}

#[test]
fn leftover_temporary_files_are_removed() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("settings-2.tmp"), "{").unwrap();
    settings::load::<GoldenSettingsV2>(folder.path()).unwrap();
    assert!(!folder.path().join("settings-2.tmp").exists());
}

#[test]
fn a_failed_write_keeps_the_old_file() {
    let folder = tempfile::tempdir().unwrap();
    settings::load::<GoldenSettingsV2>(folder.path()).unwrap();
    let before = saved(folder.path());
    // A folder where the temporary file goes makes the write fail, as a full disk would.
    std::fs::create_dir(folder.path().join("settings-2.tmp")).unwrap();
    let changed = GoldenSettingsV2 {
        first_variable: 1,
        ..GoldenSettingsV2::default()
    };
    assert!(settings::save(folder.path(), &changed).is_err());
    assert_eq!(saved(folder.path()), before);
}
