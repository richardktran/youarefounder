# Frontend (Next.js / React) Plan

## App structure (App Router)

Suggested route map:

| Route | Purpose |
|-------|---------|
| `/` | **Phase 1:** if onboarding incomplete → redirect to `/onboarding`; else → `/app` (or primary `companyId`). No marketing or auth gate. |
| `/onboarding` | Multi-step wizard: company, product, co-founder role, **AI profile** (Ollama first), **Git: host + org + PAT**, test Git access, optional **create private repo** + index status |
| `/app` | Company dashboard (selector if multi-company later; phase 1 often a single company) |
| `/app/[companyId]` | Overview: active agents, recent runs, open decisions count |
| `/app/[companyId]/workspaces` | List + create workspace |
| `/app/[companyId]/workspaces/[wsId]` | Board or list view of tickets |
| `/app/[companyId]/tickets/[ticketId]` | Ticket detail: description, comments, runs, assignee |
| `/app/[companyId]/inbox/decisions` | Queue of founder decisions |
| `/app/[companyId]/inbox/hiring` | Pending contracts |
| `/app/[companyId]/team` | People, roles, AI profiles (edit with care) |
| `/app/[companyId]/settings` | Company settings, autonomy (later), AI profiles, **Git integration & reindex** |

## Key UX flows

### Onboarding wizard

Steps with **progress persistence** (save draft to backend on each step):

1. Company name
2. Product name + description
3. Choose **co-founder archetype** (e.g. “product-minded”, “technical”, “growth”)—maps to prompt template + default ticket suggestions
4. **AI provider step:** render fields from backend **provider schema**; phase 1 only lists Ollama (base URL, model, optional key); **Test connection** calls backend test endpoint for selected `provider_kind`
5. **Git step:** choose host (GitHub first), **organization/namespace**, paste **PAT** (never stored in browser), **Test access**; optional **“Create private repository for this company”** with name preview; show link to least-privilege scope docs ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)). Allow **Skip** if product policy permits deferring to settings.
6. Confirm → land on dashboard with **seed workspace** (“Discovery”) and **one starter ticket** optional; **indexing** may continue in background with progress indicator

### Founder “inbox”

Two tabs:

- **Decisions** — card per open request; respond inline; show what tickets are blocked
- **Hiring** — contract summary; Accept / Decline with required reason on decline

### Transparency layer

- **Activity feed** at company level: merges ticket changes, runs, workspace creation (from structured events).
- Avoid raw LLM walls; show **summaries** with “view details” for power users.

## State and data fetching

- **TanStack Query** (React Query) for server state; mutations invalidate scoped keys.
- **Zustand** or small context for UI-only state (sidebar, filters).
- Generated API client from OpenAPI for type safety.
- **Phase 1:** no auth headers or token refresh; API base URL points at loopback backend only.

## Components (reusable)

- `TicketStatusBadge`, `PrioritySelect`, `MarkdownEditor` for descriptions
- `AgentRunTimeline` for run history
- `DecisionThread` for founder ↔ leadership clarification
- `ContractCard` for hiring review
- `GitIntegrationForm`, `IndexingProgress` for onboarding/settings ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md))

## Accessibility and polish

- Keyboard navigable lists; focus management on modals
- Loading and empty states for inbox zero-states

## Phase 1 constraints

- No vendor SDK or keys in browser; all inference via backend adapters
- Clear, provider-specific errors when **test connection** fails (e.g. Ollama URL / Docker networking vs invalid API key for future cloud providers)
