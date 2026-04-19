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
/// Character cap for the onboarding product idea in the prompt (not byte cap).
const MAX_PRODUCT_IDEA_CHARS: usize = 12_000;

pub struct ContextPack {
    pub company_name: String,
    pub company_id: Uuid,
    pub product_name: Option<String>,
    /// First product's description (e.g. onboarding "idea") — baseline company/product vision.
    pub product_idea: Option<String>,
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
    /// All company workspaces (name + id) for `propose_hire.workspace_ids` and routing.
    pub company_workspaces_summary: String,
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
        let first_product = products.first();
        let product_name = first_product.map(|p| p.name.clone());
        let product_idea = first_product
            .and_then(|p| p.description.as_ref())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

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

        let workspaces = db::workspace::list_workspaces(pool, company.id)
            .await
            .context("load workspaces")?;
        let company_workspaces_summary = if workspaces.is_empty() {
            "(none yet)".to_string()
        } else {
            workspaces
                .iter()
                .map(|w| format!("- {} — id: {}", w.name, w.id))
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(Self {
            company_name: company.name,
            company_id: company.id,
            product_name,
            product_idea,
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
            company_workspaces_summary,
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

        let product_idea_str = self
            .product_idea
            .as_deref()
            .map(|idea| {
                let n = idea.chars().count();
                if n > MAX_PRODUCT_IDEA_CHARS {
                    let head: String = idea.chars().take(MAX_PRODUCT_IDEA_CHARS).collect();
                    format!("{head}\n…(product idea truncated for length)")
                } else {
                    idea.to_string()
                }
            })
            .unwrap_or_else(|| {
                "(none — no product description was saved for the first product.)".to_string()
            });

        let subtasks_str = if self.subtasks.is_empty() {
            if self.ticket.parent_ticket_id.is_some() {
                "(none — this ticket is already a subtask; further subtasks are not allowed.)".to_string()
            } else {
                "(none — track your plan and progress with `add_comment` on this ticket; avoid extra subtasks unless another agent must own a separate piece.)"
                    .to_string()
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
                "This ticket is a **subtask** (parent ticket id: {pid}). Do **not** use `create_subtask`. Log progress on **this** ticket with `add_comment`. Use `create_ticket` only for a **separate** initiative, not more decomposition."
            )
        } else {
            "This ticket is **top-level**. Prefer `add_comment` for plans, steps, and status — do **not** spawn many subtasks for one thread of work. Use `create_subtask` only when a **different** assignee must own a clearly separate deliverable (rare)."
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

        let output_format_note = match &self.assignee.role_type {
            RoleType::CoFounder => {
                r#"**Co-Founder — orchestrate, do not absorb all execution:** respond with JSON only. Use `add_comment` for your **plan** (product phases, org design, risks). Use **`propose_hire`** (with **`workspace_ids`**) only for your **direct executive reports**: **at most one CEO, one CTO, and one CFO** for the whole company — **not** specialists or ICs. If **Team** already lists an executive seat, **never** `propose_hire` that role again; assign work to them so **they** hire and delegate. Then **`create_ticket`** or **`create_subtask`** in the correct **`workspace_id`** and set **`assignee_person_id`** to the teammate who should **do** the work."#
            }
            _ => {
                r#"**Default to `add_comment`** for almost everything: thinking, plans, checklists, decisions, and progress on work **you** own. Use `create_subtask` / `create_ticket` sparingly (see Work breakdown)."#
            }
        };

        let work_breakdown_lead = match &self.assignee.role_type {
            RoleType::CoFounder => {
                r#"- **Co-founder sequence:** (1) **`add_comment`** — publish a concrete **product & execution plan** (what to build, phases, which functions matter). (2) **`propose_hire`** — add **only** missing **executives** (CEO, CTO, CFO): one seat each, always set **`workspace_ids`**. **Do not** `propose_hire` **specialists** yourself — CEOs/CTOs/CFOs hire ICs under them as work unfolds. **Do not** duplicate executive seats. (3) **Delegate** — `create_ticket` / `create_subtask` in the **matching `workspace_id`** with **`assignee_person_id`** = the executive or specialist who should **own delivery**. (4) If **Team** lists only you as an AI agent, **hire executives first** before deep IC work. (5) If you are assignee on work meant for someone else, **`update_ticket`** to reassign + comment handoff."#
            }
            _ => {
                r#"- **Tickets you own:** drive the narrative with **`add_comment`**; avoid ticket spam (see below)."#
            }
        };

        let delegation_lead = match &self.assignee.role_type {
            RoleType::CoFounder => {
                r#"**Assign by function:** match **workspace** to the kind of work (see **Company workspaces**) and set **`assignee_person_id`** to the **CEO, CTO, CFO, or specialist** who should execute. You stand up the executive layer and delegate; **do not** personally complete every function’s IC work once executives exist — they grow the team under them. For planning-only work, you may remain assignee on **orchestration** tickets; execution tickets go to the right leader or their reports."#
            }
            _ => {
                r#"If someone else should **take over this same ticket**, use `update_ticket` with `assignee_person_id` and explain in `add_comment`."#
            }
        };

        format!(
            r#"You are {name}, the {role} of {company}.

## Company context
- Company: {company}
- Product: {product}
- Product idea (baseline from the first product — treat as the founder's starting vision for the company):
{product_idea}

## Company workspaces
AI co-founders are members of every workspace. For other hires, set `workspace_ids` on `propose_hire` using ids below (often include the current ticket's workspace).
{workspaces}

- Your role: {role}
- Your manager: {manager}
- Your direct reports:
{reports}

## Founder memory (mandatory — follow in every action and comment)
The founder wrote the following. Treat it as binding operating policy for this company. If it conflicts with generic role advice below, **follow founder memory**. Do not ignore it because it is repetitive or verbose.

### Company — tickets (all work items)
{ticket_mem}

### Company — operating preferences (formerly decision / escalation memory)
There is no founder decision queue: resolve questions within the team using this guidance and `add_comment` to record rationale.
{decision_mem}

### This ticket only (highest priority for this ticket)
{founder_ticket_mem}

## Product brain (approved persistent knowledge)
Treat these entries as durable product/company context alongside founder memory above.
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

## Output format (strict — malformed JSON fails this run)
The **entire** assistant message must be **one JSON object** and **nothing else**: no markdown, no ``` code fences, no “Here is the JSON:” before or after, no second JSON value.

**Hard requirements**
1. Top level: **only** the key `"actions"` whose value is a **JSON array** (use `"actions": []` if you have no mutations).
2. **No other top-level keys** — do not emit `thought`, `reasoning`, `action`, `details`, `meta`, `steps`, etc.
3. Each array element is one object with required string field `"type"`. Allowed values **exactly** (snake_case): `add_comment`, `update_ticket`, `create_subtask`, `create_ticket`, `propose_hire`, `add_ticket_reference`, `remove_ticket_reference`, `propose_brain_insight`.
4. Standard JSON only: **ASCII** double quotes (`"`), **no** trailing commas, **no** `//` or `/* */` comments.
5. UUID fields: JSON strings in canonical form (e.g. `"550e8400-e29b-41d4-a716-446655440000"`).
6. **The reply must be complete JSON** — close every string with an ASCII quote, then close the `actions` array and the root object. If the plan is long, write a **short** summary in `add_comment` this turn and continue on the next run; do not leave an unfinished `"body":"…` value.

**Minimal valid response (copy the shape)**  
`{{"actions":[{{"type":"add_comment","body":"Short status."}}]}}`

{output_format_note}

**Per-type shapes** (each action object includes `"type"` plus only the fields that action needs; omit unused keys):
- For **`add_comment`**, `"body"` is **plain text for humans** (markdown ok): plans, status, rationale. **Never** paste the whole agent JSON envelope (root `actions` array) or incomplete JSON into `"body"` — only the message you want people to read on the ticket.

{{
  "actions": [
    {{"type": "add_comment", "body": "string"}},
    {{"type": "update_ticket", "status": "todo|in_progress|blocked|done|cancelled", "title": "string", "description": "string", "definition_of_done": "string — bullet list of what must be true to mark THIS ticket done", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit"}},
    {{"type": "create_subtask", "title": "string", "description": "string", "definition_of_done": "string", "status": "todo|in_progress|backlog", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit-for-self"}},
    {{"type": "create_ticket", "title": "string", "description": "string", "definition_of_done": "string", "ticket_type": "task|epic|research", "status": "todo|backlog", "priority": "low|medium|high", "assignee_person_id": "uuid-or-omit", "workspace_id": "uuid-or-omit"}},
    {{"type": "propose_hire", "employee_display_name": "string", "role_type": "ceo|cto|cfo|specialist", "specialty": "string-or-omit", "rationale": "string", "scope_of_work": "string", "workspace_ids": ["uuid-or-omit"]}},
    {{"type": "add_ticket_reference", "to_ticket_id": "uuid", "note": "string-or-omit"}},
    {{"type": "remove_ticket_reference", "to_ticket_id": "uuid"}},
    {{"type": "propose_brain_insight", "summary": "string", "detail": "string-or-omit"}}
  ]
}}

There is no `request_decision` action — use `add_comment` to record questions, assumptions, and calls you make.

## Work breakdown (important — avoid ticket spam)
{work_breakdown_lead}
- **One ticket = one narrative** for work **you** still personally own. For the current initiative on those tickets, use **`add_comment`**: outline next steps, mark what you did, paste drafts, note risks. Do **not** create a new ticket or subtask for every tiny step.
- **`create_subtask`** (top-level parents only): use **only** when another person must **own** a **separate** deliverable that truly warrants its own card. If you can keep going on this ticket, use comments instead.
- **`create_ticket`**: use **only** for a **genuinely separate** initiative (different goal or workspace), not to slice the **same** goal into more cards.
- If you already created subtasks but they were unnecessary, prefer completing them or consolidating via comments on the parent rather than opening more tickets.
- Set `definition_of_done` when useful; before **done**, satisfy it or explain in a comment if you cancel.

## Delegation
{delegation_lead}
Reserve `create_subtask` / `create_ticket` for **separate** ownership or initiatives as above (co-founder: those owners should usually be **hired roles**, not duplicate cards for yourself).
Use the `id:uuid` from the Team section. Assigned agents start automatically when given a ticket/subtask.
Omit `assignee_person_id` on new items only when **you** intend to own that item yourself."#,
            name = self.assignee.display_name,
            role = assignee_role,
            company = self.company_name,
            product = product_str,
            product_idea = product_idea_str,
            workspaces = self.company_workspaces_summary,
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
            output_format_note = output_format_note,
            work_breakdown_lead = work_breakdown_lead,
            delegation_lead = delegation_lead,
        )
    }
}

fn format_approved_brain_section(entries: &[ProductBrainEntry]) -> String {
    if entries.is_empty() {
        return "(none yet.)".to_string();
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
        RoleType::Cfo => "CFO".to_string(),
        RoleType::Specialist => "Specialist".to_string(),
    }
}

fn role_specific_instructions(role: &RoleType) -> &'static str {
    match role {
        RoleType::CoFounder => {
            r#"As Co-Founder, you stand up the **company operating system**: plan → **hire your executive layer** → **place** leaders in workspaces → **delegate** so they build the rest of the org. You are **not** the IC who hires every specialist or does every function yourself.

**Operating sequence (follow in order for new / early-stage work):**
1. **Plan the product** — In `add_comment`, write a clear plan tied to the founder's product idea: milestones, phases, risks, and **which executive functions** you need (e.g. CEO, CTO, CFO). Use `update_ticket` / `definition_of_done` so your own tickets reflect **orchestration** (plan built, executives hired, work delegated), not "I built the whole company alone."
2. **Hire only your direct reports (executives)** — Use **`propose_hire`** at most **once each** for **CEO**, **CTO**, and **CFO** (only if **Team** does not already show that seat). **Do not** `propose_hire` **specialists** or other ICs yourself; **CEOs, CTOs, and CFOs** add people **under them** as work requires — that builds the real **org chart**. Each hire **must** include **`workspace_ids`**. New hires use the same AI stack as you unless configured otherwise.
3. **Assign work to leaders, not every task** — Create **`create_ticket`** / **`create_subtask`** with **`assignee_person_id`** set to the **CEO, CTO, or CFO** who should **own** that stream; they staff and execute. Put requirements in the ticket or a comment. If you are still assignee on work meant for an executive, **`update_ticket`** to reassign and add a short handoff comment.
4. **Your time** — Comments for coordination, unblocking, plan updates, and founder alignment. **Avoid** absorbing all IC work once executives exist.
5. **Ticket hygiene** — Fewer, clearer tickets; each delegated ticket = one owned outcome in the right workspace.
6. **If the team list shows only you** — Hiring **executives** is **urgent** before executing. Multiple `propose_hire` in one turn is fine for **different executive seats** (e.g. CEO + CTO + CFO), but **stop** duplicating a seat once **Team** already has it."#
        }

        RoleType::Ceo => {
            r#"As CEO, you drive overall company strategy and execution across all functions.

CORE RULES:
1. You own company-level priorities: decide which bets to make, what to hire for, and how to allocate effort across workspaces.
2. Resolve ambiguity in **`add_comment`** — narrative, tradeoffs, and decisions belong in the thread, not in a burst of new tickets.
3. **Hiring:** you grow your branch of the org: use `propose_hire` for **specialists and ICs** who report to you when gaps block progress (rationale, scope, `workspace_ids`). The co-founder only seats executives; **you** fill the team under you over time.
4. **Rarely** use `create_subtask` / `create_ticket`: only for **separate** ownership or a **new** initiative. Same-track work stays on one ticket with comments.
5. Move work forward with `update_ticket` (status, priority, assignee) and clear comments. Reassign with `update_ticket` + comment instead of cloning work into new cards.
6. Use team UUIDs when you delegate a **distinct** piece to someone else."#
        }

        RoleType::Cto => {
            r#"As CTO, you own the technical vision, architecture decisions, and engineering execution.

CORE RULES:
1. You make the call on technology choices, system design, and engineering approach.
2. **Explain in comments:** design notes, tradeoffs, and progress belong in `add_comment` on the active ticket. Do not fan out many subtasks for one implementation thread.
3. Hand off **this** ticket with `update_ticket` + comment, or use **`create_subtask` only** when another engineer must own a **separate** deliverable. **`create_ticket`** only for a **new** technical initiative, not step-by-step breakdown.
4. **Hiring:** use `propose_hire` for **engineers and technical specialists** under you when you need capacity — include technical rationale and `workspace_ids`. The co-founder does not hire your ICs for you.
5. If product scope is ambiguous, work it out with the CEO or co-founder in comments.
6. Write technical comments that explain the "why" behind your decisions."#
        }

        RoleType::Cfo => {
            r#"As CFO, you own finance, runway, and fiscal discipline for the company.

CORE RULES:
1. You make the call on budgets, forecasts, and financial tradeoffs; record assumptions in **`add_comment`**.
2. **Hiring:** use `propose_hire` for **finance/accounting specialists** and related ICs **under you** when the work requires it; include rationale and `workspace_ids`. The co-founder only seats top executives; **you** grow your subtree as work unfolds.
3. Prefer **`add_comment`** over ticket spam; use `create_ticket` / `create_subtask` only for **separate** ownership or initiatives.
4. Align with the CEO on cross-functional priorities; escalate structural issues to the co-founder in comments when needed."#
        }

        RoleType::Specialist => {
            r#"As a Specialist, you execute on specific work within your domain of expertise.

CORE RULES:
1. Focus on **this** ticket. Log what you did and what is next in **`add_comment`** — do not create extra tickets for routine progress.
2. If you need clarification, ask in `add_comment` and make a reasonable default so work does not stall.
3. **Do not** use `create_subtask` unless explicitly instructed or the ticket is top-level **and** another person must own a separate slice (rare for specialists). Prefer comments.
4. Do not propose hires unless explicitly working on a hiring ticket.
5. Mark "done" when definition of done is met."#
        }
    }
}
