use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonKind {
    HumanFounder,
    AiAgent,
}

impl std::fmt::Display for PersonKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HumanFounder => write!(f, "human_founder"),
            Self::AiAgent => write!(f, "ai_agent"),
        }
    }
}

impl std::str::FromStr for PersonKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human_founder" => Ok(Self::HumanFounder),
            "ai_agent" => Ok(Self::AiAgent),
            other => Err(format!("unknown person kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleType {
    CoFounder,
    Ceo,
    Cto,
    Specialist,
}

impl std::fmt::Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoFounder => write!(f, "co_founder"),
            Self::Ceo => write!(f, "ceo"),
            Self::Cto => write!(f, "cto"),
            Self::Specialist => write!(f, "specialist"),
        }
    }
}

impl std::str::FromStr for RoleType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "co_founder" => Ok(Self::CoFounder),
            "ceo" => Ok(Self::Ceo),
            "cto" => Ok(Self::Cto),
            "specialist" => Ok(Self::Specialist),
            other => Err(format!("unknown role type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: Uuid,
    pub company_id: Uuid,
    pub kind: PersonKind,
    pub display_name: String,
    pub role_type: RoleType,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePersonInput {
    pub kind: PersonKind,
    pub display_name: String,
    pub role_type: RoleType,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdatePersonInput {
    pub display_name: Option<String>,
    pub role_type: Option<RoleType>,
    /// None = don't change; Some(None) = clear; Some(Some(v)) = set
    pub specialty: Option<Option<String>>,
    /// None = don't change; Some(None) = clear; Some(Some(id)) = set
    pub ai_profile_id: Option<Option<Uuid>>,
}
