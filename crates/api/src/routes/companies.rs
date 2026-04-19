use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{Company, CreateCompanyInput, UpdateCompanyInput};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies`
pub async fn list_companies(State(state): State<AppState>) -> ApiResult<Json<Vec<Company>>> {
    let companies = db::company::list_companies(&state.pool).await?;
    Ok(Json(companies))
}

/// `GET /v1/companies/:id`
pub async fn get_company(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Company>> {
    let company = db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(company))
}

/// `POST /v1/companies`
///
/// Creates a company and optionally an inline first product.
/// On success returns 201 Created.
pub async fn create_company(
    State(state): State<AppState>,
    Json(input): Json<CreateCompanyInput>,
) -> ApiResult<(StatusCode, Json<Company>)> {
    if input.name.trim().is_empty() {
        return Err(ApiError::BadRequest("company name is required".into()));
    }

    let company = db::company::create_company(&state.pool, input).await?;
    Ok((StatusCode::CREATED, Json(company)))
}

/// `PATCH /v1/companies/:id`
pub async fn update_company(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<UpdateCompanyInput>,
) -> ApiResult<Json<Company>> {
    let company = db::company::update_company(&state.pool, company_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(company))
}

/// `POST /v1/companies/:id/complete-onboarding`
///
/// Validates all Phase 1 onboarding requirements are met, then flips
/// `onboarding_complete = true`:
///   1. At least one product exists.
///   2. At least one AI agent person (co-founder) exists with an AI profile linked.
pub async fn complete_onboarding(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Company>> {
    let company = db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Gate 1: at least one product.
    let products = db::product::list_products(&state.pool, company.id).await?;
    if products.is_empty() {
        return Err(ApiError::BadRequest(
            "company must have at least one product before completing onboarding".into(),
        ));
    }

    // Gate 2: at least one AI agent (co-founder) with an AI profile.
    let people = db::person::list_people(&state.pool, company.id).await?;
    let has_ai_cofounder = people.iter().any(|p| {
        matches!(p.kind, domain::PersonKind::AiAgent) && p.ai_profile_id.is_some()
    });
    if !has_ai_cofounder {
        return Err(ApiError::BadRequest(
            "company must have an AI co-founder with a configured AI profile before completing onboarding".into(),
        ));
    }

    let updated = db::company::update_company(
        &state.pool,
        company_id,
        UpdateCompanyInput {
            onboarding_complete: Some(true),
            ..Default::default()
        },
    )
    .await?
    .ok_or(ApiError::NotFound)?;

    db::workspace_member::ensure_ai_cofounders_in_all_company_workspaces(&state.pool, company_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(updated))
}
