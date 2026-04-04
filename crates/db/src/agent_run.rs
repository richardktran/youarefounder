use anyhow::Result;
use domain::AgentRun;
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_run(row: &PgRow) -> AgentRun {
    AgentRun {
        id: row.get("id"),
        agent_job_id: row.get("agent_job_id"),
        ticket_id: row.get("ticket_id"),
        person_id: row.get("person_id"),
        prompt_tokens: row.get("prompt_tokens"),
        completion_tokens: row.get("completion_tokens"),
        raw_response: row.get("raw_response"),
        actions_applied: row.get("actions_applied"),
        error: row.get("error"),
        created_at: row.get("created_at"),
    }
}

pub async fn list_runs_for_ticket(
    pool: &PgPool,
    ticket_id: Uuid,
) -> Result<Vec<AgentRun>> {
    let rows = sqlx::query(
        "SELECT id, agent_job_id, ticket_id, person_id, prompt_tokens, completion_tokens,
                raw_response, actions_applied, error, created_at
         FROM agent_run_history
         WHERE ticket_id = $1
         ORDER BY created_at DESC",
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_run).collect())
}

pub async fn record_run(
    pool: &PgPool,
    agent_job_id: Uuid,
    ticket_id: Uuid,
    person_id: Uuid,
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
    raw_response: Option<&str>,
    actions_applied: &Value,
    error: Option<&str>,
) -> Result<AgentRun> {
    let row = sqlx::query(
        "INSERT INTO agent_run_history
             (agent_job_id, ticket_id, person_id, prompt_tokens, completion_tokens,
              raw_response, actions_applied, error)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, agent_job_id, ticket_id, person_id, prompt_tokens, completion_tokens,
                   raw_response, actions_applied, error, created_at",
    )
    .bind(agent_job_id)
    .bind(ticket_id)
    .bind(person_id)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(raw_response)
    .bind(actions_applied)
    .bind(error)
    .fetch_one(pool)
    .await?;

    Ok(row_to_run(&row))
}
