use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use domain::{CreatePersonInput, Person, PersonKind, RoleType, UpdatePersonInput};
use serde::Deserialize;
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

    let input = CreatePersonInput {
        kind,
        display_name: req.display_name.trim().to_string(),
        role_type,
        specialty: req.specialty,
        ai_profile_id: req.ai_profile_id,
    };

    let person = db::person::create_person(&state.pool, company_id, input).await?;
    Ok((StatusCode::CREATED, Json(person)))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePersonRequest {
    pub display_name: Option<String>,
    pub role_type: Option<String>,
    pub specialty: Option<serde_json::Value>,
    pub ai_profile_id: Option<serde_json::Value>,
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

    let input = UpdatePersonInput {
        display_name: req.display_name.map(|s| s.trim().to_string()),
        role_type,
        specialty,
        ai_profile_id,
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
