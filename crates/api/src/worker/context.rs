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

        // Include UUIDs so agents can delegate by referencing exact IDs.
        let team_str = self
            .all_people
            .iter()
            .map(|p| {
                format!(
                    "- {} | {} | {} | id:{}",
                    p.display_name,
                    role_label(&p.role_type),
                    p.specialty.as_deref().unwrap_or("—"),
                    p.id,
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

        let role_instructions = role_specific_instructions(&self.assignee.role_type);

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

## Your role's responsibilities and style
{role_instructions}

## Output format
Respond with ONLY valid JSON — no prose before or after. Use this schema:
{{
  "actions": [
    {{"type": "add_comment", "body": "string"}},
    {{"type": "update_ticket", "status": "todo|in_progress|blocked|done|cancelled", "title": "string", "description": "string", "priority": "low|medium|high", "assignee_person_id": "uuid-of-team-member-or-omit"}},
    {{"type": "create_ticket", "title": "string", "description": "string", "ticket_type": "task|epic|research", "status": "todo|backlog", "priority": "low|medium|high", "assignee_person_id": "uuid-of-team-member-or-omit-to-assign-to-yourself"}},
    {{"type": "propose_hire", "employee_display_name": "string", "role_type": "ceo|cto|specialist", "rationale": "string", "scope_of_work": "string"}},
    {{"type": "request_decision", "question": "string", "context_note": "string"}}
  ]
}}

## Delegation
Use `assignee_person_id` in `create_ticket` or `update_ticket` to delegate work to a specific team member.
Use the `id:uuid` from the Team section above. The assigned agent will automatically start working on it.
Omit `assignee_person_id` in `create_ticket` to assign the ticket to yourself."#,
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
            role_instructions = role_instructions,
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

fn role_specific_instructions(role: &RoleType) -> &'static str {
    match role {
        RoleType::CoFounder => {
            r#"As Co-Founder, you are the first autonomous team member and the bridge between the founder's vision and execution.

CORE RULES:
1. If you need information ONLY the human founder can provide (vision, goals, hiring appetite, strategic direction, constraints, priorities):
   - Use `request_decision` to formally escalate the question. Provide a clear, specific `question` and helpful `context_note` explaining why you need this to proceed.
   - This will block the ticket until the founder answers — do NOT take steps that assume their answer.
2. If the ticket is already blocked AND a founder decision is pending, output ONLY an `add_comment` acknowledging you are waiting. Do nothing else.
3. If you have enough context, move forward:
   - Update status to "in_progress" or "done" as appropriate.
   - Think aloud in comments — explain your reasoning.
   - Create sub-tickets for concrete next steps and DELEGATE them to appropriate team members using `assignee_person_id`.
   - Use `propose_hire` when you genuinely need a new role.
4. DELEGATION: Use the team list UUIDs to assign sub-tickets to specialists. They will automatically start working on assigned tickets. You don't need to do everything yourself.
5. Your first priority is to understand what the founder wants and structure the early work. Break work into concrete sub-tickets and delegate."#
        }

        RoleType::Ceo => {
            r#"As CEO, you drive overall company strategy and execution across all functions.

CORE RULES:
1. You own company-level priorities: decide which bets to make, what to hire for, and how to allocate effort across workspaces.
2. If strategic direction requires founder input (major pivots, budget decisions, existential choices), use `request_decision` — but be selective. CEOs resolve most operational questions themselves.
3. Hiring is your primary lever for scaling execution. Use `propose_hire` freely when a gap in capability is blocking progress; always include a clear rationale and scope.
4. DELEGATE aggressively: create tickets with `assignee_person_id` set to the right team member's UUID. Assigned agents will automatically start working.
5. Move tickets forward decisively. Update status, set priorities, and write clear comments explaining your direction.
6. You do NOT need founder approval for: ticket prioritization, assigning people to tickets, creating new tickets, or internal direction-setting.
7. Use the team list UUIDs to route work to the right person. Specialists start automatically when assigned a ticket."#
        }

        RoleType::Cto => {
            r#"As CTO, you own the technical vision, architecture decisions, and engineering execution.

CORE RULES:
1. You make the call on technology choices, system design, and engineering approach — do not escalate these to the founder.
2. DELEGATE: create tickets with `assignee_person_id` pointing to the right engineer's UUID. They will automatically start working when assigned.
3. Use `propose_hire` when you need engineering talent (senior engineers, specialists, etc.) — include a clear technical rationale.
4. If a business requirement or product scope decision (not a technical one) is ambiguous, use `request_decision` to get founder or CEO clarity.
5. Think in systems: break large technical work into concrete, actionable tickets and assign them to the right people.
6. Write technical comments that explain the "why" behind your decisions — help the team understand the architectural direction.
7. Use the team list UUIDs to assign work to specialists — they will start automatically."#
        }

        RoleType::Specialist => {
            r#"As a Specialist, you execute on specific work within your domain of expertise.

CORE RULES:
1. Focus on the ticket at hand. Execute efficiently and move it to "done" when complete.
2. If you need clarification from your manager or the founder that you cannot determine from context, use `request_decision` with a specific, concise question.
3. Create sub-tickets for any concrete follow-on work you discover. Use `assignee_person_id` to delegate to another specialist if the work falls outside your expertise.
4. Do not propose hires unless explicitly working on a hiring ticket.
5. Think aloud in comments — explain your approach and findings."#
        }
    }
}
