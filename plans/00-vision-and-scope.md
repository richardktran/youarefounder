# Vision and Scope

## One-sentence vision

Give a solo founder a **persistent, autonomous “company simulation”** where named AI executives and staff plan work in workspaces and tickets, escalate only when a decision is truly needed, and expand the team through **founder-approved hiring contracts**.

## Core user story

1. A customer creates a **company** and **first product** (name + description).
2. During onboarding they configure an **AI co-founder** (role flavor + **AI provider profile**: phase 1 defaults to **Ollama**; same profile model will later support OpenAI, Claude, Gemini, etc.—see [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).
3. During onboarding they can provide **Git hosting credentials** (e.g. PAT) and a target **organization/namespace** so the system can **create private repositories** for the company and **index code and repo knowledge** for agents (RAG)—see [13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md).
4. After onboarding they create **tickets** (like Jira) to drive discovery and product shaping.
5. They **hire** a CEO and CTO (each with their own **AI profile**: provider kind + model + settings—phase 1 UI enables **Ollama** only).
6. The company **runs on its own**: agents create workspaces, tickets, and execute work—**grounded by indexed repo context** when Git integration is configured.
7. When leadership needs a human call, they open a **decision request** to the founder.
8. When anyone proposes a **new hire**, the founder **reviews a contract** (name, specialty, provider, model) and accepts or declines with a reason.

## Personas

| Persona | Needs |
|---------|--------|
| **Founder (human)** | Low-friction onboarding; informed **Git PAT** scopes and org choice; visibility into what AI is doing; clear queue of decisions; hiring veto/approval with audit trail. |
| **AI Co-founder / CEO / CTO** | Shared context (company + product + **retrieved code/docs** when indexed); tools to create structure (workspaces, tickets); ability to delegate; escalation path. |
| **Specialist agents (future)** | Scoped permissions; specialist prompts; same contract/hiring rules. |

## In scope (phase 1 — “local AI only”)

- **Zero pre-setup install** for persistence and background work: **app-managed PostgreSQL**, **queue in Postgres**, **in-process cache**—no separate install of Postgres, Redis, or a broker (see [11-embedded-runtime-data.md](./11-embedded-runtime-data.md)). Ollama remains the user-supplied local inference runtime.
- Single-tenant-per-user companies (or one founder owns many companies—pick one in domain model; see [02-domain-model.md](./02-domain-model.md)).
- **Inference:** **multi-provider–ready** data model and worker pipeline; **phase 1 only enables Ollama** in product UI (local base URL, model id, optional key). Cloud providers (OpenAI, Anthropic, Gemini, Azure OpenAI, …) are **future adapters**, not schema rewrites.
- CRUD for companies, products, key roles, workspaces, tickets, comments, status transitions.
- **Autonomous scheduler** that picks eligible tickets/agents and runs bounded “work sessions.”
- **Decision requests**: blocking questions with founder response + optional “unblock” rules.
- **Hiring proposals** with **contracts**; founder accept/decline + reason stored.
- **Git integration (extensible hosts, GitHub first):** onboarding capture of credentials + org; **auto-create private repos** under that org; **index** source and markdown knowledge into **pgvector** for **RAG** in agent context ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).

## Explicitly out of scope (early phases)

- **Accounts:** no register, login, JWT, sessions, or OAuth in **phase 1**. The app is **single-seat and local**: on launch, the UI goes **straight to onboarding** (or the main app if onboarding already completed). Add real auth when you ship multi-user or hosted.
- **Shipping** non-Ollama providers in the product UI/API enable-list—**after** Ollama path is stable; **interfaces and DB shape** already accommodate them ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).
- Real payroll, legal incorporation, or bank integrations.
- Multi-human collaboration inside one company (optional later).
- Full Jira parity (workflows editor, advanced permissions schemes, plugins).
- Guaranteed factual market research (treat as assisted ideation, not ground truth).

## Success criteria (product)

- First launch: **no login screen**—**zero → onboarding (company, product, AI, optional Git + repo bootstrap) → first ticket → first autonomous agent run** in under 30 minutes on a laptop with Ollama (data persists in embedded Postgres between launches).
- Founder can see **what changed** (tickets/workspaces created, statuses updated) without reading raw model dumps.
- **No silent hires**: every new employee-like agent requires founder approval in phase 1.

## Open product decisions (resolve before build)

1. **One company per install vs many**—phase 1 can assume **one primary company** per app data directory; adding accounts later affects multi-tenancy and billing.
2. **Real-time vs polling** for agent updates (WebSocket vs SSE vs refresh).
3. **How “automatic” runs**—continuous vs cron-like batches vs manual “run cycle” button for MVP.
4. **Git onboarding required vs skippable**—whether founders must connect Git before finishing onboarding or can defer to settings ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).
