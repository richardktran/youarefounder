//! Context pack builder — assembles everything the agent needs to reason about
//! a ticket: company, workspace, ticket, thread, people, and org hierarchy.

use anyhow::{Context, Result};
use domain::{Person, ProductBrainEntry, RoleType, Ticket, TicketComment};
use sqlx::PgPool;
use uuid::Uuid;

/// Caps for prompt size (product brain + referenced tickets).
const MAX_BRAIN_ENTRIES: i64 = 40;
const MAX_BRAIN_SECTION_CHARS: usize = 8_000;
const MAX_REFERENCED_TICKETS: usize = 8;
const MAX_BRAIN_PER_REF_TICKET: i64 = 6;
const MAX_COMMENTS_PER_REF: usize = 14;
const MAX_COMMENT_SNIPPET_CHARS: usize = 1_200;

pub struct ContextPack {
    pub company_name: String,
    pub company_id: Uuid,
    pub product_name: Option<String>,
    pub workspace_name: String,
    /// Company-wide founder instructions for ticket work (delegation, priorities, tone).
    pub agent_ticket_memory: Option<String>,
    /// Company-wide founder instructions for escalations and decisions.
    pub agent_decision_memory: Option<String>,
    pub ticket: Ticket,
    /// Direct child tickets (subtasks) of the current ticket.
    pub subtasks: Vec<Ticket>,
    pub comments: Vec<TicketComment>,
    /// Approved product brain (company + workspace scope), for prompt injection.
    pub approved_brain_entries: Vec<ProductBrainEntry>,
    /// Explicit cross-ticket links from this ticket (`ticket_references`).
    pub referenced_ticket_snapshots: String,
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

        let subtasks = db::ticket::list_tickets(
            pool,
            ticket.workspace_id,
            db::ticket::TicketListFilter::ChildrenOf(ticket.id),
        )
        .await
        .context("load subtasks")?;

        let approved_brain_entries = db::product_brain::list_approved_for_context(
            pool,
            company.id,
            ticket.workspace_id,
            MAX_BRAIN_ENTRIES,
        )
        .await
        .context("load product brain")?;

        let referenced_ticket_snapshots =
            build_referenced_ticket_snapshots(pool, company.id, ticket.id)
                .await
                .context("load referenced tickets")?;

        Ok(Self {
            company_name: company.name,
            company_id: company.id,
            product_name,
            workspace_name: workspace.name,
            agent_ticket_memory: company.agent_ticket_memory.clone(),
            agent_decision_memory: company.agent_decision_memory.clone(),
            ticket,
            subtasks,
            comments,
            approved_brain_entries,
            referenced_ticket_snapshots,
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

        let subtasks_str = if self.subtasks.is_empty() {
            if self.ticket.parent_ticket_id.is_some() {
                "(none — this ticket is already a subtask; further subtasks are not allowed.)".to_string()
            } else {
                "(none yet — use `create_subtask` to break work into concrete steps.)".to_string()
            }
        } else {
            self.subtasks
                .iter()
                .map(|t| {
                    format!(
                        "- [{}] {} (id: {})",
                        t.status, t.title, t.id
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let ticket_tree_str = if let Some(pid) = self.ticket.parent_ticket_id {
            format!(
                "This ticket is a **subtask** (parent ticket id: {pid}). Do **not** use `create_subtask` — only top-level tickets may have subtasks. Use `add_comment` for a checklist or `create_ticket` for new top-level work."
            )
        } else {
            "This ticket is **top-level** — you may use `create_subtask` for direct children (**one level only**; those children cannot have their own subtasks)."
                .to_string()
        };

        let dod_str = self
            .ticket
            .definition_of_done
            .as_deref()
            .unwrap_or("(not set yet — use `update_ticket` with `definition_of_done`.)");

        let ticket_mem_str = self
            .agent_ticket_memory
            .as_deref()
            .unwrap_or("(none — founder can set company-wide ticket memory in Settings.)");
        let decision_mem_str = self
            .agent_decision_memory
            .as_deref()
            .unwrap_or("(none — founder can set decision memory in Settings.)");
        let founder_ticket_mem_str = self
            .ticket
            .founder_memory
            .as_deref()
            .unwrap_or("(none — founder can add per-ticket memory on the ticket page.)");

        let brain_section = format_approved_brain_section(&self.approved_brain_entries);
        let ref_section = if self.referenced_ticket_snapshots.trim().is_empty() {
            "(none — use `add_ticket_reference` with another ticket's id to load its outcome into context.)"
                .to_string()
        } else {
            self.referenced_ticket_snapshots.clone()
        };

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

## Founder memory (mandatory — follow in every action and comment)
The founder wrote the following. Treat it as binding operating policy for this company. If it conflicts with generic role advice below, **follow founder memory**. Do not ignore it because it is repetitive or verbose.

### Company — tickets (all work items)
{ticket_mem}

### Company — decisions and escalations
When you use `request_decision`, frame questions consistent with this. When interpreting founder replies in the thread, align with this.
{decision_mem}

### This ticket only (highest priority for this ticket)
{founder_ticket_mem}

## Product brain (approved — founder-reviewed persistent knowledge)
These entries were approved by the founder. Treat them as durable product/company context alongside founder memory above.
{brain_section}

## Referenced tickets (read-only snapshots)
Tickets explicitly linked from this one. You do not re-run them; you recall what they concluded. To add a link, use `add_ticket_reference`.
{ref_section}

## Team
{team}

## Current ticket
- Workspace: {workspace}
- Title: {title}
- Status: {status}
- Priority: {priority}
- Ticket tree: {ticket_tree}
- Definition of done (must be satisfied before you set status to done): {dod}
- Description: {description}

## Subtasks of this ticket (children only)
{subtasks}

## Ticket thread
{comments}

## Your role's responsibilities and style
{role_instructions}

## Output format
Respond with ONLY valid JSON — no prose before or after. Use this schema:
{{
  "actions": [
    {{"type": "add_comment", "body": "string"}},
    {{"type": "update_ticket", "status": "todo|in_progress|blocked|done|cancelled", "title": "string", "description": "string", "definition_of_done": "string — bullet list of what must be true to mark THIS ticket done", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit"}},
    {{"type": "create_subtask", "title": "string", "description": "string", "definition_of_done": "string", "status": "todo|in_progress|backlog", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit-for-self"}},
    {{"type": "create_ticket", "title": "string", "description": "string", "definition_of_done": "string", "ticket_type": "task|epic|research", "status": "todo|backlog", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit", "workspace_id": "uuid-or-omit"}},
    {{"type": "propose_hire", "employee_display_name": "string", "role_type": "ceo|cto|specialist", "rationale": "string", "scope_of_work": "string"}},
    {{"type": "request_decision", "question": "string", "context_note": "string"}},
    {{"type": "add_ticket_reference", "to_ticket_id": "uuid", "note": "string-or-omit"}},
    {{"type": "remove_ticket_reference", "to_ticket_id": "uuid"}},
    {{"type": "propose_brain_insight", "summary": "string", "detail": "string-or-omit"}}
  ]
}}

## Work breakdown (important)
- **Only top-level tickets** may use `create_subtask`. If this ticket is already a subtask, do not create nested subtasks — see "Ticket tree" above.
- To split a **top-level** ticket into steps, use `create_subtask` (not `create_ticket`). Subtasks are linked only to that parent ticket.
- When **all** subtasks are done or cancelled, the parent ticket can auto-complete to done (unless the parent is blocked on a founder decision).
- Set `definition_of_done` early: before marking **done**, every bullet in definition of done must be satisfied (or explain in a comment why the ticket is cancelled instead).
- Use `create_ticket` only for genuinely **new** top-level work in the workspace (e.g. a separate initiative), not for decomposing the current ticket.

## Delegation
Use `assignee_person_id` in `create_subtask`, `create_ticket`, or `update_ticket` to delegate.
Use the `id:uuid` from the Team section above. The assigned agent will automatically start working on it.
Omit `assignee_person_id` to assign the new ticket/subtask to yourself."#,
            name = self.assignee.display_name,
            role = assignee_role,
            company = self.company_name,
            product = product_str,
            manager = manager_str,
            reports = reports_str,
            ticket_mem = ticket_mem_str,
            decision_mem = decision_mem_str,
            founder_ticket_mem = founder_ticket_mem_str,
            brain_section = brain_section,
            ref_section = ref_section,
            team = team_str,
            workspace = self.workspace_name,
            title = self.ticket.title,
            status = self.ticket.status,
            priority = self.ticket.priority,
            ticket_tree = ticket_tree_str,
            dod = dod_str,
            description = self
                .ticket
                .description
                .as_deref()
                .unwrap_or("(no description)"),
            subtasks = subtasks_str,
            comments = comments_str,
            role_instructions = role_instructions,
        )
    }
}

fn format_approved_brain_section(entries: &[ProductBrainEntry]) -> String {
    if entries.is_empty() {
        return "(none yet — approved entries appear after the founder promotes items from the review queue in Settings.)"
            .to_string();
    }
    let mut total = 0usize;
    let mut s = String::new();
    for e in entries {
        let body = e.body.trim();
        let block = format!("- {body}\n");
        if total + block.len() > MAX_BRAIN_SECTION_CHARS {
            s.push_str("…(further brain entries omitted for length)\n");
            break;
        }
        s.push_str(&block);
        total += block.len();
    }
    s
}

async fn build_referenced_ticket_snapshots(
    pool: &PgPool,
    company_id: Uuid,
    from_ticket_id: Uuid,
) -> Result<String> {
    let refs = db::product_brain::list_references_from(pool, from_ticket_id).await?;
    let mut out = String::new();
    for r in refs.into_iter().take(MAX_REFERENCED_TICKETS) {
        let Some(t) = db::ticket::get_ticket(pool, r.to_ticket_id).await? else {
            continue;
        };
        let brain = db::product_brain::list_entries_by_source_ticket(
            pool,
            company_id,
            t.id,
            MAX_BRAIN_PER_REF_TICKET,
        )
        .await?;
        let comments = db::ticket::list_comments(pool, t.id).await?;
        out.push_str(&format_referenced_snapshot(
            &t,
            &brain,
            &comments,
            r.note.as_deref(),
        ));
    }
    if out.len() > 24_000 {
        out.truncate(24_000);
        out.push_str("\n…(referenced ticket section truncated)\n");
    }
    Ok(out)
}

fn format_referenced_snapshot(
    t: &Ticket,
    brain: &[ProductBrainEntry],
    comments: &[TicketComment],
    note: Option<&str>,
) -> String {
    let mut s = format!(
        "### Ticket {} — {}\n- Status: {}\n",
        t.id, t.title, t.status
    );
    if let Some(note) = note {
        if !note.is_empty() {
            s.push_str(&format!("- Link note: {note}\n"));
        }
    }
    if let Some(ref dod) = t.definition_of_done {
        if !dod.is_empty() {
            s.push_str(&format!("- Definition of done:\n{dod}\n"));
        }
    }
    if let Some(ref o) = t.outcome_summary {
        if !o.is_empty() {
            s.push_str(&format!("- Outcome summary:\n{o}\n"));
        }
    }
    if !brain.is_empty() {
        s.push_str("- Approved brain tied to this ticket:\n");
        for e in brain {
            let b = e.body.trim();
            let excerpt: &str = if b.len() > 2_000 {
                &b[..2000]
            } else {
                b
            };
            s.push_str(&format!("  - {excerpt}\n"));
        }
    }
    s.push_str("- Recent thread (excerpt):\n");
    let tail: Vec<_> = comments.iter().rev().take(MAX_COMMENTS_PER_REF).collect();
    for c in tail.iter().rev() {
        let line = c.body.trim();
        let snippet: &str = if line.len() > MAX_COMMENT_SNIPPET_CHARS {
            &line[..MAX_COMMENT_SNIPPET_CHARS]
        } else {
            line
        };
        s.push_str(&format!("  - {snippet}\n"));
    }
    s.push('\n');
    s
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
   - This will block THIS ticket until the founder answers — do NOT take steps that assume their answer.
2. If THIS ticket is blocked on a pending founder decision, do not spin uselessly: one short `add_comment` if there is nothing new to say. You may still complete subtasks or work assigned on other tickets that do not depend on that answer — but do not mark THIS ticket done until the decision exists and definition of done is met.
3. If you have enough context, move forward:
   - Set `definition_of_done` when it is missing so everyone agrees how this ticket closes.
   - On a **top-level** ticket only, break down with `create_subtask` (not `create_ticket`). Delegate subtasks via `assignee_person_id`. If you are already on a subtask, use comments or top-level `create_ticket` instead of nested subtasks.
   - Update status to "in_progress" when executing; set to "done" only when definition of done is satisfied (or "cancelled" with a reason in comments).
   - Think aloud in comments — explain your reasoning.
   - Use `propose_hire` when you genuinely need a new role.
4. DELEGATION: Use the team list UUIDs. Assigned agents start automatically on subtasks.
5. Prefer measurable outcomes: definition of done should be checkable bullets, not vague goals."#
        }

        RoleType::Ceo => {
            r#"As CEO, you drive overall company strategy and execution across all functions.

CORE RULES:
1. You own company-level priorities: decide which bets to make, what to hire for, and how to allocate effort across workspaces.
2. If strategic direction requires founder input (major pivots, budget decisions, existential choices), use `request_decision` — but be selective. CEOs resolve most operational questions themselves.
3. Hiring is your primary lever for scaling execution. Use `propose_hire` freely when a gap in capability is blocking progress; always include a clear rationale and scope.
4. DELEGATE aggressively: on **top-level** tickets use `create_subtask` for direct children; use `create_ticket` only for separate initiatives. Assign with `assignee_person_id`.
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
5. Think in systems: on **top-level** tickets, break large technical work into `create_subtask` steps and assign them to the right people.
6. Write technical comments that explain the "why" behind your decisions — help the team understand the architectural direction.
7. Use the team list UUIDs to assign work to specialists — they will start automatically."#
        }

        RoleType::Specialist => {
            r#"As a Specialist, you execute on specific work within your domain of expertise.

CORE RULES:
1. Focus on the ticket at hand. Execute efficiently and move it to "done" when complete.
2. If you need clarification from your manager or the founder that you cannot determine from context, use `request_decision` with a specific, concise question.
3. On a **top-level** ticket only, use `create_subtask` for follow-on work under this ticket. Use `assignee_person_id` to delegate. Set `definition_of_done` if missing. Mark "done" only when that definition is met. Do not nest subtasks under a subtask.
4. Do not propose hires unless explicitly working on a hiring ticket.
5. Think aloud in comments — explain your approach and findings."#
        }
    }
}
