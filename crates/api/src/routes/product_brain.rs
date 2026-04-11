use axum::{
    extract::{Path, State},
    Json,
};
use domain::{
    ApprovePendingBrainInput, CreateTicketReferenceInput, ProductBrainEntry, ProductBrainPending,
    TicketReference,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/product-brain/entries`
pub async fn list_brain_entries(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ProductBrainEntry>>> {
    let rows = db::product_brain::list_entries_by_company(&state.pool, company_id, 200).await?;
    Ok(Json(rows))
}

/// `GET /v1/companies/:id/product-brain/pending`
pub async fn list_pending_brain(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ProductBrainPending>>> {
    let rows =
        db::product_brain::list_pending(&state.pool, company_id, None, 200).await?;
    Ok(Json(rows))
}

/// `POST /v1/companies/:id/product-brain/pending/:pending_id/approve`
pub async fn approve_pending_brain(
    State(state): State<AppState>,
    Path((company_id, pending_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ApprovePendingBrainInput>,
) -> ApiResult<Json<ProductBrainEntry>> {
    let pending = db::product_brain::get_pending(&state.pool, pending_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if pending.company_id != company_id {
        return Err(ApiError::NotFound);
    }
    let entry = db::product_brain::approve_pending(&state.pool, pending_id, input)
        .await?
        .ok_or(ApiError::BadRequest(
            "pending item not found or already reviewed".into(),
        ))?;
    Ok(Json(entry))
}

/// `POST /v1/companies/:id/product-brain/pending/:pending_id/reject`
pub async fn reject_pending_brain(
    State(state): State<AppState>,
    Path((company_id, pending_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let pending = db::product_brain::get_pending(&state.pool, pending_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if pending.company_id != company_id {
        return Err(ApiError::NotFound);
    }
    let ok = db::product_brain::reject_pending(&state.pool, pending_id).await?;
    if !ok {
        return Err(ApiError::BadRequest(
            "pending item not found or not rejectable".into(),
        ));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `GET /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/references`
pub async fn list_ticket_references(
    State(state): State<AppState>,
    Path((company_id, workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Vec<TicketReference>>> {
    let ticket = db::ticket::get_ticket(&state.pool, ticket_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ticket.workspace_id != workspace_id {
        return Err(ApiError::NotFound);
    }
    let ws = db::workspace::get_workspace(&state.pool, workspace_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ws.company_id != company_id {
        return Err(ApiError::NotFound);
    }
    let refs = db::product_brain::list_references_from(&state.pool, ticket_id).await?;
    Ok(Json(refs))
}

/// `POST /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/references`
pub async fn create_ticket_reference(
    State(state): State<AppState>,
    Path((company_id, workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<CreateTicketReferenceInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let ticket = db::ticket::get_ticket(&state.pool, ticket_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ticket.workspace_id != workspace_id {
        return Err(ApiError::NotFound);
    }
    let ws = db::workspace::get_workspace(&state.pool, workspace_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ws.company_id != company_id {
        return Err(ApiError::NotFound);
    }
    db::product_brain::add_ticket_reference(
        &state.pool,
        ticket_id,
        input.to_ticket_id,
        input.note,
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `DELETE /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/references/:to_ticket_id`
pub async fn delete_ticket_reference(
    State(state): State<AppState>,
    Path((company_id, workspace_id, ticket_id, to_ticket_id)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let ticket = db::ticket::get_ticket(&state.pool, ticket_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ticket.workspace_id != workspace_id {
        return Err(ApiError::NotFound);
    }
    let ws = db::workspace::get_workspace(&state.pool, workspace_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if ws.company_id != company_id {
        return Err(ApiError::NotFound);
    }
    db::product_brain::remove_ticket_reference(&state.pool, ticket_id, to_ticket_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
