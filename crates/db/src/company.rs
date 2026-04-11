use anyhow::Result;
use domain::{BootstrapStatus, Company, CreateCompanyInput, RunState, UpdateCompanyInput};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::workspace;

fn row_to_company(row: &PgRow) -> Company {
    let run_state_str: String = row.get("run_state");
    Company {
        id: row.get("id"),
        name: row.get("name"),
        slug: row.get("slug"),
        onboarding_complete: row.get("onboarding_complete"),
        run_state: run_state_str
            .parse::<RunState>()
            .unwrap_or(RunState::Stopped),
        max_concurrent_agents: row.get("max_concurrent_agents"),
        agent_ticket_memory: row.get("agent_ticket_memory"),
        agent_decision_memory: row.get("agent_decision_memory"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// Return the bootstrap status: does a company exist and is onboarding done?
/// Phase 1 assumes one primary company per install.
pub async fn get_bootstrap_status(pool: &PgPool) -> Result<BootstrapStatus> {
    let row = sqlx::query(
        "SELECT id, onboarding_complete FROM companies
         WHERE run_state != 'terminated'
         ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(BootstrapStatus {
            onboarding_complete: false,
            company_id: None,
        }),
        Some(r) => Ok(BootstrapStatus {
            onboarding_complete: r.get("onboarding_complete"),
            company_id: Some(r.get("id")),
        }),
    }
}

pub async fn list_companies(pool: &PgPool) -> Result<Vec<Company>> {
    let rows = sqlx::query(
        "SELECT id, name, slug, onboarding_complete, run_state, max_concurrent_agents,
                agent_ticket_memory, agent_decision_memory, created_at, updated_at
         FROM companies
         WHERE run_state != 'terminated'
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_company).collect())
}

pub async fn get_company(pool: &PgPool, company_id: Uuid) -> Result<Option<Company>> {
    let row = sqlx::query(
        "SELECT id, name, slug, onboarding_complete, run_state, max_concurrent_agents,
                agent_ticket_memory, agent_decision_memory, created_at, updated_at
         FROM companies
         WHERE id = $1",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_company))
}

/// Create a company (and optionally an inline first product) in one transaction.
pub async fn create_company(pool: &PgPool, input: CreateCompanyInput) -> Result<Company> {
    let slug = {
        let base = slug::slugify(&input.name);
        // Append a short random suffix to avoid slug collisions.
        let suffix = &Uuid::new_v4().to_string()[..8];
        format!("{base}-{suffix}")
    };

    let mut tx = pool.begin().await?;

    let company_row = sqlx::query(
        "INSERT INTO companies (name, slug)
         VALUES ($1, $2)
         RETURNING id, name, slug, onboarding_complete, run_state, max_concurrent_agents,
                   agent_ticket_memory, agent_decision_memory, created_at, updated_at",
    )
    .bind(&input.name)
    .bind(&slug)
    .fetch_one(&mut *tx)
    .await?;

    let company = row_to_company(&company_row);

    if let Some(product) = input.product {
        sqlx::query(
            "INSERT INTO products (company_id, name, description)
             VALUES ($1, $2, $3)",
        )
        .bind(company.id)
        .bind(&product.name)
        .bind(&product.description)
        .execute(&mut *tx)
        .await?;
    }

    workspace::seed_default_workspaces(&mut tx, company.id).await?;

    tx.commit().await?;
    Ok(company)
}

pub async fn update_company(
    pool: &PgPool,
    company_id: Uuid,
    input: UpdateCompanyInput,
) -> Result<Option<Company>> {
    let run_state_str = input.run_state.as_ref().map(|s| s.to_string());
    let row = sqlx::query(
        "UPDATE companies
         SET
             name                   = COALESCE($2, name),
             onboarding_complete    = COALESCE($3, onboarding_complete),
             run_state              = COALESCE($4, run_state),
             max_concurrent_agents  = COALESCE($5, max_concurrent_agents),
             agent_ticket_memory    = COALESCE($6, agent_ticket_memory),
             agent_decision_memory  = COALESCE($7, agent_decision_memory),
             updated_at             = NOW()
         WHERE id = $1
         RETURNING id, name, slug, onboarding_complete, run_state, max_concurrent_agents,
                   agent_ticket_memory, agent_decision_memory, created_at, updated_at",
    )
    .bind(company_id)
    .bind(&input.name)
    .bind(input.onboarding_complete)
    .bind(run_state_str)
    .bind(input.max_concurrent_agents)
    .bind(&input.agent_ticket_memory)
    .bind(&input.agent_decision_memory)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_company))
}

/// Set the simulation run state (stopped/running).
pub async fn set_run_state(
    pool: &PgPool,
    company_id: Uuid,
    state: RunState,
) -> Result<Option<Company>> {
    update_company(
        pool,
        company_id,
        UpdateCompanyInput {
            run_state: Some(state),
            ..Default::default()
        },
    )
    .await
}

/// Permanently delete a company and all cascade-deleted data.
/// This is the Terminate action — irreversible.
pub async fn terminate_company(pool: &PgPool, company_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM companies WHERE id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Delete every company row (and all cascade-deleted data). Returns rows removed.
pub async fn delete_all_companies(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM companies").execute(pool).await?;
    Ok(result.rows_affected())
}
