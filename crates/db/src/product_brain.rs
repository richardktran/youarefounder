use anyhow::Result;
use domain::{
    ApprovePendingBrainInput, ProductBrainEntry, ProductBrainPending, ProductBrainPendingStatus,
    Ticket, TicketComment, TicketReference, TicketStatus,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_entry(row: &PgRow) -> ProductBrainEntry {
    ProductBrainEntry {
        id: row.get("id"),
        company_id: row.get("company_id"),
        workspace_id: row.get("workspace_id"),
        body: row.get("body"),
        source_ticket_id: row.get("source_ticket_id"),
        created_at: row.get("created_at"),
    }
}

fn row_pending(row: &PgRow) -> ProductBrainPending {
    let status_str: String = row.get("status");
    ProductBrainPending {
        id: row.get("id"),
        company_id: row.get("company_id"),
        workspace_id: row.get("workspace_id"),
        body: row.get("body"),
        source_ticket_id: row.get("source_ticket_id"),
        status: status_str
            .parse::<ProductBrainPendingStatus>()
            .unwrap_or(ProductBrainPendingStatus::Pending),
        proposed_at: row.get("proposed_at"),
        reviewed_at: row.get("reviewed_at"),
    }
}

fn row_reference(row: &PgRow) -> TicketReference {
    TicketReference {
        from_ticket_id: row.get("from_ticket_id"),
        to_ticket_id: row.get("to_ticket_id"),
        note: row.get("note"),
        created_at: row.get("created_at"),
    }
}

/// Approved brain entries for agent context: company-wide (`workspace_id` NULL) plus this workspace.
pub async fn list_approved_for_context(
    pool: &PgPool,
    company_id: Uuid,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<ProductBrainEntry>> {
    let rows = sqlx::query(
        "SELECT id, company_id, workspace_id, body, source_ticket_id, created_at
         FROM product_brain_entries
         WHERE company_id = $1 AND (workspace_id IS NULL OR workspace_id = $2)
         ORDER BY created_at DESC
         LIMIT $3",
    )
    .bind(company_id)
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_entry).collect())
}

pub async fn list_entries_by_company(
    pool: &PgPool,
    company_id: Uuid,
    limit: i64,
) -> Result<Vec<ProductBrainEntry>> {
    let rows = sqlx::query(
        "SELECT id, company_id, workspace_id, body, source_ticket_id, created_at
         FROM product_brain_entries
         WHERE company_id = $1
         ORDER BY created_at DESC
         LIMIT $2",
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_entry).collect())
}

/// Approved entries produced from a specific ticket (for referenced-ticket snapshots).
pub async fn list_entries_by_source_ticket(
    pool: &PgPool,
    company_id: Uuid,
    source_ticket_id: Uuid,
    limit: i64,
) -> Result<Vec<ProductBrainEntry>> {
    let rows = sqlx::query(
        "SELECT id, company_id, workspace_id, body, source_ticket_id, created_at
         FROM product_brain_entries
         WHERE company_id = $1 AND source_ticket_id = $2
         ORDER BY created_at DESC
         LIMIT $3",
    )
    .bind(company_id)
    .bind(source_ticket_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_entry).collect())
}

pub async fn list_pending(
    pool: &PgPool,
    company_id: Uuid,
    status: Option<ProductBrainPendingStatus>,
    limit: i64,
) -> Result<Vec<ProductBrainPending>> {
    let rows = if let Some(st) = status {
        let s = st.to_string();
        sqlx::query(
            "SELECT id, company_id, workspace_id, body, source_ticket_id, status, proposed_at, reviewed_at
             FROM product_brain_pending
             WHERE company_id = $1 AND status = $2
             ORDER BY proposed_at DESC
             LIMIT $3",
        )
        .bind(company_id)
        .bind(s)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, company_id, workspace_id, body, source_ticket_id, status, proposed_at, reviewed_at
             FROM product_brain_pending
             WHERE company_id = $1
             ORDER BY proposed_at DESC
             LIMIT $2",
        )
        .bind(company_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.iter().map(row_pending).collect())
}

pub async fn get_pending(pool: &PgPool, id: Uuid) -> Result<Option<ProductBrainPending>> {
    let row = sqlx::query(
        "SELECT id, company_id, workspace_id, body, source_ticket_id, status, proposed_at, reviewed_at
         FROM product_brain_pending WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_pending))
}

/// Returns true if a pending row already exists for this ticket (avoids duplicate drafts on re-done toggles).
pub async fn has_pending_for_source_ticket(
    pool: &PgPool,
    source_ticket_id: Uuid,
) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM product_brain_pending
         WHERE source_ticket_id = $1 AND status = 'pending' LIMIT 1",
    )
    .bind(source_ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub async fn insert_pending(
    pool: &PgPool,
    company_id: Uuid,
    workspace_id: Option<Uuid>,
    body: String,
    source_ticket_id: Option<Uuid>,
) -> Result<Uuid> {
    let row = sqlx::query(
        "INSERT INTO product_brain_pending (company_id, workspace_id, body, source_ticket_id)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(company_id)
    .bind(workspace_id)
    .bind(&body)
    .bind(source_ticket_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get("id"))
}

pub async fn approve_pending(
    pool: &PgPool,
    pending_id: Uuid,
    input: ApprovePendingBrainInput,
) -> Result<Option<ProductBrainEntry>> {
    let mut tx = pool.begin().await?;

    let pending = sqlx::query(
        "SELECT id, company_id, workspace_id, body, source_ticket_id, status
         FROM product_brain_pending WHERE id = $1 FOR UPDATE",
    )
    .bind(pending_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(p) = pending else {
        tx.commit().await?;
        return Ok(None);
    };

    let status: String = p.get("status");
    if status != "pending" {
        tx.commit().await?;
        return Ok(None);
    }

    let body: String = p.get("body");
    let final_body = input.body.unwrap_or(body);
    let company_id: Uuid = p.get("company_id");
    let workspace_id: Option<Uuid> = p.get("workspace_id");
    let source_ticket_id: Option<Uuid> = p.get("source_ticket_id");

    let entry_row = sqlx::query(
        "INSERT INTO product_brain_entries (company_id, workspace_id, body, source_ticket_id)
         VALUES ($1, $2, $3, $4)
         RETURNING id, company_id, workspace_id, body, source_ticket_id, created_at",
    )
    .bind(company_id)
    .bind(workspace_id)
    .bind(&final_body)
    .bind(source_ticket_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE product_brain_pending
         SET status = 'promoted', reviewed_at = NOW()
         WHERE id = $1",
    )
    .bind(pending_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some(row_entry(&entry_row)))
}

pub async fn reject_pending(pool: &PgPool, pending_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE product_brain_pending
         SET status = 'rejected', reviewed_at = NOW()
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(pending_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_references_from(
    pool: &PgPool,
    from_ticket_id: Uuid,
) -> Result<Vec<TicketReference>> {
    let rows = sqlx::query(
        "SELECT from_ticket_id, to_ticket_id, note, created_at
         FROM ticket_references WHERE from_ticket_id = $1
         ORDER BY created_at ASC",
    )
    .bind(from_ticket_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_reference).collect())
}

async fn ticket_company_id(pool: &PgPool, ticket_id: Uuid) -> Result<Option<Uuid>> {
    let row = sqlx::query(
        "SELECT w.company_id FROM tickets t
         JOIN workspaces w ON t.workspace_id = w.id
         WHERE t.id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get("company_id")))
}

pub async fn add_ticket_reference(
    pool: &PgPool,
    from_ticket_id: Uuid,
    to_ticket_id: Uuid,
    note: Option<String>,
) -> Result<()> {
    let c1 = ticket_company_id(pool, from_ticket_id).await?;
    let c2 = ticket_company_id(pool, to_ticket_id).await?;
    match (c1, c2) {
        (Some(a), Some(b)) if a == b => {}
        _ => anyhow::bail!("tickets must exist and belong to the same company"),
    }

    sqlx::query(
        "INSERT INTO ticket_references (from_ticket_id, to_ticket_id, note)
         VALUES ($1, $2, $3)
         ON CONFLICT (from_ticket_id, to_ticket_id) DO UPDATE SET note = EXCLUDED.note",
    )
    .bind(from_ticket_id)
    .bind(to_ticket_id)
    .bind(&note)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_ticket_reference(
    pool: &PgPool,
    from_ticket_id: Uuid,
    to_ticket_id: Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM ticket_references WHERE from_ticket_id = $1 AND to_ticket_id = $2",
    )
    .bind(from_ticket_id)
    .bind(to_ticket_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

const DRAFT_MAX_CHARS: usize = 12_000;

fn build_draft_body(ticket: &Ticket, comments: &[TicketComment]) -> String {
    let mut s = String::new();
    s.push_str(&format!("Ticket: {}\n", ticket.title));
    if let Some(ref d) = ticket.description {
        if !d.is_empty() {
            s.push_str(&format!("Description:\n{d}\n\n"));
        }
    }
    if let Some(ref dod) = ticket.definition_of_done {
        if !dod.is_empty() {
            s.push_str(&format!("Definition of done:\n{dod}\n\n"));
        }
    }
    if let Some(ref o) = ticket.outcome_summary {
        if !o.is_empty() {
            s.push_str(&format!("Outcome summary:\n{o}\n\n"));
        }
    }
    s.push_str("Thread (recent comments, oldest first in excerpt):\n");
    let tail: Vec<_> = comments.iter().rev().take(20).collect();
    for c in tail.iter().rev() {
        let line = c.body.trim();
        if line.len() > 2000 {
            s.push_str(&format!("- {}…\n", &line[..2000]));
        } else {
            s.push_str(&format!("- {line}\n"));
        }
    }
    if s.len() > DRAFT_MAX_CHARS {
        s.truncate(DRAFT_MAX_CHARS);
        s.push_str("\n…(truncated)");
    }
    s
}

/// Queues a founder-review draft when a ticket reaches `done` (idempotent per pending row).
pub async fn enqueue_draft_from_completed_ticket(pool: &PgPool, ticket: &Ticket) -> Result<()> {
    if ticket.status != TicketStatus::Done {
        return Ok(());
    }
    if has_pending_for_source_ticket(pool, ticket.id).await? {
        return Ok(());
    }

    let row = sqlx::query("SELECT company_id FROM workspaces WHERE id = $1")
        .bind(ticket.workspace_id)
        .fetch_optional(pool)
        .await?;
    let Some(r) = row else {
        return Ok(());
    };
    let company_id: Uuid = r.get("company_id");

    let comments = crate::ticket::list_comments(pool, ticket.id).await?;
    let body = build_draft_body(ticket, &comments);
    let pending_id = insert_pending(
        pool,
        company_id,
        Some(ticket.workspace_id),
        body,
        Some(ticket.id),
    )
    .await?;
    let _ = approve_pending(
        pool,
        pending_id,
        ApprovePendingBrainInput { body: None },
    )
    .await?;
    Ok(())
}
