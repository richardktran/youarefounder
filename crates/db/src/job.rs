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
        priority: row.try_get("priority").unwrap_or(50),
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
///
/// `priority` controls processing order: lower numbers are claimed first.
/// Use `JOB_PRIORITY_*` constants or pass 50 for the default.
pub async fn enqueue(
    pool: &PgPool,
    kind: JobKind,
    company_id: Uuid,
    payload: Value,
    priority: i16,
) -> Result<AgentJob> {
    let row = sqlx::query(
        "INSERT INTO agent_jobs (kind, company_id, payload, priority)
         VALUES ($1, $2, $3, $4)
         RETURNING id, kind, company_id, payload, status, priority, run_at,
                   started_at, completed_at, error, attempts, max_attempts, created_at",
    )
    .bind(kind.to_string())
    .bind(company_id)
    .bind(payload)
    .bind(priority)
    .fetch_one(pool)
    .await?;

    Ok(row_to_job(&row))
}

/// Priority tiers for agent jobs. Lower = processed first.
pub const PRIORITY_CO_FOUNDER: i16 = 10;
pub const PRIORITY_EXECUTIVE: i16 = 20;
pub const PRIORITY_SPECIALIST: i16 = 50;

/// Reset jobs stuck in `running` to `pending`.
///
/// When the API process restarts, any in-flight job from the previous process
/// is orphaned — it still counts as `running`, which blocks `claim_next`'s
/// concurrency check and leaves the queue dead. Call once at startup.
pub async fn requeue_orphaned_running_jobs(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        "UPDATE agent_jobs
         SET status     = 'pending',
             started_at = NULL,
             run_at     = NOW(),
             error      = NULL
         WHERE status = 'running'",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Claim the next pending job using SKIP LOCKED to avoid contention.
///
/// Only claims a job when ALL of these hold for the job's company:
/// - `run_state = 'running'`
/// - The number of currently-running jobs is below `max_concurrent_agents`
///
/// This makes concurrency limits dynamic: changing `max_concurrent_agents` on
/// the company row takes effect on the next poll cycle without a server restart.
pub async fn claim_next(pool: &PgPool) -> Result<Option<AgentJob>> {
    let row = sqlx::query(
        "UPDATE agent_jobs
         SET status     = 'running',
             started_at = NOW(),
             attempts   = attempts + 1
         WHERE id = (
             SELECT aj.id
             FROM agent_jobs aj
             JOIN companies c ON c.id = aj.company_id
             WHERE aj.status = 'pending'
               AND aj.run_at <= NOW()
               AND c.run_state = 'running'
               AND (
                   SELECT COUNT(*)
                   FROM agent_jobs rj
                   WHERE rj.company_id = aj.company_id
                     AND rj.status = 'running'
               ) < GREATEST(1, c.max_concurrent_agents)
             ORDER BY aj.priority ASC, aj.run_at ASC
             LIMIT 1
             FOR UPDATE OF aj SKIP LOCKED
         )
         RETURNING id, kind, company_id, payload, status, priority, run_at,
                   started_at, completed_at, error, attempts, max_attempts, created_at",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_job))
}

/// Check if there is already a pending or running `agent_ticket_run` job
/// for the given (ticket, person) pair. Used by the scheduler to avoid
/// double-enqueuing a ticket that is already in-flight.
pub async fn has_active_job_for_ticket(
    pool: &PgPool,
    ticket_id: Uuid,
    person_id: Uuid,
) -> Result<bool> {
    // The payload is JSONB with keys `ticket_id` and `person_id`.
    let row = sqlx::query(
        "SELECT 1 FROM agent_jobs
         WHERE kind = 'agent_ticket_run'
           AND status IN ('pending', 'running')
           AND payload->>'ticket_id' = $1::text
           AND payload->>'person_id' = $2::text
         LIMIT 1",
    )
    .bind(ticket_id)
    .bind(person_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

/// List recent agent jobs for a company (newest first).
pub async fn list_jobs(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<AgentJob>> {
    let rows = sqlx::query(
        "SELECT id, kind, company_id, payload, status, priority, run_at,
                started_at, completed_at, error, attempts, max_attempts, created_at
         FROM agent_jobs
         WHERE company_id = $1
         ORDER BY created_at DESC
         LIMIT $2",
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_job).collect())
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
