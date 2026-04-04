use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Ticket status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    Backlog,
    Todo,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

impl Default for TicketStatus {
    fn default() -> Self {
        Self::Backlog
    }
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backlog => write!(f, "backlog"),
            Self::Todo => write!(f, "todo"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Done => write!(f, "done"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for TicketStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Self::Backlog),
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(format!("unknown ticket status: {other}")),
        }
    }
}

// ─── Ticket type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TicketType {
    Task,
    Epic,
    Research,
}

impl Default for TicketType {
    fn default() -> Self {
        Self::Task
    }
}

impl std::fmt::Display for TicketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Task => write!(f, "task"),
            Self::Epic => write!(f, "epic"),
            Self::Research => write!(f, "research"),
        }
    }
}

impl std::str::FromStr for TicketType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task" => Ok(Self::Task),
            "epic" => Ok(Self::Epic),
            "research" => Ok(Self::Research),
            other => Err(format!("unknown ticket type: {other}")),
        }
    }
}

// ─── Ticket priority ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
}

impl Default for TicketPriority {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for TicketPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

impl std::str::FromStr for TicketPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(format!("unknown ticket priority: {other}")),
        }
    }
}

// ─── Ticket ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub ticket_type: TicketType,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub assignee_person_id: Option<Uuid>,
    pub parent_ticket_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicketInput {
    pub title: String,
    pub description: Option<String>,
    pub ticket_type: Option<TicketType>,
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub assignee_person_id: Option<Uuid>,
    pub parent_ticket_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTicketInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub ticket_type: Option<TicketType>,
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub assignee_person_id: Option<Uuid>,
    pub parent_ticket_id: Option<Uuid>,
}

// ─── Ticket comment ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketComment {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub body: String,
    pub author_person_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentInput {
    pub body: String,
    pub author_person_id: Option<Uuid>,
}
