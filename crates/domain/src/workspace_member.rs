use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMemberRole {
    Member,
    Lead,
}

impl std::fmt::Display for WorkspaceMemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Member => write!(f, "member"),
            Self::Lead => write!(f, "lead"),
        }
    }
}

impl std::str::FromStr for WorkspaceMemberRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "member" => Ok(Self::Member),
            "lead" => Ok(Self::Lead),
            other => Err(format!("unknown workspace member role: {other}")),
        }
    }
}

impl Default for WorkspaceMemberRole {
    fn default() -> Self {
        Self::Member
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub person_id: Uuid,
    pub role: WorkspaceMemberRole,
    pub created_at: DateTime<Utc>,
    // Denormalised from `people` for convenience
    pub display_name: String,
    pub person_kind: String,
    pub role_type: String,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddWorkspaceMemberInput {
    pub person_id: Uuid,
    pub role: WorkspaceMemberRole,
}
