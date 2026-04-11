use anyhow::Result;
use domain::{
    CreateCommentInput, CreateTicketInput, Ticket, TicketComment, TicketPriority, TicketStatus,
    TicketType, UpdateTicketInput,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

/// How to filter tickets in a workspace list.
#[derive(Debug, Clone, Copy, Default)]
pub enum TicketListFilter {
    /// Every ticket in the workspace.
    #[default]
    All,
    /// Only top-level tickets (`parent_ticket_id IS NULL`), e.g. Kanban board.
    RootsOnly,
    /// Direct children of a parent ticket.
    ChildrenOf(Uuid),
}

fn row_to_ticket(row: &PgRow) -> Ticket {
    let status_str: String = row.get("status");
    let type_str: String = row.get("ticket_type");
    let priority_str: String = row.get("priority");

    Ticket {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        description: row.get("description"),
        definition_of_done: row.get("definition_of_done"),
        founder_memory: row.get("founder_memory"),
        outcome_summary: row.get("outcome_summary"),
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

pub async fn list_tickets(
    pool: &PgPool,
    workspace_id: Uuid,
    filter: TicketListFilter,
) -> Result<Vec<Ticket>> {
    let rows = match filter {
        TicketListFilter::All => {
            sqlx::query(
                "SELECT id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
                        assignee_person_id, parent_ticket_id, created_at, updated_at
                 FROM tickets
                 WHERE workspace_id = $1
                 ORDER BY created_at ASC",
            )
            .bind(workspace_id)
            .fetch_all(pool)
            .await?
        }
        TicketListFilter::RootsOnly => {
            sqlx::query(
                "SELECT id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
                        assignee_person_id, parent_ticket_id, created_at, updated_at
                 FROM tickets
                 WHERE workspace_id = $1 AND parent_ticket_id IS NULL
                 ORDER BY created_at ASC",
            )
            .bind(workspace_id)
            .fetch_all(pool)
            .await?
        }
        TicketListFilter::ChildrenOf(parent_id) => {
            sqlx::query(
                "SELECT id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
                        assignee_person_id, parent_ticket_id, created_at, updated_at
                 FROM tickets
                 WHERE workspace_id = $1 AND parent_ticket_id = $2
                 ORDER BY created_at ASC",
            )
            .bind(workspace_id)
            .bind(parent_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows.iter().map(row_to_ticket).collect())
}

pub async fn get_ticket(pool: &PgPool, ticket_id: Uuid) -> Result<Option<Ticket>> {
    let row = sqlx::query(
        "SELECT id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
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
             (workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
              assignee_person_id, parent_ticket_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
                   assignee_person_id, parent_ticket_id, created_at, updated_at",
    )
    .bind(workspace_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.definition_of_done)
    .bind(&input.founder_memory)
    .bind(&input.outcome_summary)
    .bind(&ticket_type)
    .bind(&status)
    .bind(&priority)
    .bind(input.assignee_person_id)
    .bind(input.parent_ticket_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_ticket(&row))
}

/// Walks up from `completed_ticket_id`: while every subtask under the parent is done or cancelled,
/// marks that parent done (unless blocked / already terminal). Uses a loop to avoid async recursion.
pub async fn maybe_roll_up_parent_after_subtasks_closed(
    pool: &PgPool,
    mut completed_ticket_id: Uuid,
) -> Result<()> {
    loop {
        let ticket = match get_ticket(pool, completed_ticket_id).await? {
            Some(t) => t,
            None => break,
        };

        let parent_id = match ticket.parent_ticket_id {
            Some(p) => p,
            None => break,
        };

        let siblings =
            list_tickets(pool, ticket.workspace_id, TicketListFilter::ChildrenOf(parent_id))
                .await?;
        if siblings.is_empty() {
            break;
        }

        let all_closed = siblings.iter().all(|c| {
            matches!(c.status, TicketStatus::Done | TicketStatus::Cancelled)
        });
        if !all_closed {
            break;
        }

        let parent = match get_ticket(pool, parent_id).await? {
            Some(p) => p,
            None => break,
        };
        if matches!(
            parent.status,
            TicketStatus::Done | TicketStatus::Cancelled | TicketStatus::Blocked
        ) {
            break;
        }

        let parent_ticket = update_ticket_without_roll_up_hook(
            pool,
            parent_id,
            UpdateTicketInput {
                status: Some(TicketStatus::Done),
                ..Default::default()
            },
        )
        .await?;

        if let Some(ref pt) = parent_ticket {
            if pt.status == TicketStatus::Done {
                crate::product_brain::enqueue_draft_from_completed_ticket(pool, pt).await?;
            }
        }

        completed_ticket_id = parent_id;
    }

    Ok(())
}

async fn update_ticket_without_roll_up_hook(
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
             definition_of_done  = COALESCE($4, definition_of_done),
             founder_memory      = COALESCE($5, founder_memory),
             outcome_summary     = COALESCE($6, outcome_summary),
             ticket_type         = COALESCE($7, ticket_type),
             status              = COALESCE($8, status),
             priority            = COALESCE($9, priority),
             assignee_person_id  = COALESCE($10, assignee_person_id),
             parent_ticket_id    = COALESCE($11, parent_ticket_id),
             updated_at          = NOW()
         WHERE id = $1
         RETURNING id, workspace_id, title, description, definition_of_done, founder_memory, outcome_summary, ticket_type, status, priority,
                   assignee_person_id, parent_ticket_id, created_at, updated_at",
    )
    .bind(ticket_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.definition_of_done)
    .bind(&input.founder_memory)
    .bind(&input.outcome_summary)
    .bind(&type_str)
    .bind(&status_str)
    .bind(&priority_str)
    .bind(input.assignee_person_id)
    .bind(input.parent_ticket_id)
    .fetch_optional(pool)
    .await?;

    let ticket = row.as_ref().map(row_to_ticket);

    if let Some(ref t) = ticket {
        if matches!(
            t.status,
            TicketStatus::Done | TicketStatus::Cancelled
        ) {
            crate::decision::delete_pending_decisions_for_ticket(pool, t.id).await?;
        }
    }

    Ok(ticket)
}

pub async fn update_ticket(
    pool: &PgPool,
    ticket_id: Uuid,
    input: UpdateTicketInput,
) -> Result<Option<Ticket>> {
    let prev = get_ticket(pool, ticket_id).await?;
    let should_try_roll_up = matches!(input.status, Some(TicketStatus::Done));

    let ticket = update_ticket_without_roll_up_hook(pool, ticket_id, input).await?;

    if let Some(ref t) = ticket {
        if t.status == TicketStatus::Done {
            let was_done = prev
                .as_ref()
                .is_some_and(|p| matches!(p.status, TicketStatus::Done));
            if !was_done {
                crate::product_brain::enqueue_draft_from_completed_ticket(pool, t).await?;
            }
        }
    }

    if should_try_roll_up {
        if let Some(ref t) = ticket {
            if t.status == TicketStatus::Done {
                maybe_roll_up_parent_after_subtasks_closed(pool, t.id).await?;
            }
        }
    }

    Ok(ticket)
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
