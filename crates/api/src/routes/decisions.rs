//! Phase 6: DecisionRequest routes — founder inbox for structured escalations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::{AnswerDecisionRequestInput, CreateDecisionRequestInput, DecisionRequest, DecisionStatus};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListDecisionsParams {
    pub status: Option<String>,
}

/// `GET /v1/companies/:id/decision-requests?status=<pending_founder|answered>`
pub async fn list_decisions(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Query(params): Query<ListDecisionsParams>,
) -> ApiResult<Json<Vec<DecisionRequest>>> {
    let status_filter = params
        .status
        .map(|s| {
            s.parse::<DecisionStatus>()
                .map_err(|e| ApiError::BadRequest(e))
        })
        .transpose()?;

    let decisions =
        db::decision::list_decision_requests(&state.pool, company_id, status_filter).await?;
    Ok(Json(decisions))
}

/// `GET /v1/companies/:id/decision-requests/:decision_id`
pub async fn get_decision(
    State(state): State<AppState>,
    Path((company_id, decision_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<DecisionRequest>> {
    let decision =
        db::decision::get_decision_request(&state.pool, company_id, decision_id)
            .await?
            .ok_or(ApiError::NotFound)?;
    Ok(Json(decision))
}

#[derive(Debug, Deserialize)]
pub struct CreateDecisionBody {
    pub ticket_id: Uuid,
    pub raised_by_person_id: Option<Uuid>,
    pub question: String,
    pub context_note: Option<String>,
}

/// `POST /v1/companies/:id/decision-requests`
/// Manually create a decision request (agents use the action; founders can also
/// create them directly if they want to flag something pre-emptively).
pub async fn create_decision(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(req): Json<CreateDecisionBody>,
) -> ApiResult<(StatusCode, Json<DecisionRequest>)> {
    if req.question.trim().is_empty() {
        return Err(ApiError::BadRequest("question is required".into()));
    }

    let input = CreateDecisionRequestInput {
        ticket_id: req.ticket_id,
        raised_by_person_id: req.raised_by_person_id,
        question: req.question,
        context_note: req.context_note,
    };

    let decision = db::decision::create_decision_request(&state.pool, company_id, input)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(decision)))
}

#[derive(Debug, Deserialize)]
pub struct AnswerDecisionBody {
    pub founder_answer: String,
}

/// `POST /v1/companies/:id/decision-requests/:decision_id/answer`
/// Answer a decision request, which unblocks the parent ticket.
pub async fn answer_decision(
    State(state): State<AppState>,
    Path((company_id, decision_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AnswerDecisionBody>,
) -> ApiResult<Json<DecisionRequest>> {
    if req.founder_answer.trim().is_empty() {
        return Err(ApiError::BadRequest("founder_answer is required".into()));
    }

    let input = AnswerDecisionRequestInput {
        founder_answer: req.founder_answer,
    };

    let decision =
        db::decision::answer_decision_request(&state.pool, company_id, decision_id, input)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(decision))
}
