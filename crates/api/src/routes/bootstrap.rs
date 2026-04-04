use axum::{extract::State, Json};
use domain::BootstrapStatus;

use crate::{error::ApiResult, state::AppState};

/// `GET /v1/bootstrap`
///
/// Returns the minimal status needed for the frontend to decide:
/// - Show onboarding wizard, or
/// - Jump straight to the app (company exists + onboarding complete)
pub async fn get_bootstrap(State(state): State<AppState>) -> ApiResult<Json<BootstrapStatus>> {
    let status = db::company::get_bootstrap_status(&state.pool).await?;
    Ok(Json(status))
}
