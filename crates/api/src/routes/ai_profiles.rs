use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{AiProfile, CreateAiProfileInput, UpdateAiProfileInput};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/ai-profiles`
pub async fn list_ai_profiles(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<AiProfile>>> {
    let profiles = db::ai_profile::list_ai_profiles(&state.pool, company_id).await?;
    Ok(Json(profiles))
}

/// `GET /v1/companies/:id/ai-profiles/:profile_id`
pub async fn get_ai_profile(
    State(state): State<AppState>,
    Path((company_id, profile_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<AiProfile>> {
    let profile = db::ai_profile::get_ai_profile(&state.pool, company_id, profile_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(profile))
}

/// `POST /v1/companies/:id/ai-profiles`
pub async fn create_ai_profile(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<CreateAiProfileInput>,
) -> ApiResult<(StatusCode, Json<AiProfile>)> {
    if input.provider_kind.trim().is_empty() {
        return Err(ApiError::BadRequest("provider_kind is required".into()));
    }
    if input.model_id.trim().is_empty() {
        return Err(ApiError::BadRequest("model_id is required".into()));
    }

    // Reject unknown/disabled providers.
    let enabled: Vec<_> = state
        .providers
        .enabled_providers()
        .into_iter()
        .map(|p| p.kind)
        .collect();
    if !enabled.contains(&input.provider_kind) {
        return Err(ApiError::BadRequest(format!(
            "provider '{}' is not enabled in this build",
            input.provider_kind
        )));
    }

    let profile = db::ai_profile::create_ai_profile(&state.pool, company_id, input).await?;
    Ok((StatusCode::CREATED, Json(profile)))
}

/// `PATCH /v1/companies/:id/ai-profiles/:profile_id`
pub async fn update_ai_profile(
    State(state): State<AppState>,
    Path((company_id, profile_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateAiProfileInput>,
) -> ApiResult<Json<AiProfile>> {
    let profile =
        db::ai_profile::update_ai_profile(&state.pool, company_id, profile_id, input)
            .await?
            .ok_or(ApiError::NotFound)?;
    Ok(Json(profile))
}
