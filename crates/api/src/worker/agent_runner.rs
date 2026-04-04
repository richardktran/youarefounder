//! Executes one `AgentTicketRun` job end-to-end:
//! build context → call LLM → parse actions → apply → record history.

use anyhow::{Context, Result};
use ai_core::{ChatCompletionRequest, Message};
use ai_providers::ProviderRegistry;
use domain::{
    AgentTicketRunPayload, CreateCommentInput, CreateProposalInput, CreateTicketInput,
    TicketStatus, TicketPriority, TicketType, UpdateTicketInput,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::actions::{parse_response, AgentAction};
use super::context::ContextPack;
use crate::job_events::JobEvent;

/// Run one agent turn for the given job.
///
/// Guarantees that exactly one terminal event (`Completed` or `Failed`) is
/// broadcast on every code path, preceded by a `Started` event at the
/// beginning of the LLM call.
pub async fn run_agent_job(
    pool: &PgPool,
    registry: &ProviderRegistry,
    job_id: Uuid,
    company_id: Uuid,
    payload: &serde_json::Value,
    events_tx: &broadcast::Sender<JobEvent>,
) -> Result<()> {
    events_tx.send(JobEvent::Started { job_id }).ok();

    let result = execute(pool, registry, job_id, company_id, payload).await;

    match &result {
        Ok(()) => {
            events_tx.send(JobEvent::Completed { job_id }).ok();
            info!(job_id = %job_id, "agent run complete");
        }
        Err(e) => {
            error!(job_id = %job_id, err = %e, "agent run failed");
            events_tx
                .send(JobEvent::Failed {
                    job_id,
                    error: e.to_string(),
                })
                .ok();
        }
    }

    result
}

/// Inner logic — returns `Err` on any failure so the outer wrapper can emit
/// the correct terminal event regardless of which step failed.
async fn execute(
    pool: &PgPool,
    registry: &ProviderRegistry,
    job_id: Uuid,
    company_id: Uuid,
    payload: &serde_json::Value,
) -> Result<()> {
    // ── Deserialize payload ───────────────────────────────────────────────────
    let p: AgentTicketRunPayload =
        serde_json::from_value(payload.clone()).context("deserialize job payload")?;

    info!(
        job_id = %job_id,
        ticket_id = %p.ticket_id,
        person_id = %p.person_id,
        "agent run starting"
    );

    // ── Build context ─────────────────────────────────────────────────────────
    let ctx = ContextPack::build(pool, p.ticket_id, p.person_id)
        .await
        .context("build context pack")?;

    // ── Resolve AI profile ────────────────────────────────────────────────────
    let ai_profile_id = ctx
        .assignee
        .ai_profile_id
        .context("assignee has no AI profile — cannot run agent")?;

    let profile = db::ai_profile::get_ai_profile(pool, company_id, ai_profile_id)
        .await
        .context("load ai profile")?
        .context("ai profile not found")?;

    // ── Build adapter ─────────────────────────────────────────────────────────
    let adapter = registry
        .build_adapter(&profile.provider_kind, &profile.provider_config)
        .map_err(|e| anyhow::anyhow!("build adapter: {e}"))?;

    // ── Call LLM ──────────────────────────────────────────────────────────────
    let prompt = ctx.build_prompt();

    let req = ChatCompletionRequest {
        model: profile.model_id.clone(),
        messages: vec![Message::user(&prompt)],
        temperature: profile.default_temperature,
        max_tokens: profile.default_max_tokens.map(|t| t as u32),
    };

    info!(job_id = %job_id, model = %profile.model_id, "calling LLM");

    let resp = adapter
        .complete(req)
        .await
        .map_err(|e| anyhow::anyhow!("LLM inference failed: {e}"))?;

    let raw = resp.content;

    info!(
        job_id = %job_id,
        chars = raw.len(),
        finish_reason = ?resp.finish_reason,
        "LLM responded, parsing actions"
    );

    // ── Parse actions ─────────────────────────────────────────────────────────
    let agent_resp = match parse_response(&raw) {
        Ok(r) => r,
        Err(e) => {
            warn!(job_id = %job_id, err = %e, "failed to parse agent response");
            // Record the malformed run so it shows up in history.
            db::agent_run::record_run(
                pool,
                job_id,
                p.ticket_id,
                p.person_id,
                None,
                None,
                Some(&raw),
                &json!([]),
                Some(&e),
            )
            .await
            .ok();
            return Err(anyhow::anyhow!("parse agent response: {e}"));
        }
    };

    // ── Apply actions ─────────────────────────────────────────────────────────
    let applied = apply_actions(pool, company_id, &ctx, p.person_id, &agent_resp.actions)
        .await
        .context("apply actions")?;

    // ── Record run history ────────────────────────────────────────────────────
    db::agent_run::record_run(
        pool,
        job_id,
        p.ticket_id,
        p.person_id,
        None,
        None,
        Some(&raw),
        &applied,
        None,
    )
    .await
    .context("record agent run")?;

    Ok(())
}

/// Apply all actions in order. Returns JSON of applied actions for audit.
async fn apply_actions(
    pool: &PgPool,
    company_id: Uuid,
    ctx: &ContextPack,
    person_id: Uuid,
    actions: &[AgentAction],
) -> Result<serde_json::Value> {
    let mut applied: Vec<serde_json::Value> = Vec::new();

    for action in actions {
        match action {
            AgentAction::AddComment { body } => {
                db::ticket::create_comment(
                    pool,
                    ctx.ticket.id,
                    CreateCommentInput {
                        body: body.clone(),
                        author_person_id: Some(person_id),
                    },
                )
                .await
                .context("add_comment")?;
                applied.push(json!({"type": "add_comment", "body": body}));
            }

            AgentAction::UpdateTicket { title, description, status, priority } => {
                let status_parsed = status
                    .as_deref()
                    .and_then(|s| s.parse::<TicketStatus>().ok());
                let priority_parsed = priority
                    .as_deref()
                    .and_then(|p| p.parse::<TicketPriority>().ok());

                db::ticket::update_ticket(
                    pool,
                    ctx.ticket.id,
                    UpdateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        status: status_parsed,
                        priority: priority_parsed,
                        ticket_type: None,
                        assignee_person_id: None,
                        parent_ticket_id: None,
                    },
                )
                .await
                .context("update_ticket")?;
                applied.push(json!({"type": "update_ticket", "status": status, "title": title}));
            }

            AgentAction::CreateTicket {
                title,
                description,
                ticket_type,
                status,
                priority,
                workspace_id,
            } => {
                let ws_id = workspace_id.unwrap_or(ctx.ticket.workspace_id);
                let type_parsed = ticket_type
                    .as_deref()
                    .and_then(|t| t.parse::<TicketType>().ok())
                    .unwrap_or_default();
                let status_parsed = status
                    .as_deref()
                    .and_then(|s| s.parse::<TicketStatus>().ok())
                    .unwrap_or_default();
                let priority_parsed = priority
                    .as_deref()
                    .and_then(|p| p.parse::<TicketPriority>().ok())
                    .unwrap_or_default();

                db::ticket::create_ticket(
                    pool,
                    ws_id,
                    CreateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        ticket_type: Some(type_parsed),
                        status: Some(status_parsed),
                        priority: Some(priority_parsed),
                        assignee_person_id: Some(person_id),
                        parent_ticket_id: None,
                    },
                )
                .await
                .context("create_ticket")?;
                applied.push(json!({"type": "create_ticket", "title": title}));
            }

            AgentAction::ProposeHire {
                employee_display_name,
                role_type,
                specialty,
                rationale,
                scope_of_work,
            } => {
                db::hiring::create_proposal(
                    pool,
                    company_id,
                    CreateProposalInput {
                        employee_display_name: employee_display_name.clone(),
                        role_type: role_type.clone(),
                        specialty: specialty.clone(),
                        ai_profile_id: None,
                        rationale: rationale.clone(),
                        scope_of_work: scope_of_work.clone(),
                        proposed_by_person_id: Some(person_id),
                    },
                )
                .await
                .context("propose_hire")?;
                applied.push(json!({
                    "type": "propose_hire",
                    "employee_display_name": employee_display_name,
                    "role_type": role_type,
                }));
            }
        }
    }

    Ok(serde_json::Value::Array(applied))
}
