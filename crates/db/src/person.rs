use anyhow::Result;
use domain::{CreatePersonInput, Person, PersonKind, RoleType};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_person(row: &PgRow) -> Person {
    let kind_str: String = row.get("kind");
    let role_str: String = row.get("role_type");
    Person {
        id: row.get("id"),
        company_id: row.get("company_id"),
        kind: kind_str
            .parse::<PersonKind>()
            .unwrap_or(PersonKind::AiAgent),
        display_name: row.get("display_name"),
        role_type: role_str
            .parse::<RoleType>()
            .unwrap_or(RoleType::Specialist),
        specialty: row.get("specialty"),
        ai_profile_id: row.get("ai_profile_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_people(pool: &PgPool, company_id: Uuid) -> Result<Vec<Person>> {
    let rows = sqlx::query(
        "SELECT id, company_id, kind, display_name, role_type, specialty,
                ai_profile_id, created_at, updated_at
         FROM people
         WHERE company_id = $1
         ORDER BY created_at ASC",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_person).collect())
}

pub async fn get_person(
    pool: &PgPool,
    company_id: Uuid,
    person_id: Uuid,
) -> Result<Option<Person>> {
    let row = sqlx::query(
        "SELECT id, company_id, kind, display_name, role_type, specialty,
                ai_profile_id, created_at, updated_at
         FROM people
         WHERE id = $1 AND company_id = $2",
    )
    .bind(person_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_person))
}

pub async fn create_person(
    pool: &PgPool,
    company_id: Uuid,
    input: CreatePersonInput,
) -> Result<Person> {
    let row = sqlx::query(
        "INSERT INTO people (company_id, kind, display_name, role_type, specialty, ai_profile_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, company_id, kind, display_name, role_type, specialty,
                   ai_profile_id, created_at, updated_at",
    )
    .bind(company_id)
    .bind(input.kind.to_string())
    .bind(&input.display_name)
    .bind(input.role_type.to_string())
    .bind(&input.specialty)
    .bind(input.ai_profile_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_person(&row))
}

/// Seed the human founder row. Idempotent — skips if already exists.
pub async fn seed_founder(
    pool: &PgPool,
    company_id: Uuid,
    display_name: &str,
) -> Result<Person> {
    // Try to insert; if the founder already exists (by company + kind), do nothing.
    sqlx::query(
        "INSERT INTO people (company_id, kind, display_name, role_type)
         VALUES ($1, 'human_founder', $2, 'co_founder')
         ON CONFLICT DO NOTHING",
    )
    .bind(company_id)
    .bind(display_name)
    .execute(pool)
    .await?;

    // Fetch (always succeeds after upsert above).
    let row = sqlx::query(
        "SELECT id, company_id, kind, display_name, role_type, specialty,
                ai_profile_id, created_at, updated_at
         FROM people
         WHERE company_id = $1 AND kind = 'human_founder'
         LIMIT 1",
    )
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_person(&row))
}
