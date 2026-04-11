use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Controls whether the business simulation is actively running.
/// - `Stopped` — agents do no work; jobs stay queued but are not claimed.
/// - `Running` — worker claims and executes agent jobs.
/// - `Terminated` — company and all associated data has been deleted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Stopped,
    Running,
    Terminated,
}

impl Default for RunState {
    fn default() -> Self {
        Self::Stopped
    }
}

impl std::fmt::Display for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Running => write!(f, "running"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

impl std::str::FromStr for RunState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stopped" => Ok(Self::Stopped),
            "running" => Ok(Self::Running),
            "terminated" => Ok(Self::Terminated),
            other => Err(format!("unknown run state: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub onboarding_complete: bool,
    /// Phase 4: simulation control state.
    pub run_state: RunState,
    /// Maximum number of agent jobs that can run concurrently for this company.
    pub max_concurrent_agents: i32,
    /// Founder instructions for how agents should handle tickets (delegation, tone, cadence, etc.).
    pub agent_ticket_memory: Option<String>,
    /// Founder instructions for escalations and decisions (when to ask, how to format answers, etc.).
    pub agent_decision_memory: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompanyInput {
    pub name: String,
    /// Initial product to create atomically with the company.
    pub product: Option<CreateProductInline>,
}

/// Product fields inlined into company creation for the onboarding transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductInline {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCompanyInput {
    pub name: Option<String>,
    pub onboarding_complete: Option<bool>,
    pub run_state: Option<RunState>,
    pub max_concurrent_agents: Option<i32>,
    pub agent_ticket_memory: Option<String>,
    pub agent_decision_memory: Option<String>,
}

/// Response returned by `GET /v1/bootstrap` so the UI knows
/// whether to show onboarding or the main app shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapStatus {
    pub onboarding_complete: bool,
    pub company_id: Option<Uuid>,
}

/// Exact phrase required in `POST /v1/system/reset-install` to wipe every company
/// (and cascaded rows) on this install. Keep in sync with the web client.
pub const RESET_INSTALL_CONFIRM_PHRASE: &str = "DELETE ALL LOCAL DATA";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetInstallInput {
    pub confirm_phrase: String,
}
