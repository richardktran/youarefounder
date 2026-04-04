use anyhow::Result;
use domain::{CreateProductInput, Product, ProductStatus, UpdateProductInput};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_product(row: &PgRow) -> Product {
    let status_str: String = row.get("status");
    Product {
        id: row.get("id"),
        company_id: row.get("company_id"),
        name: row.get("name"),
        description: row.get("description"),
        status: status_str.parse::<ProductStatus>().unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_products(pool: &PgPool, company_id: Uuid) -> Result<Vec<Product>> {
    let rows = sqlx::query(
        "SELECT id, company_id, name, description, status, created_at, updated_at
         FROM products
         WHERE company_id = $1
         ORDER BY created_at ASC",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_product).collect())
}

pub async fn get_product(pool: &PgPool, product_id: Uuid) -> Result<Option<Product>> {
    let row = sqlx::query(
        "SELECT id, company_id, name, description, status, created_at, updated_at
         FROM products
         WHERE id = $1",
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_product))
}

pub async fn create_product(
    pool: &PgPool,
    company_id: Uuid,
    input: CreateProductInput,
) -> Result<Product> {
    let row = sqlx::query(
        "INSERT INTO products (company_id, name, description)
         VALUES ($1, $2, $3)
         RETURNING id, company_id, name, description, status, created_at, updated_at",
    )
    .bind(company_id)
    .bind(&input.name)
    .bind(&input.description)
    .fetch_one(pool)
    .await?;

    Ok(row_to_product(&row))
}

pub async fn update_product(
    pool: &PgPool,
    product_id: Uuid,
    input: UpdateProductInput,
) -> Result<Option<Product>> {
    let status_str = input.status.as_ref().map(|s| s.to_string());

    let row = sqlx::query(
        "UPDATE products
         SET
             name        = COALESCE($2, name),
             description = COALESCE($3, description),
             status      = COALESCE($4, status),
             updated_at  = NOW()
         WHERE id = $1
         RETURNING id, company_id, name, description, status, created_at, updated_at",
    )
    .bind(product_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&status_str)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_product))
}
