use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{AddWorkspaceMemberInput, WorkspaceMember, WorkspaceMemberRole};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/workspaces/:workspace_id/members`
pub async fn list_workspace_members(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<WorkspaceMember>>> {
    let members =
        db::workspace_member::list_workspace_members(&state.pool, workspace_id).await?;
    Ok(Json(members))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub person_id: Uuid,
    pub role: Option<String>,
}

/// `POST /v1/companies/:id/workspaces/:workspace_id/members`
pub async fn add_workspace_member(
    State(state): State<AppState>,
    Path((_company_id, workspace_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AddMemberRequest>,
) -> ApiResult<(StatusCode, Json<WorkspaceMember>)> {
    let role = req
        .role
        .unwrap_or_else(|| "member".into())
        .parse::<WorkspaceMemberRole>()
        .map_err(|e| ApiError::BadRequest(e))?;

    let input = AddWorkspaceMemberInput {
        person_id: req.person_id,
        role,
    };

    let member =
        db::workspace_member::add_workspace_member(&state.pool, workspace_id, input).await?;
    Ok((StatusCode::CREATED, Json(member)))
}

/// `DELETE /v1/companies/:id/workspaces/:workspace_id/members/:person_id`
pub async fn remove_workspace_member(
    State(state): State<AppState>,
    Path((_company_id, workspace_id, person_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let removed =
        db::workspace_member::remove_workspace_member(&state.pool, workspace_id, person_id)
            .await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
