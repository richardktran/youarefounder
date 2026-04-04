use anyhow::Result;
use domain::{AddWorkspaceMemberInput, WorkspaceMember, WorkspaceMemberRole};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_member(row: &PgRow) -> WorkspaceMember {
    let role_str: String = row.get("role");
    WorkspaceMember {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        person_id: row.get("person_id"),
        role: role_str
            .parse::<WorkspaceMemberRole>()
            .unwrap_or_default(),
        created_at: row.get("created_at"),
        display_name: row.get("display_name"),
        person_kind: row.get("kind"),
        role_type: row.get("role_type"),
        specialty: row.get("specialty"),
        ai_profile_id: row.get("ai_profile_id"),
    }
}

pub async fn list_workspace_members(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceMember>> {
    let rows = sqlx::query(
        "SELECT wm.id, wm.workspace_id, wm.person_id, wm.role, wm.created_at,
                p.display_name, p.kind, p.role_type, p.specialty, p.ai_profile_id
         FROM workspace_members wm
         JOIN people p ON p.id = wm.person_id
         WHERE wm.workspace_id = $1
         ORDER BY wm.created_at ASC",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_member).collect())
}

pub async fn add_workspace_member(
    pool: &PgPool,
    workspace_id: Uuid,
    input: AddWorkspaceMemberInput,
) -> Result<WorkspaceMember> {
    let row = sqlx::query(
        "INSERT INTO workspace_members (workspace_id, person_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (workspace_id, person_id) DO UPDATE SET role = EXCLUDED.role
         RETURNING id, workspace_id, person_id, role, created_at",
    )
    .bind(workspace_id)
    .bind(input.person_id)
    .bind(input.role.to_string())
    .fetch_one(pool)
    .await?;

    // Fetch denormalised data from people.
    let person_row = sqlx::query(
        "SELECT display_name, kind, role_type, specialty, ai_profile_id
         FROM people WHERE id = $1",
    )
    .bind(input.person_id)
    .fetch_one(pool)
    .await?;

    let role_str: String = row.get("role");
    Ok(WorkspaceMember {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        person_id: row.get("person_id"),
        role: role_str.parse::<WorkspaceMemberRole>().unwrap_or_default(),
        created_at: row.get("created_at"),
        display_name: person_row.get("display_name"),
        person_kind: person_row.get("kind"),
        role_type: person_row.get("role_type"),
        specialty: person_row.get("specialty"),
        ai_profile_id: person_row.get("ai_profile_id"),
    })
}

pub async fn remove_workspace_member(
    pool: &PgPool,
    workspace_id: Uuid,
    person_id: Uuid,
) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM workspace_members WHERE workspace_id = $1 AND person_id = $2",
    )
    .bind(workspace_id)
    .bind(person_id)
    .execute(pool)
    .await?;

    Ok(res.rows_affected() > 0)
}
