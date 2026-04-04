//! Events published by the background worker as it executes agent jobs.
//!
//! These are broadcast via a `tokio::sync::broadcast` channel so that SSE
//! subscribers can observe job progress in real time.

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEvent {
    /// The worker picked up the job and the LLM call is about to start.
    Started { job_id: Uuid },
    /// The job finished successfully (all actions applied).
    Completed { job_id: Uuid },
    /// The job failed — `error` contains a human-readable reason.
    Failed { job_id: Uuid, error: String },
}

impl JobEvent {
    pub fn job_id(&self) -> Uuid {
        match self {
            Self::Started { job_id } => *job_id,
            Self::Completed { job_id } => *job_id,
            Self::Failed { job_id, .. } => *job_id,
        }
    }
}
