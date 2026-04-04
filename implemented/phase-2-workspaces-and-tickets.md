# Phase 2 — Workspaces & Tickets

**Status:** Done

## What was built

### Database migration (`003_workspaces_and_tickets.sql`)

- `workspaces` table: id, company_id, name, slug (unique per company), description, timestamps.
- `tickets` table: id, workspace_id, title, description, ticket_type (task|epic|research), status (backlog|todo|in_progress|blocked|done|cancelled), priority (low|medium|high), nullable assignee_person_id + parent_ticket_id, timestamps.
- `ticket_comments` table: id, ticket_id, body, nullable author_person_id, created_at.

### Default workspace seed

Five default workspaces are created inside the same transaction as a new company:

| Workspace | Slug | Purpose |
|-----------|------|---------|
| Discovery | discovery | Market exploration, interviews, and assumptions |
| Product | product | PRDs, user journeys, and requirements |
| R&D | rnd | Technical spikes, architecture, and implementation tasks |
| Go-to-market | gtm | Positioning, launch checklist, and growth |
| Finance | finance | Cost assumptions and pricing experiments |

### Rust domain crate

- `crates/domain/src/workspace.rs`: `Workspace`, `CreateWorkspaceInput`, `UpdateWorkspaceInput`
- `crates/domain/src/ticket.rs`: `TicketStatus`, `TicketType`, `TicketPriority` (each with `Display` + `FromStr`), `Ticket`, `CreateTicketInput`, `UpdateTicketInput`, `TicketComment`, `CreateCommentInput`

### Rust DB crate

- `crates/db/src/workspace.rs`: `list_workspaces`, `get_workspace`, `create_workspace`, `update_workspace`, `delete_workspace`, `seed_default_workspaces`
- `crates/db/src/ticket.rs`: `list_tickets`, `get_ticket`, `create_ticket`, `update_ticket`, `delete_ticket`, `list_comments`, `create_comment`

### Rust API routes

- `GET /v1/companies/:id/workspaces`
- `POST /v1/companies/:id/workspaces`
- `GET /v1/companies/:id/workspaces/:workspace_id`
- `PATCH /v1/companies/:id/workspaces/:workspace_id`
- `DELETE /v1/companies/:id/workspaces/:workspace_id`
- `GET /v1/companies/:id/workspaces/:workspace_id/tickets`
- `POST /v1/companies/:id/workspaces/:workspace_id/tickets`
- `GET /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
- `PATCH /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
- `DELETE /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id`
- `GET /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/comments`
- `POST /v1/companies/:id/workspaces/:workspace_id/tickets/:ticket_id/comments`

### Frontend (Next.js)

- `apps/web/src/lib/api.ts`: `Workspace`, `Ticket`, `TicketComment` types + all API functions.
- `apps/web/src/app/app/[companyId]/workspaces/page.tsx`: List all workspaces with live open/total ticket counts; inline create form.
- `apps/web/src/app/app/[companyId]/workspaces/[workspaceId]/page.tsx`: Workspace detail — tickets grouped by status, inline quick-create, click-to-change-status dropdown.
- `apps/web/src/app/app/[companyId]/workspaces/[workspaceId]/tickets/[ticketId]/page.tsx`: Ticket detail — editable description, status/priority/type sidebar selects, comment thread + add-comment form.

## Exit criteria

Founder can create a workspace, create tickets, move tickets through statuses, add comments, and see it all persist — without any AI involvement. Matches the Jira-lite bar from the plan.

## Deferrals

- Assignee picker (people not yet rendered in ticket sidebar; `assignee_person_id` persisted but no UI for it until Phase 3 hiring/assignee UX or Phase 5 executive-role UI per plan).
- Kanban board view (list + filters is enough per the plan; board is explicitly a "later" item).
- Ticket reporting / throughput metrics (Phase 7+ or post–v1 per plan).
