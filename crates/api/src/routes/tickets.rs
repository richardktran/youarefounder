use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{CreateCommentInput, CreateTicketInput, Ticket, TicketComment, UpdateTicketInput};
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
pub async fn create_comment(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<CreateCommentInput>,
) -> ApiResult<(StatusCode, Json<TicketComment>)> {
    if input.body.trim().is_empty() {
        return Err(ApiError::BadRequest("comment body is required".into()));
    }

    let comment = db::ticket::create_comment(&state.pool, ticket_id, input).await?;
    Ok((StatusCode::CREATED, Json(comment)))
}
