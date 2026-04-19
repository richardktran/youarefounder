use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::{AcceptProposalInput, CreateProposalInput, DeclineProposalInput, HiringProposal};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/hiring-proposals?status=<pending_founder|accepted|declined|withdrawn>`
pub async fn list_proposals(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Query(params): Query<ListProposalsParams>,
) -> ApiResult<Json<Vec<HiringProposal>>> {
    let status_filter = params
        .status
        .map(|s| {
            s.parse::<domain::ProposalStatus>()
                .map_err(|e| ApiError::BadRequest(e))
        })
        .transpose()?;

    let proposals =
        db::hiring::list_proposals(&state.pool, company_id, status_filter).await?;
    Ok(Json(proposals))
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsParams {
    pub status: Option<String>,
}

/// `GET /v1/companies/:id/hiring-proposals/:proposal_id`
pub async fn get_proposal(
    State(state): State<AppState>,
    Path((company_id, proposal_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<HiringProposal>> {
    let proposal = db::hiring::get_proposal(&state.pool, company_id, proposal_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(proposal))
}

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub employee_display_name: String,
    pub role_type: String,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
    pub rationale: Option<String>,
    pub scope_of_work: Option<String>,
    pub proposed_by_person_id: Option<Uuid>,
    pub workspace_ids: Option<Vec<Uuid>>,
}

/// `POST /v1/companies/:id/hiring-proposals`
pub async fn create_proposal(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(req): Json<CreateProposalRequest>,
) -> ApiResult<(StatusCode, Json<HiringProposal>)> {
    if req.employee_display_name.trim().is_empty() {
        return Err(ApiError::BadRequest("employee_display_name is required".into()));
    }

    let input = CreateProposalInput {
        proposed_by_person_id: req.proposed_by_person_id,
        employee_display_name: req.employee_display_name.trim().to_string(),
        role_type: req.role_type,
        specialty: req.specialty,
        ai_profile_id: req.ai_profile_id,
        rationale: req.rationale,
        scope_of_work: req.scope_of_work,
        workspace_ids: req.workspace_ids,
    };

    let proposal = db::hiring::create_proposal_auto_accept(&state.pool, company_id, input)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(proposal)))
}

#[derive(Debug, Deserialize, Default)]
pub struct AcceptProposalRequest {
    pub founder_response_text: Option<String>,
}

/// `POST /v1/companies/:id/hiring-proposals/:proposal_id/accept`
pub async fn accept_proposal(
    State(state): State<AppState>,
    Path((company_id, proposal_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<AcceptProposalRequest>>,
) -> ApiResult<Json<HiringProposal>> {
    let req = body.map(|b| b.0).unwrap_or_default();

    let input = AcceptProposalInput {
        founder_response_text: req.founder_response_text,
    };

    let proposal = db::hiring::accept_proposal(&state.pool, company_id, proposal_id, input)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(proposal))
}

#[derive(Debug, Deserialize)]
pub struct DeclineProposalRequest {
    pub founder_response_text: String,
}

/// `POST /v1/companies/:id/hiring-proposals/:proposal_id/decline`
pub async fn decline_proposal(
    State(state): State<AppState>,
    Path((company_id, proposal_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<DeclineProposalRequest>,
) -> ApiResult<Json<HiringProposal>> {
    if req.founder_response_text.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "founder_response_text (reason) is required".into(),
        ));
    }

    let input = DeclineProposalInput {
        founder_response_text: req.founder_response_text,
    };

    let proposal = db::hiring::decline_proposal(&state.pool, company_id, proposal_id, input)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(proposal))
}

/// `DELETE /v1/companies/:id/hiring-proposals/:proposal_id`
/// Removes a proposal that is still pending founder review.
pub async fn delete_proposal(
    State(state): State<AppState>,
    Path((company_id, proposal_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = db::hiring::delete_proposal(&state.pool, company_id, proposal_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
