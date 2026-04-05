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
            "SELECT id, company_id, ticket_id, raised_by_person_id, question,
                    context_note, status, founder_answer, created_at, updated_at
             FROM decision_requests
             WHERE company_id = $1 AND status = $2
             ORDER BY created_at DESC",
        )
        .bind(company_id)
        .bind(status.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, company_id, ticket_id, raised_by_person_id, question,
                    context_note, status, founder_answer, created_at, updated_at
             FROM decision_requests
             WHERE company_id = $1
             ORDER BY created_at DESC",
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
        "SELECT id, company_id, ticket_id, raised_by_person_id, question,
                context_note, status, founder_answer, created_at, updated_at
         FROM decision_requests
         WHERE id = $1 AND company_id = $2",
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

    let row = sqlx::query(
        "INSERT INTO decision_requests
             (company_id, ticket_id, raised_by_person_id, question, context_note)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, company_id, ticket_id, raised_by_person_id, question,
                   context_note, status, founder_answer, created_at, updated_at",
    )
    .bind(company_id)
    .bind(input.ticket_id)
    .bind(input.raised_by_person_id)
    .bind(input.question.trim())
    .bind(&input.context_note)
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
        "UPDATE decision_requests
         SET status = 'answered',
             founder_answer = $3,
             updated_at = NOW()
         WHERE id = $1 AND company_id = $2 AND status = 'pending_founder'
         RETURNING id, company_id, ticket_id, raised_by_person_id, question,
                   context_note, status, founder_answer, created_at, updated_at",
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
