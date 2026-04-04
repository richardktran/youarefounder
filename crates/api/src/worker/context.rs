//! Context pack builder — assembles everything the agent needs to reason about
//! a ticket: company, workspace, ticket, thread, people, and org hierarchy.

use anyhow::{Context, Result};
use domain::{Person, RoleType, Ticket, TicketComment};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ContextPack {
    pub company_name: String,
    pub product_name: Option<String>,
    pub workspace_name: String,
    pub ticket: Ticket,
    pub comments: Vec<TicketComment>,
    pub assignee: Person,
    pub manager: Option<Person>,
    pub direct_reports: Vec<Person>,
    pub all_people: Vec<Person>,
}

impl ContextPack {
    pub async fn build(pool: &PgPool, ticket_id: Uuid, person_id: Uuid) -> Result<Self> {
        // Load ticket
        let ticket = db::ticket::get_ticket(pool, ticket_id)
            .await
            .context("load ticket")?
            .context("ticket not found")?;

        // Load workspace
        let workspace = db::workspace::get_workspace(pool, ticket.workspace_id)
            .await
            .context("load workspace")?
            .context("workspace not found")?;

        // Load company
        let company = db::company::get_company(pool, workspace.company_id)
            .await
            .context("load company")?
            .context("company not found")?;

        // Load products (take first for context)
        let products = db::product::list_products(pool, company.id)
            .await
            .context("load products")?;
        let product_name = products.first().map(|p| p.name.clone());

        // Load all people in the company
        let all_people = db::person::list_people(pool, company.id)
            .await
            .context("load people")?;

        // Resolve assignee
        let assignee = all_people
            .iter()
            .find(|p| p.id == person_id)
            .cloned()
            .context("assignee person not found")?;

        // Resolve manager and direct reports from org chart
        let manager = assignee
            .reports_to_person_id
            .and_then(|mgr_id| all_people.iter().find(|p| p.id == mgr_id).cloned());

        let direct_reports: Vec<Person> = all_people
            .iter()
            .filter(|p| p.reports_to_person_id == Some(person_id))
            .cloned()
            .collect();

        // Load comments
        let comments = db::ticket::list_comments(pool, ticket_id)
            .await
            .context("load comments")?;

        Ok(Self {
            company_name: company.name,
            product_name,
            workspace_name: workspace.name,
            ticket,
            comments,
            assignee,
            manager,
            direct_reports,
            all_people,
        })
    }

    /// Render the full system + user prompt for the LLM.
    pub fn build_prompt(&self) -> String {
        let assignee_role = role_label(&self.assignee.role_type);
        let manager_str = self
            .manager
            .as_ref()
            .map(|m| format!("{} ({})", m.display_name, role_label(&m.role_type)))
            .unwrap_or_else(|| "none (you report directly to the founder)".to_string());

        let reports_str = if self.direct_reports.is_empty() {
            "none".to_string()
        } else {
            self.direct_reports
                .iter()
                .map(|p| format!("- {} ({})", p.display_name, role_label(&p.role_type)))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let team_str = self
            .all_people
            .iter()
            .map(|p| {
                format!(
                    "- {} | {} | {}",
                    p.display_name,
                    role_label(&p.role_type),
                    p.specialty.as_deref().unwrap_or("—")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let comments_str = if self.comments.is_empty() {
            "(no comments yet)".to_string()
        } else {
            self.comments
                .iter()
                .map(|c| {
                    let author = c
                        .author_person_id
                        .and_then(|id| self.all_people.iter().find(|p| p.id == id))
                        .map(|p| p.display_name.as_str())
                        .unwrap_or("Founder");
                    format!("[{}]: {}", author, c.body)
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        let product_str = self
            .product_name
            .as_deref()
            .unwrap_or("(no product set yet)");

        format!(
            r#"You are {name}, the {role} of {company}.

## Company context
- Company: {company}
- Product: {product}
- Your role: {role}
- Your manager: {manager}
- Your direct reports:
{reports}

## Team
{team}

## Current ticket
- Workspace: {workspace}
- Title: {title}
- Status: {status}
- Priority: {priority}
- Description: {description}

## Ticket thread
{comments}

## Instructions
Think carefully about this ticket. You are an autonomous AI agent helping build a real company.

IMPORTANT RULES:
1. If you need information ONLY the human founder can provide (vision, goals, hiring appetite, strategic direction, constraints, priorities), create a question for them:
   - Add a comment explaining what you need and why.
   - Update the ticket status to "blocked" so they know to respond.
   - Do NOT take autonomous steps that assume their answer (no hiring spree, no large ticket batch).
2. If the ticket is already blocked waiting for the founder AND they have not yet responded in the thread, output ONLY an `add_comment` acknowledging you are still waiting and set status to blocked. Do nothing else.
3. If you have enough context to make progress, move the ticket forward:
   - Update status to "in_progress" or "done" as appropriate.
   - Add comments explaining your reasoning ("thinking aloud").
   - Create sub-tickets for concrete next steps.
   - Use `propose_hire` if you genuinely need a new role to accomplish the work.
4. Be realistic. As co-founder, your first job is to understand what the founder wants and help them structure the early work — not to try to do everything autonomously.

## Output format
Respond with ONLY valid JSON — no prose before or after. Use this schema:
{{
  "actions": [
    {{"type": "add_comment", "body": "string"}},
    {{"type": "update_ticket", "status": "todo|in_progress|blocked|done|cancelled", "title": "string", "description": "string", "priority": "low|medium|high"}},
    {{"type": "create_ticket", "title": "string", "description": "string", "ticket_type": "task|epic|research", "status": "todo|backlog", "priority": "low|medium|high"}},
    {{"type": "propose_hire", "employee_display_name": "string", "role_type": "ceo|cto|specialist", "rationale": "string", "scope_of_work": "string"}}
  ]
}}"#,
            name = self.assignee.display_name,
            role = assignee_role,
            company = self.company_name,
            product = product_str,
            manager = manager_str,
            reports = reports_str,
            team = team_str,
            workspace = self.workspace_name,
            title = self.ticket.title,
            status = self.ticket.status,
            priority = self.ticket.priority,
            description = self
                .ticket
                .description
                .as_deref()
                .unwrap_or("(no description)"),
            comments = comments_str,
        )
    }
}

fn role_label(role: &RoleType) -> String {
    match role {
        RoleType::CoFounder => "Co-Founder".to_string(),
        RoleType::Ceo => "CEO".to_string(),
        RoleType::Cto => "CTO".to_string(),
        RoleType::Specialist => "Specialist".to_string(),
    }
}
