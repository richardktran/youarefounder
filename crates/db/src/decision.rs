use anyhow::{anyhow, Result};
use domain::{
    AnswerDecisionRequestInput, CreateDecisionRequestInput, DecisionRequest, DecisionStatus,
    TicketStatus,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_decision(row: &PgRow) -> DecisionRequest {
    let status_str: String = row.get("status");
    DecisionRequest {
        id: row.get("id"),
        company_id: row.get("company_id"),
        workspace_id: row.get("workspace_id"),
        ticket_id: row.get("ticket_id"),
        raised_by_person_id: row.get("raised_by_person_id"),
        question: row.get("question"),
        context_note: row.get("context_note"),
        status: status_str
            .parse::<DecisionStatus>()
            .unwrap_or(DecisionStatus::PendingFounder),
        founder_answer: row.get("founder_answer"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_decision_requests(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<DecisionStatus>,
) -> Result<Vec<DecisionRequest>> {
    let rows = if let Some(status) = status_filter {
        sqlx::query(
            "SELECT dr.id, dr.company_id, t.workspace_id, dr.ticket_id, dr.raised_by_person_id,
                    dr.question, dr.context_note, dr.status, dr.founder_answer,
                    dr.created_at, dr.updated_at
             FROM decision_requests dr
             INNER JOIN tickets t ON t.id = dr.ticket_id
             WHERE dr.company_id = $1 AND dr.status = $2
             ORDER BY dr.created_at DESC",
        )
        .bind(company_id)
        .bind(status.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT dr.id, dr.company_id, t.workspace_id, dr.ticket_id, dr.raised_by_person_id,
                    dr.question, dr.context_note, dr.status, dr.founder_answer,
                    dr.created_at, dr.updated_at
             FROM decision_requests dr
             INNER JOIN tickets t ON t.id = dr.ticket_id
             WHERE dr.company_id = $1
             ORDER BY dr.created_at DESC",
        )
        .bind(company_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.iter().map(row_to_decision).collect())
}

pub async fn get_decision_request(
    pool: &PgPool,
    company_id: Uuid,
    decision_id: Uuid,
) -> Result<Option<DecisionRequest>> {
    let row = sqlx::query(
        "SELECT dr.id, dr.company_id, t.workspace_id, dr.ticket_id, dr.raised_by_person_id,
                dr.question, dr.context_note, dr.status, dr.founder_answer,
                dr.created_at, dr.updated_at
         FROM decision_requests dr
         INNER JOIN tickets t ON t.id = dr.ticket_id
         WHERE dr.id = $1 AND dr.company_id = $2",
    )
    .bind(decision_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_decision))
}

pub async fn create_decision_request(
    pool: &PgPool,
    company_id: Uuid,
    input: CreateDecisionRequestInput,
) -> Result<DecisionRequest> {
    if input.question.trim().is_empty() {
        return Err(anyhow!("question is required"));
    }

    let mut tx = pool.begin().await?;

    // Reject cross-company mistakes early (clearer than a silent insert failure).
    let ticket_company: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT w.company_id
             FROM tickets t
             INNER JOIN workspaces w ON w.id = t.workspace_id
             WHERE t.id = $1"#,
    )
    .bind(input.ticket_id)
    .fetch_optional(&mut *tx)
    .await?;

    let ticket_company =
        ticket_company.ok_or_else(|| anyhow!("ticket not found for decision request"))?;
    if ticket_company != company_id {
        return Err(anyhow!(
            "ticket {} belongs to a different company than the request context",
            input.ticket_id
        ));
    }

    let id_row = sqlx::query(
        r#"INSERT INTO decision_requests (company_id, ticket_id, raised_by_person_id, question, context_note)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(company_id)
    .bind(input.ticket_id)
    .bind(input.raised_by_person_id)
    .bind(input.question.trim())
    .bind(&input.context_note)
    .fetch_one(&mut *tx)
    .await?;

    let new_id: Uuid = id_row.get("id");

    let row = sqlx::query(
        r#"SELECT dr.id, dr.company_id, t.workspace_id, dr.ticket_id, dr.raised_by_person_id, dr.question,
                  dr.context_note, dr.status, dr.founder_answer, dr.created_at, dr.updated_at
           FROM decision_requests dr
           INNER JOIN tickets t ON t.id = dr.ticket_id
           WHERE dr.id = $1"#,
    )
    .bind(new_id)
    .fetch_one(&mut *tx)
    .await?;

    // Block the parent ticket so the scheduler won't pick it up.
    sqlx::query(
        "UPDATE tickets SET status = $1, updated_at = NOW()
         WHERE id = $2 AND status NOT IN ('done', 'cancelled')",
    )
    .bind(TicketStatus::Blocked.to_string())
    .bind(input.ticket_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(row_to_decision(&row))
}

/// Answer a decision request: store the founder's answer, mark answered,
/// and unblock the parent ticket so the scheduler can pick it up again.
pub async fn answer_decision_request(
    pool: &PgPool,
    company_id: Uuid,
    decision_id: Uuid,
    input: AnswerDecisionRequestInput,
) -> Result<DecisionRequest> {
    if input.founder_answer.trim().is_empty() {
        return Err(anyhow!("founder_answer is required"));
    }

    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        r#"UPDATE decision_requests dr
         SET status = 'answered',
             founder_answer = $3,
             updated_at = NOW()
         FROM tickets t
         WHERE dr.id = $1 AND dr.company_id = $2 AND dr.ticket_id = t.id
           AND dr.status = 'pending_founder'
         RETURNING dr.id, dr.company_id, t.workspace_id, dr.ticket_id, dr.raised_by_person_id,
                   dr.question, dr.context_note, dr.status, dr.founder_answer,
                   dr.created_at, dr.updated_at"#,
    )
    .bind(decision_id)
    .bind(company_id)
    .bind(input.founder_answer.trim())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("decision request not found or already answered"))?;

    let decision = row_to_decision(&row);

    // Add the founder's answer as a comment on the ticket so the agent sees it.
    sqlx::query(
        "INSERT INTO ticket_comments (ticket_id, body, author_person_id)
         VALUES ($1, $2, NULL)",
    )
    .bind(decision.ticket_id)
    .bind(format!(
        "[Founder decision] {}\n\nFounder's answer: {}",
        decision.question, decision.founder_answer.as_deref().unwrap_or("")
    ))
    .execute(&mut *tx)
    .await?;

    // Unblock the ticket — move it back to in_progress so it can be picked up.
    sqlx::query(
        "UPDATE tickets SET status = 'in_progress', updated_at = NOW()
         WHERE id = $1 AND status = 'blocked'",
    )
    .bind(decision.ticket_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(decision)
}

/// Delete founder decision requests still pending for this ticket (e.g. when the
/// ticket is moved to done or cancelled).
pub async fn delete_pending_decisions_for_ticket(pool: &PgPool, ticket_id: Uuid) -> Result<()> {
    sqlx::query(
        "DELETE FROM decision_requests
         WHERE ticket_id = $1 AND status = 'pending_founder'",
    )
    .bind(ticket_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Check whether a ticket has any open (pending_founder) decision requests.
pub async fn has_open_decision(pool: &PgPool, ticket_id: Uuid) -> Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM decision_requests
         WHERE ticket_id = $1 AND status = 'pending_founder'
         LIMIT 1",
    )
    .bind(ticket_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

/// Remove a decision request from the inbox. If it was still pending, unblocks the ticket.
pub async fn delete_decision_request(
    pool: &PgPool,
    company_id: Uuid,
    decision_id: Uuid,
) -> Result<bool> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        r#"SELECT dr.ticket_id, dr.status
           FROM decision_requests dr
           WHERE dr.id = $1 AND dr.company_id = $2"#,
    )
    .bind(decision_id)
    .bind(company_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        tx.commit().await?;
        return Ok(false);
    };

    let ticket_id: Uuid = row.get("ticket_id");
    let status: String = row.get("status");

    sqlx::query(
        "DELETE FROM decision_requests WHERE id = $1 AND company_id = $2",
    )
    .bind(decision_id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if status == DecisionStatus::PendingFounder.to_string() {
        sqlx::query(
            "UPDATE tickets SET status = $1, updated_at = NOW()
             WHERE id = $2 AND status = $3",
        )
        .bind(TicketStatus::InProgress.to_string())
        .bind(ticket_id)
        .bind(TicketStatus::Blocked.to_string())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}
