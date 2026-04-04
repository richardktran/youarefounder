use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A single recorded agent execution against a ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: Uuid,
    pub agent_job_id: Uuid,
    pub ticket_id: Uuid,
    pub person_id: Uuid,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub raw_response: Option<String>,
    pub actions_applied: Value,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Payload stored in `agent_jobs.payload` for `AgentTicketRun` jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTicketRunPayload {
    pub ticket_id: Uuid,
    pub person_id: Uuid,
}
