use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    PendingFounder,
    Accepted,
    Declined,
    Withdrawn,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingFounder => write!(f, "pending_founder"),
            Self::Accepted => write!(f, "accepted"),
            Self::Declined => write!(f, "declined"),
            Self::Withdrawn => write!(f, "withdrawn"),
        }
    }
}

impl std::str::FromStr for ProposalStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending_founder" => Ok(Self::PendingFounder),
            "accepted" => Ok(Self::Accepted),
            "declined" => Ok(Self::Declined),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(format!("unknown proposal status: {other}")),
        }
    }
}

/// A hiring proposal (and its embedded contract terms) submitted for founder review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiringProposal {
    pub id: Uuid,
    pub company_id: Uuid,
    /// Person who submitted the proposal (founder or AI agent); `None` for system-created.
    pub proposed_by_person_id: Option<Uuid>,
    /// Name to give the new hire.
    pub employee_display_name: String,
    pub role_type: String,
    pub specialty: Option<String>,
    /// AI profile the new hire will use.
    pub ai_profile_id: Option<Uuid>,
    pub rationale: Option<String>,
    pub scope_of_work: Option<String>,
    pub status: ProposalStatus,
    pub founder_response_text: Option<String>,
    /// Set after accept — points at the newly created `Person`.
    pub created_person_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a new hiring proposal (founder-initiated for Phase 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalInput {
    pub proposed_by_person_id: Option<Uuid>,
    pub employee_display_name: String,
    pub role_type: String,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
    pub rationale: Option<String>,
    pub scope_of_work: Option<String>,
}

/// Input for accepting a proposal (creates a new Person).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcceptProposalInput {
    /// Optional founder note saved alongside the acceptance.
    pub founder_response_text: Option<String>,
}

/// Input for declining a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclineProposalInput {
    /// Reason is required on decline so agents can learn from it.
    pub founder_response_text: String,
}
