//! Phase 4: company simulation controls — Run / Stop / Terminate.

use axum::{
    extract::{Path, State},
    Json,
};
use domain::{Company, RunState};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `POST /v1/companies/:id/run`
///
/// Transition the company to Running state so agents can process jobs.
pub async fn run_company(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Company>> {
    let company = db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if !company.onboarding_complete {
        return Err(ApiError::BadRequest(
            "complete onboarding before starting the simulation".into(),
        ));
    }

    let updated = db::company::set_run_state(&state.pool, company_id, RunState::Running)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(updated))
}

/// `POST /v1/companies/:id/stop`
///
/// Pause the simulation. Jobs stay in the queue but are not claimed.
pub async fn stop_company(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Company>> {
    let updated = db::company::set_run_state(&state.pool, company_id, RunState::Stopped)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(updated))
}

#[derive(Debug, Deserialize)]
pub struct TerminateInput {
    /// Must match the company name exactly to confirm the irreversible action.
    pub confirm_name: String,
}

/// `POST /v1/companies/:id/terminate`
///
/// Permanently delete the company and all its associated data.
/// The caller must echo back the company name in `confirm_name` for safety.
pub async fn terminate_company(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<TerminateInput>,
) -> ApiResult<axum::http::StatusCode> {
    let company = db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if input.confirm_name.trim() != company.name.trim() {
        return Err(ApiError::BadRequest(
            "confirm_name does not match the company name".into(),
        ));
    }

    let deleted = db::company::terminate_company(&state.pool, company_id).await?;
    if !deleted {
        return Err(ApiError::NotFound);
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
