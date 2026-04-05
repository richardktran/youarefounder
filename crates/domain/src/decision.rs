use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    PendingFounder,
    Answered,
}

impl std::fmt::Display for DecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingFounder => write!(f, "pending_founder"),
            Self::Answered => write!(f, "answered"),
        }
    }
}

impl std::str::FromStr for DecisionStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending_founder" => Ok(Self::PendingFounder),
            "answered" => Ok(Self::Answered),
            other => Err(format!("unknown decision status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub id: Uuid,
    pub company_id: Uuid,
    pub ticket_id: Uuid,
    pub raised_by_person_id: Option<Uuid>,
    /// The specific question being escalated to the founder.
    pub question: String,
    /// Optional background context to help the founder decide.
    pub context_note: Option<String>,
    pub status: DecisionStatus,
    /// Populated once the founder answers.
    pub founder_answer: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDecisionRequestInput {
    pub ticket_id: Uuid,
    pub raised_by_person_id: Option<Uuid>,
    pub question: String,
    pub context_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerDecisionRequestInput {
    pub founder_answer: String,
}
