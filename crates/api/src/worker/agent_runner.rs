//! Executes one `AgentTicketRun` job end-to-end:
//! build context → call LLM → parse actions → apply → record history.

use anyhow::{Context, Result};
use ai_core::{ChatCompletionRequest, Message};
use ai_providers::ProviderRegistry;
use domain::{
    AgentTicketRunPayload, CreateCommentInput, CreateDecisionRequestInput, CreateProposalInput,
    CreateTicketInput, JobKind, RoleType, TicketPriority, TicketStatus, TicketType,
    UpdateTicketInput,
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
            error!(job_id = %job_id, "agent run failed: {:#}", e);
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

    if let Ok(Some(ticket)) = db::ticket::get_ticket(pool, p.ticket_id).await {
        if matches!(
            ticket.status,
            TicketStatus::Done | TicketStatus::Cancelled
        ) {
            info!(
                job_id = %job_id,
                ticket_id = %p.ticket_id,
                "skipping agent run: ticket is done or cancelled"
            );
            return Ok(());
        }
    }

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
    let applied = apply_actions(
        pool,
        company_id,
        job_id,
        &ctx,
        p.person_id,
        &agent_resp.actions,
    )
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
    job_id: Uuid,
    ctx: &ContextPack,
    person_id: Uuid,
    actions: &[AgentAction],
) -> Result<serde_json::Value> {
    let mut applied: Vec<serde_json::Value> = Vec::new();

    for (i, action) in actions.iter().enumerate() {
        let step = format!("[{i}]");
        match action {
            AgentAction::AddComment { body } => {
                if body.trim().is_empty() {
                    warn!(
                        job_id = %job_id,
                        step = %step,
                        "skipping empty add_comment (models sometimes emit blank comments)"
                    );
                    continue;
                }
                db::ticket::create_comment(
                    pool,
                    ctx.ticket.id,
                    CreateCommentInput {
                        body: body.clone(),
                        author_person_id: Some(person_id),
                    },
                )
                .await
                .with_context(|| format!("{step} add_comment"))?;
                applied.push(json!({"type": "add_comment", "body": body}));
            }

            AgentAction::UpdateTicket {
                title,
                description,
                definition_of_done,
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

                let updated = db::ticket::update_ticket(
                    pool,
                    ctx.ticket.id,
                    UpdateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        definition_of_done: definition_of_done.clone(),
                        founder_memory: None,
                        outcome_summary: None,
                        status: status_parsed,
                        priority: priority_parsed,
                        ticket_type: None,
                        assignee_person_id: resolved_assignee.as_ref().map(|p| p.id),
                        parent_ticket_id: None,
                    },
                )
                .await
                .with_context(|| format!("{step} update_ticket"))?;

                // If ticket was reassigned to an AI agent, immediately enqueue a job for them.
                if let (Some(new_assignee), Some(t)) = (&resolved_assignee, &updated) {
                    if matches!(new_assignee.kind, domain::PersonKind::AiAgent)
                        && new_assignee.ai_profile_id.is_some()
                        && !matches!(
                            t.status,
                            TicketStatus::Done | TicketStatus::Cancelled
                        )
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
                definition_of_done,
                ticket_type,
                status,
                priority,
                assignee_person_id,
                workspace_id,
            } => {
                let title = if title.trim().is_empty() {
                    warn!(job_id = %job_id, %step, "create_ticket with empty title — using placeholder");
                    "Untitled ticket".to_string()
                } else {
                    title.clone()
                };
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
                        definition_of_done: definition_of_done.clone(),
                        founder_memory: None,
                        outcome_summary: None,
                        ticket_type: Some(type_parsed),
                        status: Some(status_parsed),
                        priority: Some(priority_parsed),
                        assignee_person_id: Some(resolved_assignee.id),
                        parent_ticket_id: None,
                    },
                )
                .await
                .with_context(|| format!("{step} create_ticket"))?;

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

            AgentAction::CreateSubtask {
                title,
                description,
                definition_of_done,
                status,
                priority,
                assignee_person_id,
            } => {
                if ctx.ticket.parent_ticket_id.is_some() {
                    warn!(
                        job_id = %job_id,
                        %step,
                        "skipping create_subtask — only one level of subtasks (not under a subtask)"
                    );
                    applied.push(json!({
                        "type": "create_subtask",
                        "skipped": true,
                        "reason": "only one level of subtasks is allowed"
                    }));
                    continue;
                }
                let title = if title.trim().is_empty() {
                    warn!(job_id = %job_id, %step, "create_subtask with empty title — using placeholder");
                    "Untitled subtask".to_string()
                } else {
                    title.clone()
                };
                let status_parsed = status
                    .as_deref()
                    .and_then(|s| s.parse::<TicketStatus>().ok())
                    .unwrap_or(TicketStatus::Todo);
                let priority_parsed = priority
                    .as_deref()
                    .and_then(|p| p.parse::<TicketPriority>().ok())
                    .unwrap_or_default();

                let resolved_assignee = assignee_person_id
                    .and_then(|aid| ctx.all_people.iter().find(|p| p.id == aid).cloned())
                    .unwrap_or_else(|| ctx.assignee.clone());

                let new_ticket = db::ticket::create_ticket(
                    pool,
                    ctx.ticket.workspace_id,
                    CreateTicketInput {
                        title: title.clone(),
                        description: description.clone(),
                        definition_of_done: definition_of_done.clone(),
                        founder_memory: None,
                        outcome_summary: None,
                        ticket_type: Some(TicketType::Task),
                        status: Some(status_parsed),
                        priority: Some(priority_parsed),
                        assignee_person_id: Some(resolved_assignee.id),
                        parent_ticket_id: Some(ctx.ticket.id),
                    },
                )
                .await
                .with_context(|| format!("{step} create_subtask"))?;

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
                            "auto-enqueued job for new subtask"
                        ),
                        Err(e) => warn!(err = %e, "failed to auto-enqueue for new subtask"),
                    }
                }

                applied.push(json!({
                    "type": "create_subtask",
                    "title": title,
                    "parent_ticket_id": ctx.ticket.id,
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
                let name = if employee_display_name.trim().is_empty() {
                    warn!(job_id = %job_id, %step, "propose_hire with empty name — using placeholder");
                    "New hire".to_string()
                } else {
                    employee_display_name.clone()
                };
                let role_norm = role_type
                    .trim()
                    .parse::<RoleType>()
                    .unwrap_or_else(|_| {
                        warn!(
                            job_id = %job_id,
                            step = %step,
                            raw = %role_type,
                            "invalid role_type in propose_hire; defaulting to specialist"
                        );
                        RoleType::Specialist
                    })
                    .to_string();
                db::hiring::create_proposal(
                    pool,
                    company_id,
                    CreateProposalInput {
                        employee_display_name: name.clone(),
                        role_type: role_norm.clone(),
                        specialty: specialty.clone(),
                        ai_profile_id: None,
                        rationale: rationale.clone(),
                        scope_of_work: scope_of_work.clone(),
                        proposed_by_person_id: Some(person_id),
                    },
                )
                .await
                .with_context(|| format!("{step} propose_hire"))?;
                applied.push(json!({
                    "type": "propose_hire",
                    "employee_display_name": name,
                    "role_type": role_norm,
                }));
            }

            AgentAction::RequestDecision {
                question,
                context_note,
            } => {
                let q = question.trim();
                let question_text = if q.is_empty() {
                    warn!(
                        job_id = %job_id,
                        step = %step,
                        "request_decision with empty question — using placeholder (model output was invalid)"
                    );
                    "(The model did not provide a decision question.)".to_string()
                } else {
                    question.clone()
                };
                db::decision::create_decision_request(
                    pool,
                    company_id,
                    CreateDecisionRequestInput {
                        ticket_id: ctx.ticket.id,
                        raised_by_person_id: Some(person_id),
                        question: question_text.clone(),
                        context_note: context_note.clone(),
                    },
                )
                .await
                .with_context(|| format!("{step} request_decision"))?;
                applied.push(json!({
                    "type": "request_decision",
                    "question": question_text,
                }));
            }

            AgentAction::AddTicketReference {
                to_ticket_id,
                note,
            } => {
                db::product_brain::add_ticket_reference(
                    pool,
                    ctx.ticket.id,
                    *to_ticket_id,
                    note.clone(),
                )
                .await
                .with_context(|| format!("{step} add_ticket_reference"))?;
                applied.push(json!({
                    "type": "add_ticket_reference",
                    "to_ticket_id": to_ticket_id,
                }));
            }

            AgentAction::RemoveTicketReference { to_ticket_id } => {
                db::product_brain::remove_ticket_reference(pool, ctx.ticket.id, *to_ticket_id)
                    .await
                    .with_context(|| format!("{step} remove_ticket_reference"))?;
                applied.push(json!({
                    "type": "remove_ticket_reference",
                    "to_ticket_id": to_ticket_id,
                }));
            }

            AgentAction::ProposeBrainInsight { summary, detail } => {
                let mut body = summary.trim().to_string();
                if body.is_empty() {
                    body = "(empty proposal)".to_string();
                }
                if let Some(ref d) = detail {
                    let t = d.trim();
                    if !t.is_empty() {
                        body.push_str("\n\n");
                        body.push_str(t);
                    }
                }
                let _id = db::product_brain::insert_pending(
                    pool,
                    ctx.company_id,
                    Some(ctx.ticket.workspace_id),
                    body,
                    Some(ctx.ticket.id),
                )
                .await
                .with_context(|| format!("{step} propose_brain_insight"))?;
                applied.push(json!({
                    "type": "propose_brain_insight",
                    "ticket_id": ctx.ticket.id,
                }));
            }
        }
    }

    Ok(serde_json::Value::Array(applied))
}
