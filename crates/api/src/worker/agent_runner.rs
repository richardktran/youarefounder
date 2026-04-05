//! Executes one `AgentTicketRun` job end-to-end:
//! build context → call LLM → parse actions → apply → record history.

use anyhow::{Context, Result};
use ai_core::{ChatCompletionRequest, Message};
use ai_providers::ProviderRegistry;
use domain::{
    AgentTicketRunPayload, CreateCommentInput, CreateDecisionRequestInput, CreateProposalInput,
    CreateTicketInput, JobKind, TicketPriority, TicketStatus, TicketType, UpdateTicketInput,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::actions::{parse_response, AgentAction};
use super::context::ContextPack;
use super::scheduler::role_priority;
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

            AgentAction::UpdateTicket {
                title,
                description,
                status,
                priority,
                assignee_person_id,
            } => {
                let status_parsed = status
                    .as_deref()
                    .and_then(|s| s.parse::<TicketStatus>().ok());
                let priority_parsed = priority
                    .as_deref()
                    .and_then(|p| p.parse::<TicketPriority>().ok());

                // Validate the assignee exists in the team before applying.
                let resolved_assignee = assignee_person_id.and_then(|aid| {
                    ctx.all_people.iter().find(|p| p.id == aid).cloned()
                });

                db::ticket::update_ticket(
                    pool,
                    ctx.ticket.id,
                    UpdateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        status: status_parsed,
                        priority: priority_parsed,
                        ticket_type: None,
                        assignee_person_id: resolved_assignee.as_ref().map(|p| p.id),
                        parent_ticket_id: None,
                    },
                )
                .await
                .context("update_ticket")?;

                // If ticket was reassigned to an AI agent, immediately enqueue a job for them.
                if let Some(new_assignee) = &resolved_assignee {
                    if matches!(new_assignee.kind, domain::PersonKind::AiAgent)
                        && new_assignee.ai_profile_id.is_some()
                    {
                        let job_priority = role_priority(&new_assignee.role_type);
                        let payload = json!({
                            "ticket_id": ctx.ticket.id,
                            "person_id": new_assignee.id,
                        });
                        match db::job::enqueue(
                            pool,
                            JobKind::AgentTicketRun,
                            company_id,
                            payload,
                            job_priority,
                        )
                        .await
                        {
                            Ok(j) => info!(
                                job_id = %j.id,
                                assignee = %new_assignee.display_name,
                                "auto-enqueued job for reassigned ticket"
                            ),
                            Err(e) => warn!(err = %e, "failed to auto-enqueue for reassigned ticket"),
                        }
                    }
                }

                applied.push(json!({
                    "type": "update_ticket",
                    "status": status,
                    "title": title,
                    "assignee_person_id": assignee_person_id,
                }));
            }

            AgentAction::CreateTicket {
                title,
                description,
                ticket_type,
                status,
                priority,
                assignee_person_id,
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

                // Resolve assignee: use explicit override if valid, otherwise default to self.
                let resolved_assignee = assignee_person_id
                    .and_then(|aid| ctx.all_people.iter().find(|p| p.id == aid).cloned())
                    .unwrap_or_else(|| ctx.assignee.clone());

                let new_ticket = db::ticket::create_ticket(
                    pool,
                    ws_id,
                    CreateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        ticket_type: Some(type_parsed),
                        status: Some(status_parsed),
                        priority: Some(priority_parsed),
                        assignee_person_id: Some(resolved_assignee.id),
                        parent_ticket_id: None,
                    },
                )
                .await
                .context("create_ticket")?;

                // If the assignee is an AI agent, immediately enqueue a job so they
                // can start working without waiting for the next scheduler tick.
                if matches!(resolved_assignee.kind, domain::PersonKind::AiAgent)
                    && resolved_assignee.ai_profile_id.is_some()
                {
                    let job_priority = role_priority(&resolved_assignee.role_type);
                    let payload = json!({
                        "ticket_id": new_ticket.id,
                        "person_id": resolved_assignee.id,
                    });
                    match db::job::enqueue(
                        pool,
                        JobKind::AgentTicketRun,
                        company_id,
                        payload,
                        job_priority,
                    )
                    .await
                    {
                        Ok(j) => info!(
                            job_id = %j.id,
                            ticket_id = %new_ticket.id,
                            assignee = %resolved_assignee.display_name,
                            "auto-enqueued job for new ticket"
                        ),
                        Err(e) => warn!(err = %e, "failed to auto-enqueue for new ticket"),
                    }
                }

                applied.push(json!({
                    "type": "create_ticket",
                    "title": title,
                    "assignee_person_id": resolved_assignee.id,
                }));
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

            AgentAction::RequestDecision {
                question,
                context_note,
            } => {
                db::decision::create_decision_request(
                    pool,
                    company_id,
                    CreateDecisionRequestInput {
                        ticket_id: ctx.ticket.id,
                        raised_by_person_id: Some(person_id),
                        question: question.clone(),
                        context_note: context_note.clone(),
                    },
                )
                .await
                .context("request_decision")?;
                applied.push(json!({
                    "type": "request_decision",
                    "question": question,
                }));
            }
        }
    }

    Ok(serde_json::Value::Array(applied))
}
