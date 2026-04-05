//! JSON action schema that the agent emits and the worker applies.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All actions an agent may emit in a single turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub actions: Vec<AgentAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentAction {
    /// Post a comment on the current ticket (agent "thinks aloud").
    AddComment {
        body: String,
    },
    /// Update fields on the current ticket.
    UpdateTicket {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority: Option<String>,
        /// Reassign the ticket to another team member (use their UUID from the team list).
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee_person_id: Option<Uuid>,
    },
    /// Create a new ticket in the same workspace (or a specified one).
    CreateTicket {
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ticket_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority: Option<String>,
        /// Assign to a specific team member (use their UUID from the team list).
        /// Defaults to yourself if omitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee_person_id: Option<Uuid>,
        /// Optional override workspace; defaults to same workspace as current ticket.
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace_id: Option<Uuid>,
    },
    /// Propose hiring a new person — produces a pending `HiringProposal`.
    ProposeHire {
        employee_display_name: String,
        role_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        specialty: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rationale: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope_of_work: Option<String>,
    },
    /// Escalate a structured decision to the founder — blocks the ticket until answered.
    RequestDecision {
        /// The specific question being asked.
        question: String,
        /// Optional background context to help the founder decide.
        #[serde(skip_serializing_if = "Option::is_none")]
        context_note: Option<String>,
    },
}

/// Parse the raw LLM response string into an `AgentResponse`.
/// Attempts to extract JSON even if the model wrapped it in prose/markdown.
pub fn parse_response(raw: &str) -> Result<AgentResponse, String> {
    // Try direct parse first.
    if let Ok(r) = serde_json::from_str::<AgentResponse>(raw) {
        return Ok(r);
    }

    // Try to extract the first JSON object from the text (handles markdown fences).
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if end >= start {
                let slice = &raw[start..=end];
                if let Ok(r) = serde_json::from_str::<AgentResponse>(slice) {
                    return Ok(r);
                }
            }
        }
    }

    Err(format!(
        "could not parse agent response as AgentResponse JSON: {}",
        &raw[..raw.len().min(200)]
    ))
}
