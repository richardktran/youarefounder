//! Phase 7: Priority-aware agent scheduler.
//!
//! Runs as a separate Tokio task. On each tick it finds all running companies,
//! looks for tickets assigned to ANY AI agent that are not done/cancelled/blocked,
//! and enqueues an `AgentTicketRun` job for any that don't already have one in-flight.
//!
//! Priority tiers (lower = processed first):
//!   10  co_founder   — bridge between founder vision and execution
//!   20  ceo / cto    — executive leadership
//!   50  specialist   — domain execution
//!
//! When a new ticket is created by an agent action, it is immediately enqueued
//! (see `agent_runner.rs`). This scheduler acts as the safety net, ensuring
//! all pending tickets are always in the queue even if immediate enqueue failed.

use std::time::Duration;

use db::job::{PRIORITY_CO_FOUNDER, PRIORITY_EXECUTIVE, PRIORITY_SPECIALIST};
use domain::{JobKind, RoleType};
use serde_json::json;
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

/// How often the scheduler scans for unqueued work.
/// Short interval keeps specialist tickets responsive after managers delegate.
const SCHEDULER_INTERVAL: Duration = Duration::from_secs(10);

/// Spawn the agent scheduler. Call once from `main` after the pool is ready.
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        info!(
            "agent scheduler started (interval = {:?})",
            SCHEDULER_INTERVAL
        );
        loop {
            tokio::time::sleep(SCHEDULER_INTERVAL).await;
            if let Err(e) = tick(&pool).await {
                error!(err = %e, "scheduler tick error");
            }
        }
    });
}

async fn tick(pool: &PgPool) -> anyhow::Result<()> {
    let running_companies: Vec<uuid::Uuid> = {
        use sqlx::Row;
        sqlx::query("SELECT id FROM companies WHERE run_state = 'running'")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| r.get("id"))
            .collect()
    };

    if running_companies.is_empty() {
        debug!("scheduler: no running companies");
        return Ok(());
    }

    for company_id in running_companies {
        if let Err(e) = schedule_company(pool, company_id).await {
            warn!(company_id = %company_id, err = %e, "scheduler: error processing company");
        }
    }

    Ok(())
}

/// Scan one company immediately and enqueue jobs for all unqueued AI agent tickets.
/// Called both by the background scheduler loop and directly when a simulation starts.
pub async fn schedule_company(pool: &PgPool, company_id: uuid::Uuid) -> anyhow::Result<()> {
    use sqlx::Row;

    // Find ALL AI-agent people in this company (executives and specialists).
    // Specialists are now auto-scheduled — they are picked up whenever a ticket
    // is assigned to them, either manually or by an executive agent.
    let agents: Vec<(uuid::Uuid, String)> = sqlx::query(
        "SELECT id, role_type FROM people
         WHERE company_id = $1
           AND kind = 'ai_agent'
           AND ai_profile_id IS NOT NULL",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.get("id"), r.get("role_type")))
    .collect();

    if agents.is_empty() {
        return Ok(());
    }

    for (person_id, role_type_str) in agents {
        let role_type = role_type_str
            .parse::<RoleType>()
            .unwrap_or(RoleType::Specialist);

        let priority = role_priority(&role_type);

        // Find assigned tickets that can be worked on.
        // Skip: done, cancelled, blocked (blocked = waiting for founder).
        let tickets: Vec<uuid::Uuid> = sqlx::query(
            "SELECT t.id FROM tickets t
             WHERE t.assignee_person_id = $1
               AND t.status NOT IN ('done', 'cancelled', 'blocked')
             ORDER BY
               CASE t.priority
                 WHEN 'high'   THEN 1
                 WHEN 'medium' THEN 2
                 ELSE               3
               END,
               t.updated_at ASC",
        )
        .bind(person_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| r.get("id"))
        .collect();

        for ticket_id in tickets {
            // Skip if already queued for this (ticket, person) pair.
            match db::job::has_active_job_for_ticket(pool, ticket_id, person_id).await {
                Ok(true) => {
                    debug!(
                        ticket_id = %ticket_id,
                        person_id = %person_id,
                        "scheduler: skipping — job already in-flight"
                    );
                    continue;
                }
                Ok(false) => {}
                Err(e) => {
                    warn!(err = %e, "scheduler: error checking active job");
                    continue;
                }
            }

            let payload = json!({
                "ticket_id": ticket_id,
                "person_id": person_id,
            });

            match db::job::enqueue(pool, JobKind::AgentTicketRun, company_id, payload, priority)
                .await
            {
                Ok(job) => {
                    info!(
                        job_id = %job.id,
                        ticket_id = %ticket_id,
                        person_id = %person_id,
                        role = %role_type_str,
                        priority = priority,
                        "scheduler: enqueued agent run"
                    );
                }
                Err(e) => {
                    warn!(
                        ticket_id = %ticket_id,
                        person_id = %person_id,
                        err = %e,
                        "scheduler: failed to enqueue job"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Map a person's role to a job priority tier.
pub fn role_priority(role: &RoleType) -> i16 {
    match role {
        RoleType::CoFounder => PRIORITY_CO_FOUNDER,
        RoleType::Ceo | RoleType::Cto | RoleType::Cfo => PRIORITY_EXECUTIVE,
        RoleType::Specialist => PRIORITY_SPECIALIST,
    }
}
