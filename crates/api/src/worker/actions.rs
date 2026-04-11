//! JSON action schema that the agent emits and the worker applies.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
        /// Bullet list or short paragraph: what must be true to set status to done.
        #[serde(skip_serializing_if = "Option::is_none")]
        definition_of_done: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority: Option<String>,
        /// Reassign the ticket to another team member (use their UUID from the team list).
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee_person_id: Option<Uuid>,
    },
    /// Create a new top-level ticket in the same workspace (or a specified one). Prefer `create_subtask` when breaking down the *current* ticket.
    CreateTicket {
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        definition_of_done: Option<String>,
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
    /// Create a subtask linked to the current ticket only (`parent` = this ticket). Same workspace as the current ticket.
    CreateSubtask {
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        definition_of_done: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee_person_id: Option<Uuid>,
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
    /// Link another ticket (by id) so its outcome appears in context on future runs of this ticket.
    AddTicketReference {
        to_ticket_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// Remove a previously added ticket reference.
    RemoveTicketReference {
        to_ticket_id: Uuid,
    },
    /// Queue a proposed insight for the founder to approve into the product brain (does not bypass review).
    ProposeBrainInsight {
        summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Remove ```json … ``` wrappers so we parse real JSON only.
fn strip_markdown_fence(raw: &str) -> &str {
    let s = raw.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```JSON"))
        .unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s).trim();
    s.strip_suffix("```").unwrap_or(s).trim()
}

/// Some models (e.g. smaller Ollama chat models) emit shorthand objects like
/// `{ "add_comment": "hello" }` instead of `{ "type": "add_comment", "body": "hello" }`.
/// Normalize into the tagged shape serde expects.
fn normalize_agent_response_value(root: &mut Value) {
    let Some(actions) = root.get_mut("actions").and_then(|a| a.as_array_mut()) else {
        return;
    };
    let mut normalized: Vec<Value> = Vec::with_capacity(actions.len());
    for item in actions.drain(..) {
        normalized.push(normalize_one_action(item));
    }
    *actions = normalized;
}

fn normalize_one_action(v: Value) -> Value {
    match v {
        Value::String(body) => json!({ "type": "add_comment", "body": body }),
        Value::Object(map) => {
            if map.contains_key("type") {
                return Value::Object(map);
            }
            if map.len() != 1 {
                return Value::Object(map);
            }
            let (key, val) = map.into_iter().next().expect("len checked");
            match key.as_str() {
                "add_comment" => match val {
                    Value::String(body) => json!({ "type": "add_comment", "body": body }),
                    other => json!({ "type": "add_comment", "body": other.to_string() }),
                },
                "update_ticket" => merge_type_object("update_ticket", val),
                "create_ticket" => merge_type_object("create_ticket", val),
                "create_subtask" => merge_type_object("create_subtask", val),
                "propose_hire" => merge_type_object("propose_hire", val),
                "request_decision" => merge_type_object("request_decision", val),
                "add_ticket_reference" => merge_type_object("add_ticket_reference", val),
                "remove_ticket_reference" => merge_type_object("remove_ticket_reference", val),
                "propose_brain_insight" => merge_type_object("propose_brain_insight", val),
                _ => {
                    // Unknown single key — best-effort: treat string as add_comment body key typo
                    if let Value::String(s) = val {
                        json!({ "type": "add_comment", "body": format!("[{key}] {s}") })
                    } else {
                        json!({ "type": "add_comment", "body": format!("{key}: {val}") })
                    }
                }
            }
        }
        other => json!({ "type": "add_comment", "body": other.to_string() }),
    }
}

fn merge_type_object(ty: &'static str, val: Value) -> Value {
    match val {
        Value::Object(mut inner) => {
            inner.insert("type".to_string(), Value::String(ty.to_string()));
            Value::Object(inner)
        }
        Value::String(s) if ty == "add_comment" => json!({ "type": "add_comment", "body": s }),
        other => json!({ "type": ty, "body": other.to_string() }),
    }
}

/// Parse the raw LLM response string into an `AgentResponse`.
/// Attempts to extract JSON even if the model wrapped it in prose/markdown.
pub fn parse_response(raw: &str) -> Result<AgentResponse, String> {
    let trimmed = strip_markdown_fence(raw);

    if let Ok(r) = serde_json::from_str::<AgentResponse>(trimmed) {
        return Ok(r);
    }

    // Try to extract the first JSON object from the text (handles leading prose).
    let slice = extract_json_object(trimmed).ok_or_else(|| {
        format!(
            "no JSON object found in agent response: {}",
            &trimmed[..trimmed.len().min(120)]
        )
    })?;

    if let Ok(r) = serde_json::from_str::<AgentResponse>(slice) {
        return Ok(r);
    }

    let mut v: Value =
        serde_json::from_str(slice).map_err(|e| format!("invalid JSON object: {e}"))?;
    normalize_agent_response_value(&mut v);

    serde_json::from_value(v).map_err(|e| {
        format!(
            "could not map normalized JSON to AgentResponse: {e}; excerpt: {}",
            &slice[..slice.len().min(280)]
        )
    })
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end >= start).then_some(&s[start..=end])
}
