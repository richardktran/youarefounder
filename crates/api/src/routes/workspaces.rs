use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{CreateWorkspaceInput, UpdateWorkspaceInput, Workspace};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/workspaces`
pub async fn list_workspaces(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Workspace>>> {
    let workspaces = db::workspace::list_workspaces(&state.pool, company_id).await?;
    Ok(Json(workspaces))
}

/// `GET /v1/companies/:id/workspaces/:workspace_id`
pub async fn get_workspace(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Workspace>> {
    let workspace = db::workspace::get_workspace(&state.pool, workspace_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(workspace))
}

/// `POST /v1/companies/:id/workspaces`
pub async fn create_workspace(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<CreateWorkspaceInput>,
) -> ApiResult<(StatusCode, Json<Workspace>)> {
    if input.name.trim().is_empty() {
        return Err(ApiError::BadRequest("workspace name is required".into()));
    }

    let workspace = db::workspace::create_workspace(&state.pool, company_id, input).await?;
    db::workspace_member::ensure_ai_cofounders_in_workspace(
        &state.pool,
        company_id,
        workspace.id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(workspace)))
}

/// `PATCH /v1/companies/:id/workspaces/:workspace_id`
pub async fn update_workspace(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateWorkspaceInput>,
) -> ApiResult<Json<Workspace>> {
    let workspace = db::workspace::update_workspace(&state.pool, workspace_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(workspace))
}

/// `DELETE /v1/companies/:id/workspaces/:workspace_id`
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = db::workspace::delete_workspace(&state.pool, workspace_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
