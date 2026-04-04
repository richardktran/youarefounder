# Workspaces and Tickets (Jira-like)

## Concepts

- **Workspace** ≈ Jira **project**: a container with its own ticket list and optional board.
- **Ticket** ≈ **issue**: atomic unit of work with status, assignee, comments, and linkage.

## Defaults at company creation

Seed workspaces (editable):

| Workspace | Purpose |
|-----------|---------|
| Discovery | Market exploration, interviews, assumptions |
| Product | PRDs, user journeys, requirements |
| R&D | Technical spikes, architecture, implementation tasks |
| Go-to-market | Positioning, launch checklist |
| Finance | Cost assumptions, pricing experiments (lightweight) |

**Auto-creation rule (agent-driven):** CEO/CTO may propose `create_workspace` when a class of work repeatedly clutters an existing workspace; founder policy can require decision above X workspaces.

## Ticket lifecycle (MVP)

Recommended minimal workflow:

`backlog` → `todo` → `in_progress` → `done`

Add:

- `blocked` — requires decision or external input
- `cancelled` — explicit discard

## Assignment model

- Tickets may be assigned to **AI persons** or left unassigned.
- Scheduler prefers: `todo` + assigned AI → pick run; optionally allow unassigned pool with CEO triage ticket.

## Automation (non-AI rules)

Lightweight rules engine in Rust (deterministic), examples:

- When ticket moves to `done`, notify activity feed
- When ticket created with label `research`, default assignee = co-founder

Keep **AI autonomy** separate from **rules** for debuggability.

## Board vs list

MVP: **list + filters** is enough.

Later: Kanban columns per status; drag-drop updates status with optimistic UI.

## Reporting (later)

- Throughput per workspace, average time in status, agent run success rate.

## Migration from “idea” to “build”

Use either:

- **Ticket types** (`research` vs `engineering`), or
- **Workspace move** when epic completes—pick one product pattern and stick to it in UI copy.
