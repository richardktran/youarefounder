use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{CreatePersonInput, Person, PersonKind, RoleType, UpdatePersonInput};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/people`
pub async fn list_people(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Person>>> {
    let people = db::person::list_people(&state.pool, company_id).await?;
    Ok(Json(people))
}

/// `GET /v1/companies/:id/people/:person_id`
pub async fn get_person(
    State(state): State<AppState>,
    Path((company_id, person_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Person>> {
    let person = db::person::get_person(&state.pool, company_id, person_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(person))
}

/// Wire type for creating a person via the API.
/// Uses string enums so the JSON is idiomatic (snake_case).
#[derive(Debug, Deserialize)]
pub struct CreatePersonRequest {
    pub kind: String,
    pub display_name: String,
    pub role_type: String,
    pub specialty: Option<String>,
    pub ai_profile_id: Option<Uuid>,
}

/// `POST /v1/companies/:id/people`
pub async fn create_person(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
    Json(req): Json<CreatePersonRequest>,
) -> ApiResult<(StatusCode, Json<Person>)> {
    if req.display_name.trim().is_empty() {
        return Err(ApiError::BadRequest("display_name is required".into()));
    }

    let kind = req
        .kind
        .parse::<PersonKind>()
        .map_err(|e| ApiError::BadRequest(e))?;

    let role_type = req
        .role_type
        .parse::<RoleType>()
        .map_err(|e| ApiError::BadRequest(e))?;

    let mut ai_profile_id = req.ai_profile_id;
    if matches!(kind, PersonKind::AiAgent) && ai_profile_id.is_none() {
        ai_profile_id =
            db::person::ai_profile_id_of_ai_co_founder(&state.pool, company_id).await?;
    }
    if matches!(kind, PersonKind::AiAgent) && ai_profile_id.is_none() {
        return Err(ApiError::BadRequest(
            "AI agents need an AI profile — link one in the form or add an AI co-founder with a profile first."
                .into(),
        ));
    }

    let input = CreatePersonInput {
        kind,
        display_name: req.display_name.trim().to_string(),
        role_type,
        specialty: req.specialty,
        ai_profile_id,
    };

    let person = db::person::create_person(&state.pool, company_id, input).await?;

    if matches!(person.kind, PersonKind::AiAgent) && person.role_type == RoleType::CoFounder {
        db::workspace_member::ensure_ai_cofounders_in_all_company_workspaces(&state.pool, company_id)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    }

    Ok((StatusCode::CREATED, Json(person)))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePersonRequest {
    pub display_name: Option<String>,
    pub role_type: Option<String>,
    pub specialty: Option<serde_json::Value>,
    pub ai_profile_id: Option<serde_json::Value>,
    pub reports_to_person_id: Option<serde_json::Value>,
}

/// `PATCH /v1/companies/:id/people/:person_id`
pub async fn update_person(
    State(state): State<AppState>,
    Path((company_id, person_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePersonRequest>,
) -> ApiResult<Json<Person>> {
    let role_type = req
        .role_type
        .map(|s| s.parse::<RoleType>().map_err(|e| ApiError::BadRequest(e)))
        .transpose()?;

    // specialty: null JSON value means clear; absent key means don't change.
    let specialty = req.specialty.map(|v| match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        _ => None,
    });

    let ai_profile_id = req.ai_profile_id.map(|v| match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => s.parse::<Uuid>().ok(),
        _ => None,
    });

    let reports_to_person_id = req.reports_to_person_id.map(|v| match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => s.parse::<Uuid>().ok(),
        _ => None,
    });

    let input = UpdatePersonInput {
        display_name: req.display_name.map(|s| s.trim().to_string()),
        role_type,
        specialty,
        ai_profile_id,
        reports_to_person_id,
    };

    let person = db::person::update_person(&state.pool, company_id, person_id, input)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(person))
}

/// `DELETE /v1/companies/:id/people/:person_id`
pub async fn delete_person(
    State(state): State<AppState>,
    Path((company_id, person_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = db::person::delete_person(&state.pool, company_id, person_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// Flat node in the org chart response — same as Person but no extra allocation.
#[derive(Debug, Serialize)]
pub struct OrgNode {
    pub id: Uuid,
    pub display_name: String,
    pub role_type: String,
    pub specialty: Option<String>,
    pub kind: String,
    pub reports_to_person_id: Option<Uuid>,
}

/// `GET /v1/companies/:id/org-chart`
///
/// Returns all people for the company as a flat list; clients build the tree.
/// Each node includes `reports_to_person_id` so the client can reconstruct hierarchy.
pub async fn get_org_chart(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrgNode>>> {
    let people = db::person::list_people(&state.pool, company_id).await?;
    let nodes = people
        .into_iter()
        .map(|p| OrgNode {
            id: p.id,
            display_name: p.display_name,
            role_type: p.role_type.to_string(),
            specialty: p.specialty,
            kind: p.kind.to_string(),
            reports_to_person_id: p.reports_to_person_id,
        })
        .collect();
    Ok(Json(nodes))
}

#[derive(Debug, Deserialize)]
pub struct UpdateReportingLineRequest {
    /// UUID of the new manager, or `null` to clear (make this person a root).
    pub reports_to_person_id: Option<Uuid>,
}

/// `PATCH /v1/companies/:id/people/:person_id/reporting-line`
///
/// Set or clear the `reports_to_person_id` for a person.
/// Returns 400 if the update would create a cycle.
pub async fn update_reporting_line(
    State(state): State<AppState>,
    Path((company_id, person_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateReportingLineRequest>,
) -> ApiResult<Json<Person>> {
    let person = db::person::update_reporting_line(
        &state.pool,
        company_id,
        person_id,
        req.reports_to_person_id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(person))
}
