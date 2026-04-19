//! When a simulation starts and the company has no tickets yet, create a single
//! kickoff ticket for the AI co-founder so agents run without manual ticket creation.

use anyhow::Result;
use domain::{
    CreateTicketInput, Ticket, TicketPriority, TicketStatus, TicketType,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// If the company has **no** tickets across any workspace, create one high-priority
/// epic assigned to the first AI co-founder (with profile), in the earliest workspace.
pub async fn ensure_first_simulation_ticket(pool: &PgPool, company_id: Uuid) -> Result<Option<Ticket>> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM tickets t
         INNER JOIN workspaces w ON w.id = t.workspace_id
         WHERE w.company_id = $1",
    )
    .bind(company_id)
    .fetch_one(pool)
    .await?;

    if count > 0 {
        return Ok(None);
    }

    let cofounder_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM people
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

    let Some(cofounder_id) = cofounder_id else {
        return Ok(None);
    };

    let workspace_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM workspaces
         WHERE company_id = $1
         ORDER BY created_at ASC
         LIMIT 1",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };

    let prod_row = sqlx::query(
        "SELECT name, description FROM products
         WHERE company_id = $1
         ORDER BY created_at ASC
         LIMIT 1",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let (title, description, definition_of_done) = if let Some(r) = prod_row {
        let pname: String = r.get("name");
        let pdesc: Option<String> = r.get("description");
        let title = format!("Kickoff: {pname} — first milestones");
        let mut desc = String::from(
            "The simulation started with no work items yet — this ticket was created automatically. \
             Use the founder's product vision (below) and company context to plan concrete next steps.",
        );
        if let Some(d) = pdesc.filter(|s| !s.trim().is_empty()) {
            desc.push_str("\n\n**Founder product idea (from onboarding):**\n\n");
            desc.push_str(d.trim());
        }
        let dod = "- You have internalized the product idea and company/agent memory.\n\
                   - You have posted a **plan** in comments (phases, needed functions).\n\
                   - You have **`propose_hire`**’d a minimal org (e.g. CEO, CTO, key specialists) with correct **`workspace_ids`**, and created **delegated** tickets/subtasks in the right workspaces with **`assignee_person_id`** set — you orchestrate; hires execute.\n\
                   - The path forward is clear without the founder doing IC work for every function."
            .to_string();
        (title, Some(desc), Some(dod))
    } else {
        (
            "Kickoff — define the first milestones".to_string(),
            Some(
                "The simulation started with no work items yet — this ticket was created automatically. \
                 Review company context and capture what the team should do next."
                    .to_string(),
            ),
            Some(
                "- Plan in comments; hire org with workspace placement; delegate execution to the right assignees."
                    .to_string(),
            ),
        )
    };

    let ticket = crate::ticket::create_ticket(
        pool,
        workspace_id,
        CreateTicketInput {
            title,
            description,
            definition_of_done,
            founder_memory: None,
            outcome_summary: None,
            ticket_type: Some(TicketType::Epic),
            status: Some(TicketStatus::Todo),
            priority: Some(TicketPriority::High),
            assignee_person_id: Some(cofounder_id),
            parent_ticket_id: None,
        },
    )
    .await?;

    Ok(Some(ticket))
}
