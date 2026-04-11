use axum::{extract::State, http::StatusCode, Json};
use domain::{ResetInstallInput, RESET_INSTALL_CONFIRM_PHRASE};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `POST /v1/system/reset-install`
///
/// Removes every company and all associated data for this local install, returning
/// bootstrap to “onboarding first”. Caller must send the exact confirmation phrase.
pub async fn reset_install(
    State(state): State<AppState>,
    Json(input): Json<ResetInstallInput>,
) -> ApiResult<StatusCode> {
    if input.confirm_phrase.trim() != RESET_INSTALL_CONFIRM_PHRASE {
        return Err(ApiError::BadRequest(
            "confirm_phrase does not match the required phrase".into(),
        ));
    }

    db::company::delete_all_companies(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
