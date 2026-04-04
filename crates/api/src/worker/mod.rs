//! In-process background worker.
//!
//! Spawned as a Tokio task from `main.rs`. Polls the `agent_jobs` queue every
//! few seconds, claims the next pending job (for a company in `Running` state),
//! and dispatches it to the appropriate handler.

pub mod actions;
pub mod agent_runner;
pub mod context;

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
            match tick(&pool, &registry, &events_tx).await {
                Ok(true) => {} // processed a job, poll immediately
                Ok(false) => {
                    // nothing to do; sleep before next poll
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

/// One worker tick: claim and execute at most one job.
/// Returns `true` if a job was processed (so caller can poll again immediately).
async fn tick(
    pool: &PgPool,
    registry: &ProviderRegistry,
    events_tx: &broadcast::Sender<JobEvent>,
) -> anyhow::Result<bool> {
    let Some(job) = db::job::claim_next(pool).await? else {
        return Ok(false);
    };

    info!(
        job_id = %job.id,
        kind   = %job.kind,
        "claimed job"
    );

    let result = match job.kind {
        JobKind::AgentTicketRun => {
            agent_runner::run_agent_job(
                pool,
                registry,
                job.id,
                job.company_id,
                &job.payload,
                events_tx,
            )
            .await
        }
        JobKind::IndexRepository => {
            // Phase 9 — not implemented yet.
            warn!(job_id = %job.id, "IndexRepository job not implemented; skipping");
            Ok(())
        }
    };

    match result {
        Ok(()) => {
            db::job::complete_job(pool, job.id).await?;
            info!(job_id = %job.id, "job completed");
        }
        Err(e) => {
            error!(job_id = %job.id, err = %e, "job failed");
            db::job::fail_job(pool, job.id, &e.to_string()).await?;
        }
    }

    Ok(true)
}
