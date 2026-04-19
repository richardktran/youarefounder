//! JSON action schema that the agent emits and the worker applies.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// All actions an agent may emit in a single turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
        /// Workspaces the new hire should join (omit for `co_founder` — they join all automatically).
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace_ids: Option<Vec<Uuid>>,
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
    /// Propose an insight for the product brain (promoted immediately in autonomous mode).
    ProposeBrainInsight {
        summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Ticket thread text when we must not paste agent JSON (invalid, incomplete, or non-standard).
pub(crate) const COMMENT_BODY_PLACEHOLDER: &str =
    "The agent left an update that could not be shown as plain text here.";

/// Remove ```json … ``` wrappers so we parse real JSON only.
fn strip_markdown_fence(raw: &str) -> &str {
    let s = raw.trim();
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```JSON"))
        .unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s).trim();
    s.strip_suffix("```").unwrap_or(s).trim()
}

/// LLMs often emit trailing commas before `}` or `]`, which strict JSON rejects.
/// Strips them only outside of string literals (respects escapes).
fn strip_trailing_commas_json(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            i += 1;
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
                i += 1;
            }
            ',' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && matches!(chars[j], '}' | ']') {
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Normalize common LLM JSON slip-ups before `serde_json` parsing.
fn sanitize_agent_json(s: &str) -> String {
    let s = strip_trailing_commas_json(s);
    // Curly/smart quotes sometimes appear instead of ASCII `"` around keys.
    s.chars()
        .map(|c| match c {
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{00AB}' | '\u{00BB}' => '"',
            _ => c,
        })
        .collect()
}

fn response_excerpt(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push_str("\n…(truncated)");
    }
    out
}

/// First complete top-level JSON object `{...}` or array `[...]` in `s`.
/// String-aware so braces/brackets inside JSON strings do not change depth.
fn extract_first_json_value(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let first_obj = s.find('{');
    let first_arr = s.find('[');
    let start = match (first_obj, first_arr) {
        (Some(o), Some(a)) => Some(std::cmp::min(o, a)),
        (Some(o), None) => Some(o),
        (None, Some(a)) => Some(a),
        (None, None) => None,
    }?;
    let bytes = s.as_bytes();
    match bytes.get(start).copied()? {
        b'{' => extract_balanced_slice(s, start, b'{', b'}'),
        b'[' => extract_balanced_slice(s, start, b'[', b']'),
        _ => None,
    }
}

/// True when `s` looks like the start of our agent envelope (so truncation repair is safe to try).
fn looks_like_agent_actions_envelope(s: &str) -> bool {
    let head: String = s.trim_start().chars().take(72).collect();
    s.trim_start().starts_with('{') && head.contains("\"actions\"")
}

/// If the model hit `max_tokens` (or provider default) mid-stream, JSON is often cut inside a
/// string — balanced extraction never completes. Close the open string and any `[` `{` frames.
fn try_repair_truncated_actions_json(s: &str) -> Option<String> {
    if !looks_like_agent_actions_envelope(s) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut stack: Vec<u8> = Vec::new();
    let mut i = 0usize;
    let mut in_string = false;
    let mut escape = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'{' => {
                stack.push(b'{');
                i += 1;
            }
            b'[' => {
                stack.push(b'[');
                i += 1;
            }
            b'}' => {
                if stack.last() == Some(&b'{') {
                    stack.pop();
                }
                i += 1;
            }
            b']' => {
                if stack.last() == Some(&b'[') {
                    stack.pop();
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    if stack.is_empty() && !in_string {
        return None;
    }

    let mut out = s.to_string();
    if in_string {
        out.push('"');
    }
    while let Some(op) = stack.pop() {
        match op {
            b'{' => out.push('}'),
            b'[' => out.push(']'),
            _ => {}
        }
    }
    Some(out)
}

fn extract_balanced_slice(s: &str, start: usize, open: u8, close: u8) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_string = true;
                i += 1;
            }
            o if o == open => {
                depth += 1;
                i += 1;
            }
            c if c == close => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(&s[start..i]);
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// Models sometimes return a custom envelope (`action`, `details`) or a single object
/// where `actions` should be an array. Coerce into `{ "actions": [ ... ] }` before
/// per-action normalization.
fn coerce_root_to_actions_envelope(root: &mut Value) {
    let Value::Object(obj) = root else {
        return;
    };

    if let Some(a) = obj.get("actions") {
        match a {
            Value::Array(_) => return,
            Value::Object(_) => {
                let single = obj.remove("actions").expect("just read");
                obj.insert("actions".to_string(), Value::Array(vec![single]));
                return;
            }
            other => {
                let _ = other;
                obj.insert(
                    "actions".to_string(),
                    Value::Array(vec![json!({
                        "type": "add_comment",
                        "body": COMMENT_BODY_PLACEHOLDER
                    })]),
                );
                return;
            }
        }
    }

    if obj.contains_key("action") {
        let action_label = match obj.get("action") {
            Some(Value::String(s)) => s.clone(),
            Some(o) => o.to_string(),
            None => "unknown_action".to_string(),
        };

        let mut parts = vec![format!("**{action_label}**")];

        if let Some(details) = obj.get("details") {
            match details {
                Value::Object(m) => {
                    if let Some(step) = m.get("step").and_then(|v| v.as_str()) {
                        parts.push(format!("**Step:** {step}"));
                    }
                    if let Some(output) = m.get("output").and_then(|v| v.as_str()) {
                        parts.push(format!("**Output:** {output}"));
                    }
                    for (k, v) in m {
                        if k == "step" || k == "output" {
                            continue;
                        }
                        let line = if let Some(s) = v.as_str() {
                            format!("**{k}:** {s}")
                        } else {
                            format!("**{k}:** {}", v)
                        };
                        parts.push(line);
                    }
                }
                _ => parts.push(format!("**Details:** {}", details)),
            }
        }

        if let Some(r) = obj
            .get("ticket_reference")
            .or_else(|| obj.get("ticket_ref"))
        {
            let r_str = r.as_str().map(|s| s.to_string()).unwrap_or_else(|| r.to_string());
            parts.push(format!("**Ticket reference:** {r_str}"));
        }

        let body = parts.join("\n\n");
        *root = json!({ "actions": [{ "type": "add_comment", "body": body }] });
        return;
    }

    if obj.is_empty() {
        *root = json!({ "actions": [] });
        return;
    }

    *root = json!({
        "actions": [{
            "type": "add_comment",
            "body": COMMENT_BODY_PLACEHOLDER
        }]
    });
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
                    _ => json!({ "type": "add_comment", "body": COMMENT_BODY_PLACEHOLDER }),
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
                    if let Value::String(s) = val {
                        json!({ "type": "add_comment", "body": s })
                    } else {
                        json!({ "type": "add_comment", "body": COMMENT_BODY_PLACEHOLDER })
                    }
                }
            }
        }
        other => {
            if let Value::String(s) = &other {
                json!({ "type": "add_comment", "body": s.clone() })
            } else {
                json!({ "type": "add_comment", "body": COMMENT_BODY_PLACEHOLDER })
            }
        }
    }
}

fn merge_type_object(ty: &'static str, val: Value) -> Value {
    match val {
        Value::Object(mut inner) => {
            inner.insert("type".to_string(), Value::String(ty.to_string()));
            Value::Object(inner)
        }
        Value::String(s) if ty == "add_comment" => json!({ "type": "add_comment", "body": s }),
        other => {
            if ty == "add_comment" {
                json!({ "type": "add_comment", "body": COMMENT_BODY_PLACEHOLDER })
            } else {
                json!({ "type": ty, "body": other.to_string() })
            }
        }
    }
}

/// Parse the raw LLM response string into an `AgentResponse`.
/// Attempts to extract JSON even if the model wrapped it in prose/markdown.
pub fn parse_response(raw: &str) -> Result<AgentResponse, String> {
    let trimmed = strip_markdown_fence(raw);
    let relaxed = sanitize_agent_json(trimmed);

    parse_response_inner(&relaxed).or_else(|e| {
        if let Some(fixed) = try_repair_truncated_actions_json(&relaxed) {
            parse_response_inner(&fixed).map_err(|e2| {
                format!(
                    "{e}\n(recovered from likely truncated JSON; second error: {e2})"
                )
            })
        } else {
            Err(e)
        }
    })
}

fn parse_response_inner(relaxed: &str) -> Result<AgentResponse, String> {
    if let Ok(r) = serde_json::from_str::<AgentResponse>(relaxed) {
        return Ok(r);
    }

    // First complete JSON value (handles leading prose; avoids greedy `rfind('}')` bugs).
    let Some(slice) = extract_first_json_value(relaxed) else {
        return Err(format!(
            "no complete JSON object or array found (often: response truncated mid-string — raise max_tokens or shorten output); excerpt:\n{}",
            response_excerpt(relaxed, 800)
        ));
    };

    if let Ok(r) = serde_json::from_str::<AgentResponse>(slice) {
        return Ok(r);
    }

    let mut v: Value = serde_json::from_str(slice).map_err(|e| {
        format!(
            "invalid JSON: {e}; excerpt:\n{}",
            response_excerpt(slice, 1200)
        )
    })?;

    if v.is_array() {
        v = json!({ "actions": v });
    }

    coerce_root_to_actions_envelope(&mut v);
    normalize_agent_response_value(&mut v);

    serde_json::from_value(v).map_err(|e| {
        format!(
            "could not map normalized JSON to AgentResponse: {e}; excerpt:\n{}",
            response_excerpt(slice, 800)
        )
    })
}

const MAX_COMMENT_JSON_UNWRAP_DEPTH: u8 = 8;

/// Prepare `add_comment.body` for the ticket UI: unwrap nested `{"actions":[…]}` envelopes so only
/// human prose is stored; never surface full agent JSON or malformed JSON in the thread.
pub fn comment_body_for_ticket(body: &str) -> String {
    comment_body_plain_inner(body.trim(), 0)
}

fn comment_body_plain_inner(body: &str, depth: u8) -> String {
    if body.is_empty() {
        return String::new();
    }
    if depth >= MAX_COMMENT_JSON_UNWRAP_DEPTH {
        return COMMENT_BODY_PLACEHOLDER.to_string();
    }

    let looks_like_json = body.starts_with('{') || body.starts_with('[');
    if !looks_like_json {
        return body.to_string();
    }

    if let Ok(resp) = parse_response(body) {
        let mut pieces: Vec<String> = Vec::new();
        for action in &resp.actions {
            if let AgentAction::AddComment { body: b } = action {
                let inner = comment_body_plain_inner(b.trim(), depth + 1);
                if !inner.is_empty() {
                    pieces.push(inner);
                }
            }
        }
        if !pieces.is_empty() {
            return pieces.join("\n\n");
        }
        return COMMENT_BODY_PLACEHOLDER.to_string();
    }

    COMMENT_BODY_PLACEHOLDER.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_comma_in_actions_array() {
        let raw = r#"{"actions": [{"type": "add_comment", "body": "hi"},]}"#;
        let out = parse_response(raw).expect("parse");
        assert_eq!(out.actions.len(), 1);
    }

    #[test]
    fn trailing_comma_after_last_object_key() {
        let raw = r#"{"actions": [{"type": "add_comment", "body": "x",},],}"#;
        let out = parse_response(raw).expect("parse");
        assert_eq!(out.actions.len(), 1);
    }

    #[test]
    fn smart_quotes_on_keys_normalized() {
        let raw = format!("{{{}actions{}: []}}", '\u{201C}', '\u{201D}');
        let out = parse_response(&raw).expect("parse");
        assert!(out.actions.is_empty());
    }

    #[test]
    fn alternate_action_details_envelope_becomes_comment() {
        let raw = r#"{
  "action": "update_progress",
  "details": {
    "step": "Select stack",
    "output": "Use Rust"
  },
  "ticket_reference": "RAG_System_Phase_1_Architecture"
}"#;
        let out = parse_response(raw).expect("parse");
        assert_eq!(out.actions.len(), 1);
        match &out.actions[0] {
            AgentAction::AddComment { body } => {
                assert!(body.contains("update_progress"));
                assert!(body.contains("Select stack"));
                assert!(body.contains("RAG_System"));
            }
            _ => panic!("expected add_comment"),
        }
    }

    #[test]
    fn actions_singular_object_wrapped_as_one_action() {
        let raw = r#"{"actions": {"type": "add_comment", "body": "hello"}}"#;
        let out = parse_response(raw).expect("parse");
        assert_eq!(out.actions.len(), 1);
    }

    #[test]
    fn rejects_unknown_top_level_keys() {
        let raw = r#"{"actions":[],"note":"x"}"#;
        assert!(parse_response(raw).is_err());
    }

    #[test]
    fn prose_before_json_and_brace_inside_string() {
        let raw = r#"Sure — {"actions": [{"type": "add_comment", "body": "literal } brace"}]}"#;
        let out = parse_response(raw).expect("parse");
        match &out.actions[0] {
            AgentAction::AddComment { body } => assert!(body.contains('}')),
            _ => panic!("expected add_comment"),
        }
    }

    #[test]
    fn root_array_wrapped_as_actions() {
        let raw = r#"[{"type": "add_comment", "body": "x"}]"#;
        let out = parse_response(raw).expect("parse");
        assert_eq!(out.actions.len(), 1);
    }

    #[test]
    fn repairs_token_truncated_add_comment_body() {
        let raw = "{\"actions\":[{\"type\":\"add_comment\",\"body\":\"## Plan **Vision:** Keep typing until the model runs out of";
        let out = parse_response(raw).expect("parse");
        match &out.actions[0] {
            AgentAction::AddComment { body } => {
                assert!(body.contains("Vision"));
                assert!(body.contains("runs out of"));
            }
            _ => panic!("expected add_comment"),
        }
    }

    #[test]
    fn comment_body_for_ticket_unwraps_envelope() {
        let inner = "Ship the MVP draft.";
        let blob = format!(
            "{{\"actions\":[{{\"type\":\"add_comment\",\"body\":\"{}\"}}]}}",
            inner.replace('\\', "\\\\").replace('"', "\\\"")
        );
        assert_eq!(comment_body_for_ticket(&blob), inner);
    }

    #[test]
    fn comment_body_for_ticket_plain_unchanged() {
        let s = "Just a normal note with { braces } not json.";
        assert_eq!(comment_body_for_ticket(s), s);
    }

    #[test]
    fn comment_body_for_ticket_invalid_json_placeholder() {
        assert_eq!(
            comment_body_for_ticket("{unquoted: not valid json}"),
            COMMENT_BODY_PLACEHOLDER
        );
    }
}
