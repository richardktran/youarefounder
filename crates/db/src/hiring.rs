use anyhow::{anyhow, Result};
use domain::{
    AcceptProposalInput, CreateProposalInput, DeclineProposalInput, HiringProposal,
    PersonKind, ProposalStatus, RoleType,
};
use sqlx::{postgres::PgRow, PgPool, Row};
use tracing::warn;
use uuid::Uuid;

const AUTO_ACCEPT_NOTE: &str = "Auto-approved — team runs autonomously.";

const HIRING_PROPOSAL_COLS: &str = "id, company_id, proposed_by_person_id, employee_display_name,
            role_type, specialty, ai_profile_id, rationale, scope_of_work, workspace_ids,
            status, founder_response_text, created_person_id, created_at, updated_at";

fn row_to_proposal(row: &PgRow) -> HiringProposal {
    let status_str: String = row.get("status");
    let workspace_ids: Option<Vec<Uuid>> = row.get("workspace_ids");
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
        workspace_ids: workspace_ids.filter(|v| !v.is_empty()),
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
        sqlx::query(&format!(
            "SELECT {HIRING_PROPOSAL_COLS}
             FROM hiring_proposals
             WHERE company_id = $1 AND status = $2
             ORDER BY created_at DESC",
        ))
        .bind(company_id)
        .bind(status.to_string())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(&format!(
            "SELECT {HIRING_PROPOSAL_COLS}
             FROM hiring_proposals
             WHERE company_id = $1
             ORDER BY created_at DESC",
        ))
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
    let row = sqlx::query(&format!(
        "SELECT {HIRING_PROPOSAL_COLS}
         FROM hiring_proposals
         WHERE id = $1 AND company_id = $2",
    ))
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

    let workspace_ids = input
        .workspace_ids
        .as_ref()
        .filter(|v| !v.is_empty())
        .cloned();

    let row = sqlx::query(&format!(
        "INSERT INTO hiring_proposals (
            company_id, proposed_by_person_id, employee_display_name,
            role_type, specialty, ai_profile_id, rationale, scope_of_work, workspace_ids
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING {HIRING_PROPOSAL_COLS}",
    ))
    .bind(company_id)
    .bind(input.proposed_by_person_id)
    .bind(input.employee_display_name.trim())
    .bind(&input.role_type)
    .bind(&input.specialty)
    .bind(input.ai_profile_id)
    .bind(&input.rationale)
    .bind(&input.scope_of_work)
    .bind(workspace_ids)
    .fetch_one(pool)
    .await?;

    Ok(row_to_proposal(&row))
}

/// Create a hiring proposal and immediately accept it when an AI profile is available
/// (explicit `input.ai_profile_id`, else the AI co-founder's profile).
pub async fn create_proposal_auto_accept(
    pool: &PgPool,
    company_id: Uuid,
    mut input: CreateProposalInput,
) -> Result<HiringProposal> {
    if input.ai_profile_id.is_none() {
        input.ai_profile_id =
            crate::person::ai_profile_id_of_ai_co_founder(pool, company_id).await?;
    }

    let proposal = create_proposal(pool, company_id, input).await?;

    if proposal.ai_profile_id.is_none() {
        warn!(
            company_id = %company_id,
            proposal_id = %proposal.id,
            "hiring proposal left pending: no AI profile (add an AI co-founder with a profile)"
        );
        return Ok(proposal);
    }

    accept_proposal(
        pool,
        company_id,
        proposal.id,
        AcceptProposalInput {
            founder_response_text: Some(AUTO_ACCEPT_NOTE.to_string()),
        },
    )
    .await
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
    let prop_row = sqlx::query(&format!(
        "SELECT {HIRING_PROPOSAL_COLS}
         FROM hiring_proposals
         WHERE id = $1 AND company_id = $2
         FOR UPDATE",
    ))
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

    let reports_to_person_id = proposal.proposed_by_person_id;

    let person_row = sqlx::query(
        "INSERT INTO people (company_id, kind, display_name, role_type, specialty, ai_profile_id, reports_to_person_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(company_id)
    .bind(PersonKind::AiAgent.to_string())
    .bind(&proposal.employee_display_name)
    .bind(role_type.to_string())
    .bind(&proposal.specialty)
    .bind(proposal.ai_profile_id)
    .bind(reports_to_person_id)
    .fetch_one(&mut *tx)
    .await?;

    let new_person_id: Uuid = person_row.get("id");

    let workspaces_to_join: Vec<Uuid> = if role_type == RoleType::CoFounder {
        sqlx::query_scalar(
            "SELECT id FROM workspaces WHERE company_id = $1 ORDER BY created_at ASC",
        )
        .bind(company_id)
        .fetch_all(&mut *tx)
        .await?
    } else {
        proposal.workspace_ids.clone().unwrap_or_default()
    };

    for wid in workspaces_to_join {
        let ws_row = sqlx::query("SELECT company_id FROM workspaces WHERE id = $1")
            .bind(wid)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(ws_row) = ws_row else {
            continue;
        };
        if ws_row.get::<Uuid, _>("company_id") != company_id {
            continue;
        }
        sqlx::query(
            "INSERT INTO workspace_members (workspace_id, person_id, role)
             VALUES ($1, $2, 'member')
             ON CONFLICT (workspace_id, person_id) DO NOTHING",
        )
        .bind(wid)
        .bind(new_person_id)
        .execute(&mut *tx)
        .await?;
    }

    // Update the proposal
    let updated_row = sqlx::query(&format!(
        "UPDATE hiring_proposals
         SET status = 'accepted',
             founder_response_text = $3,
             created_person_id = $4,
             updated_at = NOW()
         WHERE id = $1 AND company_id = $2
         RETURNING {HIRING_PROPOSAL_COLS}",
    ))
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

    let row = sqlx::query(&format!(
        "UPDATE hiring_proposals
         SET status = 'declined',
             founder_response_text = $3,
             updated_at = NOW()
         WHERE id = $1 AND company_id = $2 AND status = 'pending_founder'
         RETURNING {HIRING_PROPOSAL_COLS}",
    ))
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
