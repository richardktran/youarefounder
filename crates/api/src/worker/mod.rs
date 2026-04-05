//! In-process background worker.
//!
//! Spawned as a Tokio task from `main.rs`. Polls the `agent_jobs` queue every
//! few seconds, claims the next pending job (for a company in `Running` state),
//! and dispatches it to the appropriate handler.
//!
//! Concurrency is enforced by the `claim_next` SQL query, which compares the
//! number of already-running jobs against the company's `max_concurrent_agents`
//! setting. Changing that column takes effect on the next poll cycle with no
//! server restart needed. Each claimed job is run in its own Tokio task so
//! multiple jobs can proceed in parallel.

pub mod actions;
pub mod agent_runner;
pub mod context;
pub mod scheduler;

use std::time::Duration;

use ai_providers::ProviderRegistry;
use domain::JobKind;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::job_events::JobEvent;

const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the worker loop. Call once from `main` after the pool is ready.
pub fn spawn(
    pool: PgPool,
    registry: ProviderRegistry,
    events_tx: broadcast::Sender<JobEvent>,
) {
    tokio::spawn(async move {
        info!("worker started (poll interval = {:?})", POLL_INTERVAL);
        loop {
            match try_claim_and_spawn(&pool, &registry, &events_tx).await {
                Ok(true) => {
                    // A job was claimed and spawned — poll immediately so we
                    // can claim another if concurrency allows.
                }
                Ok(false) => {
                    // Either no pending jobs or concurrency limit reached.
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
                Err(e) => {
                    error!(err = %e, "worker tick error");
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }
        }
    });
}

/// Try to claim one job. If successful, spawn a task to run it and return
/// `true` so the caller can immediately try to claim another.
async fn try_claim_and_spawn(
    pool: &PgPool,
    registry: &ProviderRegistry,
    events_tx: &broadcast::Sender<JobEvent>,
) -> anyhow::Result<bool> {
    let Some(job) = db::job::claim_next(pool).await? else {
        return Ok(false);
    };

    info!(job_id = %job.id, kind = %job.kind, "claimed job");

    let pool = pool.clone();
    let registry = registry.clone();
    let events_tx = events_tx.clone();

    tokio::spawn(async move {
        let result = match job.kind {
            JobKind::AgentTicketRun => {
                agent_runner::run_agent_job(
                    &pool,
                    &registry,
                    job.id,
                    job.company_id,
                    &job.payload,
                    &events_tx,
                )
                .await
            }
            JobKind::IndexRepository => {
                warn!(job_id = %job.id, "IndexRepository job not implemented; skipping");
                Ok(())
            }
        };

        match result {
            Ok(()) => {
                if let Err(e) = db::job::complete_job(&pool, job.id).await {
                    error!(job_id = %job.id, err = %e, "failed to mark job completed");
                } else {
                    info!(job_id = %job.id, "job completed");
                }
            }
            Err(e) => {
                error!(job_id = %job.id, err = %e, "job failed");
                if let Err(db_err) = db::job::fail_job(&pool, job.id, &e.to_string()).await {
                    error!(job_id = %job.id, err = %db_err, "failed to record job failure");
                }
            }
        }
    });

    Ok(true)
}
