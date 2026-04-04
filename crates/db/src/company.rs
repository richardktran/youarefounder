use anyhow::Result;
use domain::{BootstrapStatus, Company, CreateCompanyInput, UpdateCompanyInput};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_company(row: &PgRow) -> Company {
    Company {
        id: row.get("id"),
        name: row.get("name"),
        slug: row.get("slug"),
        onboarding_complete: row.get("onboarding_complete"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// Return the bootstrap status: does a company exist and is onboarding done?
/// Phase 1 assumes one primary company per install.
pub async fn get_bootstrap_status(pool: &PgPool) -> Result<BootstrapStatus> {
    let row = sqlx::query(
        "SELECT id, onboarding_complete FROM companies ORDER BY created_at ASC LIMIT 1",
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
        "SELECT id, name, slug, onboarding_complete, created_at, updated_at
         FROM companies
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_company).collect())
}

pub async fn get_company(pool: &PgPool, company_id: Uuid) -> Result<Option<Company>> {
    let row = sqlx::query(
        "SELECT id, name, slug, onboarding_complete, created_at, updated_at
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
         RETURNING id, name, slug, onboarding_complete, created_at, updated_at",
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

    tx.commit().await?;
    Ok(company)
}

pub async fn update_company(
    pool: &PgPool,
    company_id: Uuid,
    input: UpdateCompanyInput,
) -> Result<Option<Company>> {
    let row = sqlx::query(
        "UPDATE companies
         SET
             name                = COALESCE($2, name),
             onboarding_complete = COALESCE($3, onboarding_complete),
             updated_at          = NOW()
         WHERE id = $1
         RETURNING id, name, slug, onboarding_complete, created_at, updated_at",
    )
    .bind(company_id)
    .bind(&input.name)
    .bind(input.onboarding_complete)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_company))
}
