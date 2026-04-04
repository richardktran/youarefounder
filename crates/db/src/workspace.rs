use anyhow::Result;
use domain::{CreateWorkspaceInput, UpdateWorkspaceInput, Workspace};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_workspace(row: &PgRow) -> Workspace {
    Workspace {
        id: row.get("id"),
        company_id: row.get("company_id"),
        name: row.get("name"),
        slug: row.get("slug"),
        description: row.get("description"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_workspaces(pool: &PgPool, company_id: Uuid) -> Result<Vec<Workspace>> {
    let rows = sqlx::query(
        "SELECT id, company_id, name, slug, description, created_at, updated_at
         FROM workspaces
         WHERE company_id = $1
         ORDER BY created_at ASC",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_workspace).collect())
}

pub async fn get_workspace(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<Option<Workspace>> {
    let row = sqlx::query(
        "SELECT id, company_id, name, slug, description, created_at, updated_at
         FROM workspaces
         WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_workspace))
}

pub async fn create_workspace(
    pool: &PgPool,
    company_id: Uuid,
    input: CreateWorkspaceInput,
) -> Result<Workspace> {
    let slug = input
        .slug
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slug::slugify(&input.name));

    let row = sqlx::query(
        "INSERT INTO workspaces (company_id, name, slug, description)
         VALUES ($1, $2, $3, $4)
         RETURNING id, company_id, name, slug, description, created_at, updated_at",
    )
    .bind(company_id)
    .bind(&input.name)
    .bind(&slug)
    .bind(&input.description)
    .fetch_one(pool)
    .await?;

    Ok(row_to_workspace(&row))
}

pub async fn update_workspace(
    pool: &PgPool,
    workspace_id: Uuid,
    input: UpdateWorkspaceInput,
) -> Result<Option<Workspace>> {
    let row = sqlx::query(
        "UPDATE workspaces
         SET
             name        = COALESCE($2, name),
             description = COALESCE($3, description),
             updated_at  = NOW()
         WHERE id = $1
         RETURNING id, company_id, name, slug, description, created_at, updated_at",
    )
    .bind(workspace_id)
    .bind(&input.name)
    .bind(&input.description)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_workspace))
}

pub async fn delete_workspace(pool: &PgPool, workspace_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Seed the five default workspaces inside an open transaction.
pub async fn seed_default_workspaces(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    company_id: Uuid,
) -> Result<()> {
    let defaults = [
        ("Discovery", "discovery", "Market exploration, interviews, and assumptions"),
        ("Product", "product", "PRDs, user journeys, and requirements"),
        ("R&D", "rnd", "Technical spikes, architecture, and implementation tasks"),
        ("Go-to-market", "gtm", "Positioning, launch checklist, and growth"),
        ("Finance", "finance", "Cost assumptions and pricing experiments"),
    ];

    for (name, slug_val, description) in &defaults {
        sqlx::query(
            "INSERT INTO workspaces (company_id, name, slug, description)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (company_id, slug) DO NOTHING",
        )
        .bind(company_id)
        .bind(name)
        .bind(slug_val)
        .bind(description)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}
