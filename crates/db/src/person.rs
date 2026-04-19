use anyhow::{anyhow, Result};
use domain::{CreatePersonInput, Person, PersonKind, RoleType, UpdatePersonInput};
use sqlx::{postgres::PgPool, PgPool as Pool, Row};
use sqlx::postgres::PgRow;
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
        reports_to_person_id: row.get("reports_to_person_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

const SELECT_COLS: &str = "id, company_id, kind, display_name, role_type, specialty,
                ai_profile_id, reports_to_person_id, created_at, updated_at";

pub async fn list_people(pool: &PgPool, company_id: Uuid) -> Result<Vec<Person>> {
    let rows = sqlx::query(&format!(
        "SELECT {SELECT_COLS}
         FROM people
         WHERE company_id = $1
         ORDER BY created_at ASC"
    ))
    .bind(company_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_person).collect())
}

/// `ai_profile_id` of the company's first AI co-founder — used as the default for new hires.
pub async fn ai_profile_id_of_ai_co_founder(
    pool: &PgPool,
    company_id: Uuid,
) -> Result<Option<Uuid>> {
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT ai_profile_id FROM people
         WHERE company_id = $1
           AND kind = 'ai_agent'
           AND role_type = 'co_founder'
           AND ai_profile_id IS NOT NULL
         ORDER BY created_at ASC
         LIMIT 1",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(id)
}

/// Whether the company already has someone in this **executive** seat (`ceo`, `cto`, or `cfo`).
/// Used to block duplicate `propose_hire` for the same top role.
pub async fn company_has_executive_role(
    pool: &PgPool,
    company_id: Uuid,
    role: RoleType,
) -> Result<bool> {
    if !matches!(role, RoleType::Ceo | RoleType::Cto | RoleType::Cfo) {
        return Ok(false);
    }
    let role_s = role.to_string();
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM people WHERE company_id = $1 AND role_type = $2",
    )
    .bind(company_id)
    .bind(role_s)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

pub async fn get_person(
    pool: &PgPool,
    company_id: Uuid,
    person_id: Uuid,
) -> Result<Option<Person>> {
    let row = sqlx::query(&format!(
        "SELECT {SELECT_COLS}
         FROM people
         WHERE id = $1 AND company_id = $2"
    ))
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
    let row = sqlx::query(&format!(
        "INSERT INTO people (company_id, kind, display_name, role_type, specialty, ai_profile_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {SELECT_COLS}"
    ))
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

pub async fn update_person(
    pool: &Pool,
    company_id: Uuid,
    person_id: Uuid,
    input: UpdatePersonInput,
) -> Result<Option<Person>> {
    let row = sqlx::query(&format!(
        "UPDATE people
         SET display_name          = COALESCE($3, display_name),
             role_type             = COALESCE($4, role_type),
             specialty             = CASE WHEN $5 THEN $6 ELSE specialty END,
             ai_profile_id        = CASE WHEN $7 THEN $8 ELSE ai_profile_id END,
             reports_to_person_id = CASE WHEN $9 THEN $10 ELSE reports_to_person_id END,
             updated_at            = NOW()
         WHERE id = $1 AND company_id = $2
         RETURNING {SELECT_COLS}"
    ))
    .bind(person_id)
    .bind(company_id)
    .bind(input.display_name.as_deref())
    .bind(input.role_type.as_ref().map(|r| r.to_string()))
    .bind(input.specialty.is_some())
    .bind(input.specialty.flatten().as_deref())
    .bind(input.ai_profile_id.is_some())
    .bind(input.ai_profile_id.flatten())
    .bind(input.reports_to_person_id.is_some())
    .bind(input.reports_to_person_id.flatten())
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_person))
}

pub async fn delete_person(
    pool: &PgPool,
    company_id: Uuid,
    person_id: Uuid,
) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM people WHERE id = $1 AND company_id = $2",
    )
    .bind(person_id)
    .bind(company_id)
    .execute(pool)
    .await?;

    Ok(res.rows_affected() > 0)
}

/// Seed the human founder row. Idempotent — skips if already exists.
pub async fn seed_founder(
    pool: &PgPool,
    company_id: Uuid,
    display_name: &str,
) -> Result<Person> {
    sqlx::query(
        "INSERT INTO people (company_id, kind, display_name, role_type)
         VALUES ($1, 'human_founder', $2, 'co_founder')
         ON CONFLICT DO NOTHING",
    )
    .bind(company_id)
    .bind(display_name)
    .execute(pool)
    .await?;

    let row = sqlx::query(&format!(
        "SELECT {SELECT_COLS}
         FROM people
         WHERE company_id = $1 AND kind = 'human_founder'
         LIMIT 1"
    ))
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_person(&row))
}

/// Update the `reports_to_person_id` for a person with cycle detection.
///
/// Pass `new_manager_id = None` to clear the reporting line (make root).
/// Returns an error if the update would create a cycle.
pub async fn update_reporting_line(
    pool: &PgPool,
    company_id: Uuid,
    person_id: Uuid,
    new_manager_id: Option<Uuid>,
) -> Result<Person> {
    if let Some(manager_id) = new_manager_id {
        if manager_id == person_id {
            return Err(anyhow!("a person cannot report to themselves"));
        }

        // Cycle check: walk UP from new_manager_id; if we encounter person_id, it's a cycle.
        let would_cycle: bool = sqlx::query_scalar(
            "WITH RECURSIVE upchain AS (
                SELECT id, reports_to_person_id
                FROM people
                WHERE id = $1 AND company_id = $3
                UNION ALL
                SELECT p.id, p.reports_to_person_id
                FROM people p
                JOIN upchain u ON p.id = u.reports_to_person_id
                WHERE u.reports_to_person_id IS NOT NULL
            )
            SELECT EXISTS(SELECT 1 FROM upchain WHERE id = $2)",
        )
        .bind(manager_id)
        .bind(person_id)
        .bind(company_id)
        .fetch_one(pool)
        .await?;

        if would_cycle {
            return Err(anyhow!(
                "setting this manager would create a reporting cycle"
            ));
        }

        // Verify manager belongs to the same company
        let manager_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM people WHERE id = $1 AND company_id = $2)",
        )
        .bind(manager_id)
        .bind(company_id)
        .fetch_one(pool)
        .await?;

        if !manager_exists {
            return Err(anyhow!("manager not found in this company"));
        }
    }

    let row = sqlx::query(&format!(
        "UPDATE people
         SET reports_to_person_id = $3, updated_at = NOW()
         WHERE id = $1 AND company_id = $2
         RETURNING {SELECT_COLS}"
    ))
    .bind(person_id)
    .bind(company_id)
    .bind(new_manager_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("person not found"))?;

    Ok(row_to_person(&row))
}
