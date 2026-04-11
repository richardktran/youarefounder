use anyhow::{anyhow, Result};
use domain::{
    AcceptProposalInput, CreateProposalInput, DeclineProposalInput, HiringProposal,
    PersonKind, ProposalStatus, RoleType,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

fn row_to_proposal(row: &PgRow) -> HiringProposal {
    let status_str: String = row.get("status");
    HiringProposal {
        id: row.get("id"),
        company_id: row.get("company_id"),
        proposed_by_person_id: row.get("proposed_by_person_id"),
        employee_display_name: row.get("employee_display_name"),
        role_type: row.get("role_type"),
        specialty: row.get("specialty"),
        ai_profile_id: row.get("ai_profile_id"),
        rationale: row.get("rationale"),
        scope_of_work: row.get("scope_of_work"),
        status: status_str
            .parse::<ProposalStatus>()
            .unwrap_or(ProposalStatus::PendingFounder),
        founder_response_text: row.get("founder_response_text"),
        created_person_id: row.get("created_person_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_proposals(
    pool: &PgPool,
    company_id: Uuid,
    status_filter: Option<ProposalStatus>,
) -> Result<Vec<HiringProposal>> {
    let rows = if let Some(status) = status_filter {
        sqlx::query(
            "SELECT id, company_id, proposed_by_person_id, employee_display_name,
                    role_type, specialty, ai_profile_id, rationale, scope_of_work,
                    status, founder_response_text, created_person_id, created_at, updated_at
             FROM hiring_proposals
             WHERE company_id = $1 AND status = $2
             ORDER BY created_at DESC",
        )
        .bind(company_id)
        .bind(status.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, company_id, proposed_by_person_id, employee_display_name,
                    role_type, specialty, ai_profile_id, rationale, scope_of_work,
                    status, founder_response_text, created_person_id, created_at, updated_at
             FROM hiring_proposals
             WHERE company_id = $1
             ORDER BY created_at DESC",
        )
        .bind(company_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.iter().map(row_to_proposal).collect())
}

pub async fn get_proposal(
    pool: &PgPool,
    company_id: Uuid,
    proposal_id: Uuid,
) -> Result<Option<HiringProposal>> {
    let row = sqlx::query(
        "SELECT id, company_id, proposed_by_person_id, employee_display_name,
                role_type, specialty, ai_profile_id, rationale, scope_of_work,
                status, founder_response_text, created_person_id, created_at, updated_at
         FROM hiring_proposals
         WHERE id = $1 AND company_id = $2",
    )
    .bind(proposal_id)
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_proposal))
}

pub async fn create_proposal(
    pool: &PgPool,
    company_id: Uuid,
    input: CreateProposalInput,
) -> Result<HiringProposal> {
    if input.employee_display_name.trim().is_empty() {
        return Err(anyhow!("employee_display_name is required"));
    }
    // Validate role_type
    input
        .role_type
        .parse::<RoleType>()
        .map_err(|e| anyhow!("invalid role_type: {e}"))?;

    let row = sqlx::query(
        "INSERT INTO hiring_proposals (
            company_id, proposed_by_person_id, employee_display_name,
            role_type, specialty, ai_profile_id, rationale, scope_of_work
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, company_id, proposed_by_person_id, employee_display_name,
                   role_type, specialty, ai_profile_id, rationale, scope_of_work,
                   status, founder_response_text, created_person_id, created_at, updated_at",
    )
    .bind(company_id)
    .bind(input.proposed_by_person_id)
    .bind(input.employee_display_name.trim())
    .bind(&input.role_type)
    .bind(&input.specialty)
    .bind(input.ai_profile_id)
    .bind(&input.rationale)
    .bind(&input.scope_of_work)
    .fetch_one(pool)
    .await?;

    Ok(row_to_proposal(&row))
}

/// Accept a hiring proposal: creates a new `Person` and links it back.
/// Runs in a transaction so both updates succeed or neither does.
pub async fn accept_proposal(
    pool: &PgPool,
    company_id: Uuid,
    proposal_id: Uuid,
    input: AcceptProposalInput,
) -> Result<HiringProposal> {
    let mut tx = pool.begin().await?;

    // Lock and fetch the proposal
    let prop_row = sqlx::query(
        "SELECT id, company_id, proposed_by_person_id, employee_display_name,
                role_type, specialty, ai_profile_id, rationale, scope_of_work,
                status, founder_response_text, created_person_id, created_at, updated_at
         FROM hiring_proposals
         WHERE id = $1 AND company_id = $2
         FOR UPDATE",
    )
    .bind(proposal_id)
    .bind(company_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("proposal not found"))?;

    let proposal = row_to_proposal(&prop_row);

    if proposal.status != ProposalStatus::PendingFounder {
        return Err(anyhow!(
            "cannot accept a proposal with status '{}'",
            proposal.status
        ));
    }

    // Create the new Person
    let role_type = proposal
        .role_type
        .parse::<RoleType>()
        .unwrap_or(RoleType::Specialist);

    let person_row = sqlx::query(
        "INSERT INTO people (company_id, kind, display_name, role_type, specialty, ai_profile_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(company_id)
    .bind(PersonKind::AiAgent.to_string())
    .bind(&proposal.employee_display_name)
    .bind(role_type.to_string())
    .bind(&proposal.specialty)
    .bind(proposal.ai_profile_id)
    .fetch_one(&mut *tx)
    .await?;

    let new_person_id: Uuid = person_row.get("id");

    // Update the proposal
    let updated_row = sqlx::query(
        "UPDATE hiring_proposals
         SET status = 'accepted',
             founder_response_text = $3,
             created_person_id = $4,
             updated_at = NOW()
         WHERE id = $1 AND company_id = $2
         RETURNING id, company_id, proposed_by_person_id, employee_display_name,
                   role_type, specialty, ai_profile_id, rationale, scope_of_work,
                   status, founder_response_text, created_person_id, created_at, updated_at",
    )
    .bind(proposal_id)
    .bind(company_id)
    .bind(&input.founder_response_text)
    .bind(new_person_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(row_to_proposal(&updated_row))
}

/// Decline a hiring proposal. Reason is required.
pub async fn decline_proposal(
    pool: &PgPool,
    company_id: Uuid,
    proposal_id: Uuid,
    input: DeclineProposalInput,
) -> Result<HiringProposal> {
    if input.founder_response_text.trim().is_empty() {
        return Err(anyhow!("founder_response_text (reason) is required when declining"));
    }

    let row = sqlx::query(
        "UPDATE hiring_proposals
         SET status = 'declined',
             founder_response_text = $3,
             updated_at = NOW()
         WHERE id = $1 AND company_id = $2 AND status = 'pending_founder'
         RETURNING id, company_id, proposed_by_person_id, employee_display_name,
                   role_type, specialty, ai_profile_id, rationale, scope_of_work,
                   status, founder_response_text, created_person_id, created_at, updated_at",
    )
    .bind(proposal_id)
    .bind(company_id)
    .bind(input.founder_response_text.trim())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("proposal not found or not in pending_founder status"))?;

    Ok(row_to_proposal(&row))
}

/// Remove a pending hiring proposal (dismiss without accept/decline).
pub async fn delete_proposal(
    pool: &PgPool,
    company_id: Uuid,
    proposal_id: Uuid,
) -> Result<bool> {
    let r = sqlx::query(
        "DELETE FROM hiring_proposals
         WHERE id = $1 AND company_id = $2 AND status = $3",
    )
    .bind(proposal_id)
    .bind(company_id)
    .bind(ProposalStatus::PendingFounder.to_string())
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}
