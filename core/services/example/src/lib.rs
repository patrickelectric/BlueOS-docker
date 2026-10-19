//! The example service: a pump that moves to a level one step at a time, as a cancellable SetLevel Job. It is the
//! smallest complete use of blueos-service, and it is never shipped.

pub mod pump;
pub mod routes;

use blueos_idl::{
    blueos_example_msgs::{LevelRequest, LevelResponse, PumpState, SetLevelFeedback, SetLevelResult},
    blueos_msgs::{CommandAck, JobStatus},
};
use blueos_service::{Action, Command, Error, Jobs, Service, Submitted, Timers, settings::Settings};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::{
    pump::{Progress, Pump, STEP},
    routes::ROUTES,
};

pub const NAME: &str = "example";

#[derive(Clone, Debug, clap::Args)]
pub struct Arguments {}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExampleSettings {
    pub max_level: u8,
}

pub enum Request {
    SetLevel(u8),
}

impl Default for ExampleSettings {
    fn default() -> Self {
        Self { max_level: 100 }
    }
}

impl Settings for ExampleSettings {
    const VERSION: u64 = 1;
}

pub async fn serve(service: Service<Arguments>) -> Result<(), Error> {
    let mut commands = service.routes(ROUTES).await?;
    let job_keys = service.job_keys(ROUTES).await?;
    let state = service.state::<PumpState>("pump").await?;
    let settings = service.settings::<ExampleSettings>().await?;
    let mut pump = Pump::new(settings.borrow().max_level);
    let (published, level) = watch::channel(pump_state(&pump));
    service
        .io_query("Level", move |_: LevelRequest| {
            let PumpState { level, max_level } = level.borrow().clone();
            async move { Ok(LevelResponse { level, max_level }) }
        })
        .await?;
    let _ready = service.ready().await?;

    let mut jobs = Jobs::default();
    let mut timers = Timers::default();
    loop {
        tokio::select! {
            Some(command) = commands.recv() => handle(&mut pump, &mut jobs, &mut timers, command)?,
            () = timers.next() => {
                if let Some((job, target)) = pump.moving.clone() {
                    match pump.step() {
                        Some(done) => jobs.finish(&done, JobStatus::STATUS_SUCCEEDED, "", &SetLevelResult { level: target })?,
                        None => {
                            jobs.feedback(&job, &SetLevelFeedback { level: pump.level })?;
                            timers.schedule((), STEP);
                        }
                    }
                }
            }
            () = service.shutdown.cancelled() => return Ok(()),
        }
        published.send_replace(pump_state(&pump));
        state.put_if_changed(&pump_state(&pump)).await?;
        job_keys.sync(&mut jobs).await?;
    }
}

fn handle(pump: &mut Pump, jobs: &mut Jobs, timers: &mut Timers<()>, command: Command<Request>) -> Result<(), Error> {
    let id = command.job_id.clone();
    let ack: CommandAck = match command.action {
        Action::Submit(Request::SetLevel(level)) => {
            match jobs.submit(&id, command.job_type, &command.goal, command.cancellable) {
                Ok(Submitted::New) => match pump.set_level(id.clone(), level) {
                    Err(reason) => jobs.reject(&id, reason),
                    Ok(Progress::Reached) => {
                        jobs.finish(&id, JobStatus::STATUS_SUCCEEDED, "", &SetLevelResult { level })?;
                        jobs.ack(&id)
                    }
                    Ok(Progress::Moving) => {
                        jobs.start(&id);
                        timers.schedule((), STEP);
                        jobs.ack(&id)
                    }
                },
                Ok(Submitted::Retry(ack)) | Err(ack) => ack,
            }
        }
        Action::Cancel => {
            let ack = jobs.cancel(&id);
            if !ack.accepted {
                ack
            } else {
                pump.stop();
                timers.cancel(&());
                let level = pump.level;
                jobs.finish(&id, JobStatus::STATUS_CANCELED, "cancelled", &SetLevelResult { level })?;
                jobs.ack(&id)
            }
        }
    };
    command.reply.send(ack);
    Ok(())
}

fn pump_state(pump: &Pump) -> PumpState {
    PumpState {
        level: pump.level,
        max_level: pump.max_level,
    }
}
