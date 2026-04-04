use anyhow::Result;
use domain::{
    CreateCommentInput, CreateTicketInput, Ticket, TicketComment, TicketPriority, TicketStatus,
    TicketType, UpdateTicketInput,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_ticket(row: &PgRow) -> Ticket {
    let status_str: String = row.get("status");
    let type_str: String = row.get("ticket_type");
    let priority_str: String = row.get("priority");

    Ticket {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        description: row.get("description"),
        ticket_type: type_str.parse::<TicketType>().unwrap_or_default(),
        status: status_str.parse::<TicketStatus>().unwrap_or_default(),
        priority: priority_str.parse::<TicketPriority>().unwrap_or_default(),
        assignee_person_id: row.get("assignee_person_id"),
        parent_ticket_id: row.get("parent_ticket_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_comment(row: &PgRow) -> TicketComment {
    TicketComment {
        id: row.get("id"),
        ticket_id: row.get("ticket_id"),
        body: row.get("body"),
        author_person_id: row.get("author_person_id"),
        created_at: row.get("created_at"),
    }
}

// ─── Tickets ──────────────────────────────────────────────────────────────────

pub async fn list_tickets(pool: &PgPool, workspace_id: Uuid) -> Result<Vec<Ticket>> {
    let rows = sqlx::query(
        "SELECT id, workspace_id, title, description, ticket_type, status, priority,
                assignee_person_id, parent_ticket_id, created_at, updated_at
         FROM tickets
         WHERE workspace_id = $1
         ORDER BY created_at ASC",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_ticket).collect())
}

pub async fn get_ticket(pool: &PgPool, ticket_id: Uuid) -> Result<Option<Ticket>> {
    let row = sqlx::query(
        "SELECT id, workspace_id, title, description, ticket_type, status, priority,
                assignee_person_id, parent_ticket_id, created_at, updated_at
         FROM tickets
         WHERE id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_ticket))
}

pub async fn create_ticket(
    pool: &PgPool,
    workspace_id: Uuid,
    input: CreateTicketInput,
) -> Result<Ticket> {
    let ticket_type = input.ticket_type.unwrap_or_default().to_string();
    let status = input.status.unwrap_or_default().to_string();
    let priority = input.priority.unwrap_or_default().to_string();

    let row = sqlx::query(
        "INSERT INTO tickets
             (workspace_id, title, description, ticket_type, status, priority,
              assignee_person_id, parent_ticket_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, workspace_id, title, description, ticket_type, status, priority,
                   assignee_person_id, parent_ticket_id, created_at, updated_at",
    )
    .bind(workspace_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&ticket_type)
    .bind(&status)
    .bind(&priority)
    .bind(input.assignee_person_id)
    .bind(input.parent_ticket_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_ticket(&row))
}

pub async fn update_ticket(
    pool: &PgPool,
    ticket_id: Uuid,
    input: UpdateTicketInput,
) -> Result<Option<Ticket>> {
    let type_str = input.ticket_type.as_ref().map(|t| t.to_string());
    let status_str = input.status.as_ref().map(|s| s.to_string());
    let priority_str = input.priority.as_ref().map(|p| p.to_string());

    let row = sqlx::query(
        "UPDATE tickets
         SET
             title               = COALESCE($2, title),
             description         = COALESCE($3, description),
             ticket_type         = COALESCE($4, ticket_type),
             status              = COALESCE($5, status),
             priority            = COALESCE($6, priority),
             assignee_person_id  = COALESCE($7, assignee_person_id),
             parent_ticket_id    = COALESCE($8, parent_ticket_id),
             updated_at          = NOW()
         WHERE id = $1
         RETURNING id, workspace_id, title, description, ticket_type, status, priority,
                   assignee_person_id, parent_ticket_id, created_at, updated_at",
    )
    .bind(ticket_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&type_str)
    .bind(&status_str)
    .bind(&priority_str)
    .bind(input.assignee_person_id)
    .bind(input.parent_ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_ticket))
}

pub async fn delete_ticket(pool: &PgPool, ticket_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM tickets WHERE id = $1")
        .bind(ticket_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// ─── Comments ─────────────────────────────────────────────────────────────────

pub async fn list_comments(pool: &PgPool, ticket_id: Uuid) -> Result<Vec<TicketComment>> {
    let rows = sqlx::query(
        "SELECT id, ticket_id, body, author_person_id, created_at
         FROM ticket_comments
         WHERE ticket_id = $1
         ORDER BY created_at ASC",
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_comment).collect())
}

pub async fn create_comment(
    pool: &PgPool,
    ticket_id: Uuid,
    input: CreateCommentInput,
) -> Result<TicketComment> {
    let row = sqlx::query(
        "INSERT INTO ticket_comments (ticket_id, body, author_person_id)
         VALUES ($1, $2, $3)
         RETURNING id, ticket_id, body, author_person_id, created_at",
    )
    .bind(ticket_id)
    .bind(&input.body)
    .bind(input.author_person_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_comment(&row))
}
