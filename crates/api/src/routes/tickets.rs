use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db::job::{PRIORITY_CO_FOUNDER, PRIORITY_EXECUTIVE, PRIORITY_SPECIALIST};
use domain::{
    AgentTicketRunPayload, CreateCommentInput, CreateTicketInput, JobKind, PersonKind, RoleType,
    RunState, Ticket, TicketComment, UpdateTicketInput,
};
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ─── Tickets ──────────────────────────────────────────────────────────────────

/// `GET /v1/companies/:id/workspaces/:workspace_id/tickets`
pub async fn list_tickets(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<Ticket>>> {
    let tickets = db::ticket::list_tickets(&state.pool, workspace_id).await?;
    Ok(Json(tickets))
}

/// `GET /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
pub async fn get_ticket(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Ticket>> {
    let ticket = db::ticket::get_ticket(&state.pool, ticket_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(ticket))
}

/// `POST /v1/companies/:id/workspaces/:workspace_id/tickets`
///
/// After creating the ticket, auto-enqueues an agent run for every AI assignee
/// in the workspace if the company is in the `running` state.
pub async fn create_ticket(
    State(state): State<AppState>,
    Path((company_id, workspace_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateTicketInput>,
) -> ApiResult<(StatusCode, Json<Ticket>)> {
    if input.title.trim().is_empty() {
        return Err(ApiError::BadRequest("ticket title is required".into()));
    }

    let ticket = db::ticket::create_ticket(&state.pool, workspace_id, input).await?;

    // Auto-trigger agents for the new ticket.
    maybe_trigger_agents_for_ticket(&state, company_id, workspace_id, ticket.id).await;

    Ok((StatusCode::CREATED, Json(ticket)))
}

/// `PATCH /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
///
/// After updating the ticket, auto-enqueues an agent run for any AI assignee
/// if the company is running (e.g. a new assignee was just set).
pub async fn update_ticket(
    State(state): State<AppState>,
    Path((company_id, workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<UpdateTicketInput>,
) -> ApiResult<Json<Ticket>> {
    let ticket = db::ticket::update_ticket(&state.pool, ticket_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Auto-trigger agents after any update (e.g. new description, new assignee).
    maybe_trigger_agents_for_ticket(&state, company_id, workspace_id, ticket.id).await;

    Ok(Json(ticket))
}

/// `DELETE /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
pub async fn delete_ticket(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = db::ticket::delete_ticket(&state.pool, ticket_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

// ─── Comments ─────────────────────────────────────────────────────────────────

/// `GET /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/comments`
pub async fn list_comments(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Vec<TicketComment>>> {
    let comments = db::ticket::list_comments(&state.pool, ticket_id).await?;
    Ok(Json(comments))
}

/// `POST /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/comments`
///
/// After saving the comment, auto-continues the agent loop whenever a human
/// posts: if the ticket has an AI assignee and the company is running, the
/// agent is re-enqueued regardless of the current ticket status.
pub async fn create_comment(
    State(state): State<AppState>,
    Path((company_id, workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<CreateCommentInput>,
) -> ApiResult<(StatusCode, Json<TicketComment>)> {
    if input.body.trim().is_empty() {
        return Err(ApiError::BadRequest("comment body is required".into()));
    }

    let author_person_id = input.author_person_id;
    let comment = db::ticket::create_comment(&state.pool, ticket_id, input).await?;

    // Fire-and-forget: auto-continue the agent loop if applicable.
    maybe_continue_agent(&state, company_id, workspace_id, ticket_id, author_person_id).await;

    Ok((StatusCode::CREATED, Json(comment)))
}

// ─── Auto-trigger helpers ─────────────────────────────────────────────────────

/// Enqueue agent runs for all AI members of a workspace when the company is
/// running. Called on ticket create and ticket update.
async fn maybe_trigger_agents_for_ticket(
    state: &AppState,
    company_id: Uuid,
    workspace_id: Uuid,
    ticket_id: Uuid,
) {
    let company = match db::company::get_company(&state.pool, company_id).await {
        Ok(Some(c)) => c,
        _ => return,
    };

    // Terminated companies never run agents.
    if company.run_state == RunState::Terminated {
        return;
    }

    // If the simulation is paused, resume it so enqueued work can be claimed.
    // (Otherwise jobs sit pending until the user opens the company page and clicks Run.)
    if company.run_state == RunState::Stopped {
        match db::company::set_run_state(&state.pool, company_id, RunState::Running).await {
            Ok(Some(_)) => {
                info!(
                    company_id = %company_id,
                    "auto-started simulation so ticket activity can run agents"
                );
            }
            Ok(None) | Err(_) => return,
        }
    }

    // Reload the ticket to get the current assignee.
    let ticket = match db::ticket::get_ticket(&state.pool, ticket_id).await {
        Ok(Some(t)) => t,
        _ => return,
    };

    // If the ticket has a specific AI assignee, enqueue only for them.
    if let Some(assignee_id) = ticket.assignee_person_id {
        if let Ok(Some(person)) = db::person::get_person(&state.pool, company_id, assignee_id).await {
            if person.kind == PersonKind::AiAgent {
                enqueue_for_person(state, company_id, ticket_id, assignee_id, &person.role_type).await;
                return;
            }
        }
    }

    // No specific AI assignee — enqueue for all AI agents in the workspace.
    let members = match db::workspace_member::list_workspace_members(&state.pool, workspace_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    for member in members {
        if let Ok(Some(person)) = db::person::get_person(&state.pool, company_id, member.person_id).await {
            if person.kind == PersonKind::AiAgent {
                enqueue_for_person(state, company_id, ticket_id, person.id, &person.role_type).await;
            }
        }
    }
}

/// Enqueue an agent run if a human comment should (re-)trigger the agent loop.
///
/// Conditions (all must be true):
/// 1. The author is human (founder) — AI agent comments must not re-trigger.
/// 2. The ticket has an **AI agent** assignee.
/// 3. The company is in **running** state.
///
/// Note: the ticket status restriction has been removed — agents respond to
/// human messages regardless of whether the ticket is blocked or not.
async fn maybe_continue_agent(
    state: &AppState,
    company_id: Uuid,
    workspace_id: Uuid,
    ticket_id: Uuid,
    author_person_id: Option<Uuid>,
) {
    // Check author: skip if the commenter is an AI agent.
    if let Some(person_id) = author_person_id {
        match db::person::get_person(&state.pool, company_id, person_id).await {
            Ok(Some(p)) if p.kind == PersonKind::AiAgent => return,
            _ => {}
        }
    }

    maybe_trigger_agents_for_ticket(state, company_id, workspace_id, ticket_id).await;
}

/// Insert one `AgentTicketRun` job into the queue with role-based priority.
async fn enqueue_for_person(
    state: &AppState,
    company_id: Uuid,
    ticket_id: Uuid,
    person_id: Uuid,
    role_type: &RoleType,
) {
    let priority = match role_type {
        RoleType::CoFounder => PRIORITY_CO_FOUNDER,
        RoleType::Ceo | RoleType::Cto => PRIORITY_EXECUTIVE,
        RoleType::Specialist => PRIORITY_SPECIALIST,
    };

    let payload = match serde_json::to_value(AgentTicketRunPayload {
        ticket_id,
        person_id,
    }) {
        Ok(v) => v,
        Err(_) => return,
    };

    match db::job::enqueue(&state.pool, JobKind::AgentTicketRun, company_id, payload, priority).await {
        Ok(job) => {
            info!(
                job_id   = %job.id,
                ticket_id = %ticket_id,
                person_id = %person_id,
                priority = priority,
                "auto-enqueued agent run"
            );
        }
        Err(e) => {
            tracing::warn!(err = %e, "failed to auto-enqueue agent run");
        }
    }
}
