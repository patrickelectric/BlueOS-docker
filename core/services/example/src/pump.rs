//! The pump: it moves its level one step at a time towards the level a SetLevel Job asked for.

use std::time::Duration;

use blueos_service::JobId;

pub const STEP: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq)]
pub struct Pump {
    pub level: u8,
    pub max_level: u8,
    /// The Job the pump moves for, and the level it moves to.
    pub moving: Option<(JobId, u8)>,
}

#[derive(Debug, PartialEq)]
pub enum Progress {
    Moving,
    Reached,
}

impl Pump {
    pub fn new(max_level: u8) -> Self {
        Self {
            level: 0,
            max_level,
            moving: None,
        }
    }

    /// Starts moving to `target`, or refuses it without changing anything.
    pub fn set_level(&mut self, job: JobId, target: u8) -> Result<Progress, &'static str> {
        if target > self.max_level {
            return Err("the level is above the maximum");
        }
        if self.moving.is_some() {
            return Err("the pump is moving for another job");
        }
        if target == self.level {
            return Ok(Progress::Reached);
        }
        self.moving = Some((job, target));
        Ok(Progress::Moving)
    }

    /// Moves one step. Returns the Job whose level is reached.
    pub fn step(&mut self) -> Option<JobId> {
        let (job, target) = self.moving.clone()?;
        if self.level < target {
            self.level += 1;
        } else {
            self.level -= 1;
        }
        (self.level == target).then(|| {
            self.moving = None;
            job
        })
    }

    pub fn stop(&mut self) {
        self.moving = None;
    }
}

#[cfg(test)]
mod tests {
    use blueos_service::testing::assert_rejection_leaves_state;

    use super::*;

    fn job() -> JobId {
        JobId::parse("0b5e8f5c-6f0a-4c4e-9a52-2f1e7d3c9b10").unwrap()
    }

    #[test]
    fn the_pump_reaches_its_level_one_step_at_a_time() {
        let mut pump = Pump::new(10);
        assert_eq!(pump.set_level(job(), 2), Ok(Progress::Moving));
        assert_eq!(pump.step(), None);
        assert_eq!(pump.step(), Some(job()));
        assert_eq!((pump.level, pump.moving.is_none()), (2, true));
        assert_eq!(pump.set_level(job(), 2), Ok(Progress::Reached));
    }

    #[test]
    fn a_refused_level_changes_nothing() {
        let mut pump = Pump::new(10);
        assert_rejection_leaves_state(&mut pump, |pump| pump.set_level(job(), 11));
        pump.set_level(job(), 5).unwrap();
        assert_rejection_leaves_state(&mut pump, |pump| pump.set_level(job(), 1));
    }
}
