use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{
    AgentTicketRunPayload, CreateCommentInput, CreateTicketInput, JobKind, PersonKind, RunState,
    Ticket, TicketComment, TicketStatus, UpdateTicketInput,
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
pub async fn create_ticket(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateTicketInput>,
) -> ApiResult<(StatusCode, Json<Ticket>)> {
    if input.title.trim().is_empty() {
        return Err(ApiError::BadRequest("ticket title is required".into()));
    }

    let ticket = db::ticket::create_ticket(&state.pool, workspace_id, input).await?;
    Ok((StatusCode::CREATED, Json(ticket)))
}

/// `PATCH /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
pub async fn update_ticket(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<UpdateTicketInput>,
) -> ApiResult<Json<Ticket>> {
    let ticket = db::ticket::update_ticket(&state.pool, ticket_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;
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
/// After saving the comment, checks whether to auto-continue the agent loop:
/// if the comment comes from a **human** (founder) and the ticket is **blocked**
/// with an **AI assignee** in a **running** company, enqueue an `AgentTicketRun`.
pub async fn create_comment(
    State(state): State<AppState>,
    Path((company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<CreateCommentInput>,
) -> ApiResult<(StatusCode, Json<TicketComment>)> {
    if input.body.trim().is_empty() {
        return Err(ApiError::BadRequest("comment body is required".into()));
    }

    let author_person_id = input.author_person_id;
    let comment = db::ticket::create_comment(&state.pool, ticket_id, input).await?;

    // Fire-and-forget: auto-continue the agent loop if applicable.
    maybe_continue_agent(&state, company_id, ticket_id, author_person_id).await;

    Ok((StatusCode::CREATED, Json(comment)))
}

/// Enqueue an agent run if this human comment should unblock the ticket loop.
///
/// Conditions (all must be true):
/// 1. The author is human (founder) or unattributed (None = founder UI post).
/// 2. The ticket is **blocked** and has an **AI agent** assignee.
/// 3. The company is in **running** state.
async fn maybe_continue_agent(
    state: &AppState,
    company_id: Uuid,
    ticket_id: Uuid,
    author_person_id: Option<Uuid>,
) {
    // Check author kind: None = founder web comment, Some(id) = look up person.
    if let Some(person_id) = author_person_id {
        match db::person::get_person(&state.pool, company_id, person_id).await {
            Ok(Some(p)) if p.kind == PersonKind::AiAgent => {
                // AI agent commented — do not re-trigger.
                return;
            }
            _ => {} // human or not found — continue with checks
        }
    }

    // Load ticket.
    let ticket = match db::ticket::get_ticket(&state.pool, ticket_id).await {
        Ok(Some(t)) => t,
        _ => return,
    };

    // Must be blocked.
    if ticket.status != TicketStatus::Blocked {
        return;
    }

    // Must have an AI assignee.
    let assignee_id = match ticket.assignee_person_id {
        Some(id) => id,
        None => return,
    };

    let assignee = match db::person::get_person(&state.pool, company_id, assignee_id).await {
        Ok(Some(p)) => p,
        _ => return,
    };

    if assignee.kind != PersonKind::AiAgent {
        return;
    }

    // Company must be running.
    let company = match db::company::get_company(&state.pool, company_id).await {
        Ok(Some(c)) => c,
        _ => return,
    };

    if company.run_state != RunState::Running {
        return;
    }

    // All conditions met — enqueue.
    let payload = match serde_json::to_value(AgentTicketRunPayload {
        ticket_id,
        person_id: assignee_id,
    }) {
        Ok(v) => v,
        Err(_) => return,
    };

    match db::job::enqueue(&state.pool, JobKind::AgentTicketRun, company_id, payload).await {
        Ok(job) => {
            info!(
                job_id = %job.id,
                ticket_id = %ticket_id,
                assignee = %assignee.display_name,
                "auto-enqueued agent run after founder comment on blocked ticket"
            );
        }
        Err(e) => {
            tracing::warn!(err = %e, "failed to auto-enqueue agent run");
        }
    }
}
