//! The Jobs of a service: what clients asked for, its status, feedback and result, like a ROS 2 action. Lives in
//! the service loop, so it needs no lock.

use std::collections::VecDeque;

use blueos_idl::{
    blueos_msgs::{CommandAck, JobFeedback, JobFeedbackList, JobList, JobResult, JobStatus},
    cdr::{self, CdrError},
};
use serde::Serialize;

/// Finished Jobs kept to answer retries and the history Query.
pub const HISTORY_LENGTH: usize = 32;

/// The UUID a client generated for its Job.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobId(String);

impl JobId {
    pub fn parse(text: &str) -> Option<Self> {
        uuid::Uuid::parse_str(text).ok().map(|_| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    status: JobStatus,
    goal: Vec<u8>,
    cancellable: bool,
    feedback: Vec<u8>,
}

/// What `submit` did with a Job.
#[derive(Debug, PartialEq)]
pub enum Submitted {
    /// A new Job, now Accepted: the service starts it.
    New,
    /// The same id with the same goal again: nothing to start, answer with this ack.
    Retry(CommandAck),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Jobs {
    active: Vec<Entry>,
    finished: VecDeque<Entry>,
    results: Vec<JobResult>,
}

impl Jobs {
    /// Accepts a new Job. The same id with the same goal is a retry; with another goal it is rejected.
    pub fn submit(
        &mut self,
        id: &JobId,
        job_type: &str,
        goal: &[u8],
        cancellable: bool,
    ) -> Result<Submitted, CommandAck> {
        if let Some(entry) = self.find(id) {
            if entry.status.job_type == job_type && entry.goal == goal {
                return Ok(Submitted::Retry(ack(&entry.status)));
            }
            return Err(rejected(Some(id), "id reused"));
        }
        self.active.push(Entry {
            status: JobStatus {
                job_id: id.as_str().to_owned(),
                job_type: job_type.to_owned(),
                status: JobStatus::STATUS_ACCEPTED,
                reason: String::new(),
            },
            goal: goal.to_vec(),
            cancellable,
            feedback: Vec::new(),
        });
        Ok(Submitted::New)
    }

    /// Takes back a Job just submitted that the service refuses: it was never accepted, so it leaves no trace.
    pub fn reject(&mut self, id: &JobId, reason: &str) -> CommandAck {
        self.active.retain(|entry| entry.status.job_id != id.as_str());
        rejected(Some(id), reason)
    }

    pub fn start(&mut self, id: &JobId) {
        if let Some(entry) = self.active_mut(id) {
            entry.status.status = JobStatus::STATUS_EXECUTING;
        }
    }

    pub fn feedback<Feedback: Serialize>(&mut self, id: &JobId, feedback: &Feedback) -> Result<(), CdrError> {
        if let Some(entry) = self.active_mut(id) {
            entry.feedback = cdr::encode(feedback)?;
        }
        Ok(())
    }

    /// Ends an active Job as Succeeded, Canceled or Aborted, with its Job result.
    pub fn finish<Output: Serialize>(
        &mut self,
        id: &JobId,
        status: u8,
        reason: &str,
        result: &Output,
    ) -> Result<(), CdrError> {
        let Some(index) = self.active.iter().position(|entry| entry.status.job_id == id.as_str()) else {
            return Ok(());
        };
        let result = cdr::encode(result)?;
        let mut entry = self.active.remove(index);
        entry.status.status = status;
        entry.status.reason = reason.to_owned();
        self.results.push(JobResult {
            job: entry.status.clone(),
            result,
        });
        if self.finished.len() == HISTORY_LENGTH {
            self.finished.pop_front();
        }
        self.finished.push_back(entry);
        Ok(())
    }

    /// Moves a cancellable active Job to Canceling; the service then finishes it as Canceled.
    pub fn cancel(&mut self, id: &JobId) -> CommandAck {
        if self.finished.iter().any(|entry| entry.status.job_id == id.as_str()) {
            return rejected(Some(id), "the job already ended");
        }
        match self.active_mut(id) {
            None => rejected(Some(id), "unknown job"),
            Some(entry) if !entry.cancellable => rejected(Some(id), "the job cannot be cancelled"),
            Some(entry) => {
                entry.status.status = JobStatus::STATUS_CANCELING;
                ack(&entry.status)
            }
        }
    }

    pub fn is_canceling(&self, id: &JobId) -> bool {
        self.find(id)
            .is_some_and(|entry| entry.status.status == JobStatus::STATUS_CANCELING)
    }

    /// The ack of a known Job, with its status now, so a Job that ended in the same step returns its final status.
    pub fn ack(&self, id: &JobId) -> CommandAck {
        self.find(id)
            .map_or_else(|| rejected(Some(id), "unknown job"), |entry| ack(&entry.status))
    }

    /// The `jobs` State: every active Job.
    pub fn list(&self) -> JobList {
        JobList {
            jobs: self.active.iter().map(|entry| entry.status.clone()).collect(),
        }
    }

    pub fn history(&self, job_type: &str) -> JobList {
        JobList {
            jobs: self
                .finished
                .iter()
                .filter(|entry| entry.status.job_type == job_type)
                .map(|entry| entry.status.clone())
                .collect(),
        }
    }

    pub fn feedback_list(&self, job_type: &str) -> JobFeedbackList {
        JobFeedbackList {
            jobs: self
                .active
                .iter()
                .filter(|entry| entry.status.job_type == job_type && !entry.feedback.is_empty())
                .map(|entry| JobFeedback {
                    job_id: entry.status.job_id.clone(),
                    feedback: entry.feedback.clone(),
                })
                .collect(),
        }
    }

    /// The results of the Jobs that ended since the last call, to publish as Events.
    pub fn take_results(&mut self) -> Vec<JobResult> {
        std::mem::take(&mut self.results)
    }

    fn find(&self, id: &JobId) -> Option<&Entry> {
        self.active
            .iter()
            .chain(self.finished.iter())
            .find(|entry| entry.status.job_id == id.as_str())
    }

    fn active_mut(&mut self, id: &JobId) -> Option<&mut Entry> {
        self.active.iter_mut().find(|entry| entry.status.job_id == id.as_str())
    }
}

pub fn rejected(id: Option<&JobId>, reason: &str) -> CommandAck {
    CommandAck {
        accepted: false,
        job_id: id.map(JobId::as_str).unwrap_or_default().to_owned(),
        status: CommandAck::STATUS_UNKNOWN,
        reason: reason.to_owned(),
    }
}

fn ack(status: &JobStatus) -> CommandAck {
    CommandAck {
        accepted: true,
        job_id: status.job_id.clone(),
        status: status.status,
        reason: status.reason.clone(),
    }
}
