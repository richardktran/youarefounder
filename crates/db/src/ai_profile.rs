use anyhow::Result;
use domain::{AiProfile, CreateAiProfileInput, UpdateAiProfileInput};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_ai_profile(row: &PgRow) -> AiProfile {
    AiProfile {
        id: row.get("id"),
        company_id: row.get("company_id"),
        display_name: row.get("display_name"),
        provider_kind: row.get("provider_kind"),
        model_id: row.get("model_id"),
        provider_config: row.get::<serde_json::Value, _>("provider_config"),
        default_temperature: row.get("default_temperature"),
        default_max_tokens: row.get("default_max_tokens"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_ai_profiles(pool: &PgPool, company_id: Uuid) -> Result<Vec<AiProfile>> {
    let rows = sqlx::query(
        "SELECT id, company_id, display_name, provider_kind, model_id, provider_config,
                default_temperature, default_max_tokens, created_at, updated_at
         FROM ai_profiles
         WHERE company_id = $1
         ORDER BY created_at ASC",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_ai_profile).collect())
}

pub async fn get_ai_profile(
    pool: &PgPool,
    company_id: Uuid,
    profile_id: Uuid,
) -> Result<Option<AiProfile>> {
    let row = sqlx::query(
        "SELECT id, company_id, display_name, provider_kind, model_id, provider_config,
                default_temperature, default_max_tokens, created_at, updated_at
         FROM ai_profiles
         WHERE id = $1 AND company_id = $2",
    )
    .bind(profile_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_ai_profile))
}

pub async fn create_ai_profile(
    pool: &PgPool,
    company_id: Uuid,
    input: CreateAiProfileInput,
) -> Result<AiProfile> {
    let config = input
        .provider_config
        .unwrap_or_else(|| serde_json::json!({ "schema_version": 1 }));

    let row = sqlx::query(
        "INSERT INTO ai_profiles
             (company_id, display_name, provider_kind, model_id, provider_config,
              default_temperature, default_max_tokens)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, company_id, display_name, provider_kind, model_id, provider_config,
                   default_temperature, default_max_tokens, created_at, updated_at",
    )
    .bind(company_id)
    .bind(&input.display_name)
    .bind(&input.provider_kind)
    .bind(&input.model_id)
    .bind(&config)
    .bind(input.default_temperature)
    .bind(input.default_max_tokens)
    .fetch_one(pool)
    .await?;

    Ok(row_to_ai_profile(&row))
}

pub async fn update_ai_profile(
    pool: &PgPool,
    company_id: Uuid,
    profile_id: Uuid,
    input: UpdateAiProfileInput,
) -> Result<Option<AiProfile>> {
    let row = sqlx::query(
        "UPDATE ai_profiles
         SET
             display_name        = COALESCE($3, display_name),
             model_id            = COALESCE($4, model_id),
             provider_config     = COALESCE($5, provider_config),
             default_temperature = COALESCE($6, default_temperature),
             default_max_tokens  = COALESCE($7, default_max_tokens),
             updated_at          = NOW()
         WHERE id = $1 AND company_id = $2
         RETURNING id, company_id, display_name, provider_kind, model_id, provider_config,
                   default_temperature, default_max_tokens, created_at, updated_at",
    )
    .bind(profile_id)
    .bind(company_id)
    .bind(&input.display_name)
    .bind(&input.model_id)
    .bind(&input.provider_config)
    .bind(input.default_temperature)
    .bind(input.default_max_tokens)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_ai_profile))
}
