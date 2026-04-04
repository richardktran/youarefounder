use anyhow::Result;
use domain::{AgentJob, JobKind, JobStatus};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_job(row: &PgRow) -> AgentJob {
    let kind_str: String = row.get("kind");
    let status_str: String = row.get("status");
    AgentJob {
        id: row.get("id"),
        kind: kind_str
            .parse::<JobKind>()
            .unwrap_or(JobKind::AgentTicketRun),
        company_id: row.get("company_id"),
        payload: row.get("payload"),
        status: status_str
            .parse::<JobStatus>()
            .unwrap_or(JobStatus::Pending),
        run_at: row.get("run_at"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        error: row.get("error"),
        attempts: row.get("attempts"),
        max_attempts: row.get("max_attempts"),
        created_at: row.get("created_at"),
    }
}

/// Enqueue a new job.
pub async fn enqueue(
    pool: &PgPool,
    kind: JobKind,
    company_id: Uuid,
    payload: Value,
) -> Result<AgentJob> {
    let row = sqlx::query(
        "INSERT INTO agent_jobs (kind, company_id, payload)
         VALUES ($1, $2, $3)
         RETURNING id, kind, company_id, payload, status, run_at,
                   started_at, completed_at, error, attempts, max_attempts, created_at",
    )
    .bind(kind.to_string())
    .bind(company_id)
    .bind(payload)
    .fetch_one(pool)
    .await?;

    Ok(row_to_job(&row))
}

/// Claim the next pending job using SKIP LOCKED to avoid contention.
pub async fn claim_next(pool: &PgPool) -> Result<Option<AgentJob>> {
    let row = sqlx::query(
        "UPDATE agent_jobs
         SET status     = 'running',
             started_at = NOW(),
             attempts   = attempts + 1
         WHERE id = (
             SELECT id FROM agent_jobs
             WHERE status = 'pending'
               AND run_at <= NOW()
             ORDER BY run_at ASC
             LIMIT 1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, kind, company_id, payload, status, run_at,
                   started_at, completed_at, error, attempts, max_attempts, created_at",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_job))
}

pub async fn complete_job(pool: &PgPool, job_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE agent_jobs
         SET status = 'succeeded', completed_at = NOW()
         WHERE id = $1",
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark a job as failed. Re-queues with exponential back-off if retries remain.
pub async fn fail_job(pool: &PgPool, job_id: Uuid, error: &str) -> Result<()> {
    sqlx::query(
        "UPDATE agent_jobs
         SET status       = CASE
                               WHEN attempts >= max_attempts THEN 'failed'
                               ELSE 'pending'
                            END,
             completed_at = CASE
                               WHEN attempts >= max_attempts THEN NOW()
                               ELSE NULL
                            END,
             run_at       = CASE
                               WHEN attempts >= max_attempts THEN run_at
                               ELSE NOW() + (INTERVAL '30 seconds' * attempts)
                            END,
             error        = $2
         WHERE id = $1",
    )
    .bind(job_id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}
