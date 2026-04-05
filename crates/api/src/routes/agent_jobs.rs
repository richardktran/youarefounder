//! Phase 4: agent job queue management.
//! - List recent jobs for a company
//! - Manually enqueue an `AgentTicketRun` for a specific ticket
//! - List agent run history for a ticket
//! - Stream live events for a running job via SSE

use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use domain::{AgentJob, AgentRun, AgentTicketRunPayload, JobKind, RunState};
use futures_util::StreamExt as _;
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

/// `GET /v1/companies/:id/agent-jobs`
pub async fn list_agent_jobs(
    State(state): State<AppState>,
    Path(company_id): Path<Uuid>,
) -> ApiResult<Json<Vec<AgentJob>>> {
    db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let jobs = db::job::list_jobs(&state.pool, company_id, 50).await?;
    Ok(Json(jobs))
}

#[derive(Debug, Deserialize)]
pub struct EnqueueRunInput {
    /// The ID of the person (agent) who should run this ticket.
    pub person_id: Uuid,
}

/// `POST /v1/companies/:company_id/workspaces/:workspace_id/tickets/:ticket_id/run-agent`
///
/// Manually enqueue an `AgentTicketRun` job for a ticket.
pub async fn enqueue_ticket_run(
    State(state): State<AppState>,
    Path((company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<EnqueueRunInput>,
) -> ApiResult<(StatusCode, Json<AgentJob>)> {
    let company = db::company::get_company(&state.pool, company_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if company.run_state == RunState::Stopped {
        db::company::set_run_state(&state.pool, company_id, RunState::Running)
            .await?
            .ok_or(ApiError::NotFound)?;
    } else if company.run_state == RunState::Terminated {
        return Err(ApiError::NotFound);
    }

    let payload = serde_json::to_value(AgentTicketRunPayload {
        ticket_id,
        person_id: input.person_id,
    })
    .map_err(|e| ApiError::Internal(e.into()))?;

    let job = db::job::enqueue(&state.pool, JobKind::AgentTicketRun, company_id, payload)
        .await?;

    Ok((StatusCode::CREATED, Json(job)))
}

/// `GET /v1/companies/:company_id/workspaces/:workspace_id/tickets/:ticket_id/agent-runs`
///
/// List agent run history for a specific ticket.
pub async fn list_ticket_agent_runs(
    State(state): State<AppState>,
    Path((_company_id, _workspace_id, ticket_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Vec<AgentRun>>> {
    let runs = db::agent_run::list_runs_for_ticket(&state.pool, ticket_id).await?;
    Ok(Json(runs))
}

/// `GET /v1/agent-jobs/:job_id/stream`
///
/// Server-Sent Events stream for a specific agent job.
///
/// The client receives a stream of JSON-encoded [`JobEvent`] objects:
/// - `{"type":"started","job_id":"..."}` — LLM call is beginning
/// - `{"type":"token","job_id":"...","token":"..."}` — one streaming token
/// - `{"type":"completed","job_id":"..."}` — job finished successfully
/// - `{"type":"failed","job_id":"...","error":"..."}` — job failed
///
/// The stream stays open until the connection is closed or a terminal event
/// (`completed` / `failed`) is received.
pub async fn stream_job_events(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        let event = match result {
            Ok(job_event) if job_event.job_id() == job_id => {
                let data = serde_json::to_string(&job_event).unwrap_or_default();
                Some(Ok(Event::default().data(data)))
            }
            // Different job or lagged receiver — skip silently.
            _ => None,
        };
        futures_util::future::ready(event)
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
