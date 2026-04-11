use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductBrainPendingStatus {
    Pending,
    Rejected,
    Promoted,
}

impl std::fmt::Display for ProductBrainPendingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Rejected => write!(f, "rejected"),
            Self::Promoted => write!(f, "promoted"),
        }
    }
}

impl std::str::FromStr for ProductBrainPendingStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "rejected" => Ok(Self::Rejected),
            "promoted" => Ok(Self::Promoted),
            other => Err(format!("unknown product brain pending status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBrainEntry {
    pub id: Uuid,
    pub company_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub body: String,
    pub source_ticket_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBrainPending {
    pub id: Uuid,
    pub company_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub body: String,
    pub source_ticket_id: Option<Uuid>,
    pub status: ProductBrainPendingStatus,
    pub proposed_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketReference {
    pub from_ticket_id: Uuid,
    pub to_ticket_id: Uuid,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicketReferenceInput {
    pub to_ticket_id: Uuid,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovePendingBrainInput {
    /// Optional edit of body before promoting to approved corpus.
    pub body: Option<String>,
}
