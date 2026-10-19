//! Logs go to the console and, as foxglove `Log` messages, to `blueos/v1/<service>/log`, where other services and
//! recordings read them. Panics are logged too.

use std::{
    fmt::{Debug, Write as _},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use blueos_idl::{builtin_interfaces::Time, cdr, foxglove_msgs::Log};
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{EnvFilter, Layer, layer::Context, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{keys, lock};

/// Records kept until the session opens, and while a burst is published.
const BUFFER: usize = 1024;

static RECORDS: Mutex<Option<mpsc::Receiver<Log>>> = Mutex::new(None);

struct Backbone {
    records: mpsc::Sender<Log>,
}

#[derive(Default)]
struct Text {
    message: String,
    fields: String,
}

/// Starts logging to the console at once; the records for the `log` key wait until a session publishes them.
pub fn init(verbose: u8) {
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let (records, receiver) = mpsc::channel(BUFFER);
    let installed = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(Backbone { records })
        .try_init();
    if installed.is_ok() {
        *lock(&RECORDS) = Some(receiver);
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            tracing::error!(%info, "Panic");
            default_hook(info);
        }));
    }
}

/// Publishes the records until shutdown, then the ones still queued.
pub(crate) fn publish(
    session: &zenoh::Session,
    service: &'static str,
    tasks: &TaskTracker,
    shutdown: CancellationToken,
) {
    let Some(mut records) = lock(&RECORDS).take() else {
        return;
    };
    let session = session.clone();
    tasks.spawn(async move {
        let key = keys::log(service);
        loop {
            let record = tokio::select! {
                record = records.recv() => record,
                () = shutdown.cancelled() => None,
            };
            let Some(record) = record else {
                break;
            };
            put(&session, &key, service, record).await;
        }
        while let Ok(record) = records.try_recv() {
            put(&session, &key, service, record).await;
        }
    });
}

async fn put(session: &zenoh::Session, key: &str, service: &str, mut record: Log) {
    record.name = service.to_owned();
    if let Ok(bytes) = cdr::encode(&record) {
        // A failure here cannot be logged: that record would come back to this same put.
        let _ = session.put(key, bytes).encoding(cdr::encoding::<Log>()).await;
    }
}

impl<S: Subscriber> Layer<S> for Backbone {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        // Publishing a record logs inside zenoh, publishing those would never end.
        if metadata.target().starts_with("zenoh") {
            return;
        }
        let mut text = Text::default();
        event.record(&mut text);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let record = Log {
            timestamp: Time {
                sec: i32::try_from(now.as_secs()).unwrap_or(i32::MAX),
                nanosec: now.subsec_nanos(),
            },
            level: match *metadata.level() {
                Level::TRACE | Level::DEBUG => Log::DEBUG,
                Level::INFO => Log::INFO,
                Level::WARN => Log::WARNING,
                Level::ERROR => Log::ERROR,
            },
            message: text.message + &text.fields,
            name: String::new(),
            file: metadata.file().unwrap_or_default().to_owned(),
            line: metadata.line().unwrap_or_default(),
        };
        // ponytail: drops records when the buffer is full, a bigger buffer if bursts lose lines that matter.
        let _ = self.records.try_send(record);
    }
}

impl Visit for Text {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={value}", field.name());
        }
    }
}
