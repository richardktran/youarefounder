# Implementation Phases (Start to End)

**Phase sequencing, dependency graph, quality gates, and parallel tracks** are designed in [14-implementation-phases-design.md](./14-implementation-phases-design.md). This file is the **detailed deliverables and exit criteria** checklist.

---

## MVP story (what we ship first)

The **minimum lovable product** is a **working company simulation** without Git or code indexing:

1. **Onboard** a new company, **configure AI** (Ollama-first), create the **co-founder** `Person` + profile.
2. Use **workspaces and tickets** as the single place work lives—including **hiring work** (e.g. tickets titled “Hire CEO”, “Hire CTO”, “Hire engineer”).
3. **Hiring & contracts:** every new AI hire is a **pending contract** the founder **accepts or declines**; on accept, a new `Person` (and optional **role**: CEO, CTO, IC) exists and can be **assigned**.
4. **Organization chart:** each person has a **reporting line** (who works under whom); the founder can **see and edit** the tree before automation leans on it.
5. **Agent loop:** the **worker** runs **assignees** on tickets (starting with the co-founder): JSON actions update tickets, add comments, and emit **`propose_hire`** into the contract pipeline—so the co-founder can **execute** “hire CEO/CTO” tickets; once those people exist, they can be **assignees** on further hiring tickets (e.g. CEO runs “Hire another role”). The agent **context pack** includes **manager / direct reports** from Phase 3.5.
6. **CEO / CTO layer:** role-specific prompts, **scheduler**, and policies so executives **pick up** tickets (including hiring) on a cadence—**after** people, **org structure**, and contracts exist and the agent loop works.

**Not in MVP:** private Git repos, **pgvector**, indexer jobs, or **RAG** in the context pack. Those ship in **Phase 9** so the agent loop **does not depend** on a repo index. The context pack for MVP is **tickets, people, company, thread, reporting hierarchy**—enough to hire, **organize**, and execute work.

---

## Phase 0 — Foundations (week-scale for a solo dev; adjust to team)

**Deliverables**

- Monorepo scaffold: Next app, Rust API, Postgres migrations.
- **App-managed PostgreSQL** bootstrap (first-run init + migrations + shutdown hook); optional Docker Compose **for developers only**, not a requirement for end users.
- **Job queue** as Postgres tables from day one (no separate broker).
- **In-process cache** where needed; no Redis dependency for default build.
- **No auth in MVP:** app opens to **onboarding** (or existing company) directly.
- Create/list company; create product; basic settings page.

**Exit criteria:** On first launch, user completes onboarding and creates a company and product; on second launch, data is still there—**without register/login/JWT and without having installed PostgreSQL themselves** (see [11-embedded-runtime-data.md](./11-embedded-runtime-data.md)).

---

## Phase 1 — Onboarding + Ollama (multi-provider–ready codebase)

**Deliverables**

- Onboarding wizard UI + APIs (**company, product, AI**). **No Git step** in MVP (stub, “later”, or omit—Git is **Phase 9**).
- **`AIProfile` CRUD** with **`provider_kind` + `model_id` + JSONB `provider_config`**; only **`ollama`** enabled in UI and `GET /v1/ai-providers`.
- **`InferenceProvider` trait + registry + Ollama adapter**; test-connection runs through registry.
- Create **co-founder** `Person` linked to profile.

**Exit criteria:** Fresh user completes onboarding; test connection succeeds against local Ollama; co-founder exists. Adding a new vendor later is **adapter + schema + enable-list**, not a redesign.

---

## Phase 2 — Workspaces & tickets

**Deliverables**

- Workspaces CRUD; ticket CRUD; comments; status updates; seed default workspaces (optionally seed **example hiring tickets** or templates: “Hire CEO”, “Hire CTO”).
- List + ticket detail UI; **assignee** surfaced in UI when multiple people exist (co-founder first; more after Phase 3).

**Exit criteria:** Founder can manage a Jira-lite structure; tickets can represent **hiring work** as well as product/engineering work.

---

## Phase 3 — Hiring proposals & contracts

**Deliverables**

- `HiringProposal` + `Contract` entities; **inbox UI**; accept/decline with reason; immutable snapshot on accept.
- Materialize new `Person` on accept; support **role designation** (CEO, CTO, IC, …) as required for downstream prompts and assignment.
- Founder- or API-initiated proposals for MVP (so the first CEO/CTO hires do not depend on the worker).

**Exit criteria:** Founder can approve contracts until **CEO, CTO, and additional staff** exist as assignable people; hiring is always **founder-approved** via contract.

---

## Phase 3.5 — Organization chart (who reports to whom)

**Deliverables**

- **Reporting relationship** in the data model (e.g. nullable **`reports_to_person_id`** on `person`, company-scoped): each person **at most one manager**; **no cycles**; root(s) consistent with product rules (e.g. co-founder and/or founder-facing root).
- **APIs** to read/update reporting lines and **list direct reports**; validate on write (cycle detection, same-company guard).
- **UI:** **organization chart** view (tree, layered list, or similar) plus **per-person** control to set/clear manager; empty state when only the co-founder exists.
- Optional: default **suggested** reporting from **role** (e.g. IC → CTO → CEO) as a starting point the founder can override—policy in product, not required for exit.

**Exit criteria:** For a company with multiple people, the founder can **see** who works under whom and **change** reporting lines; the graph stays acyclic and matches the mental model of the **simulation** (escalation and “my manager” for Phase 4+ context).

---

## Phase 4 — Worker + agent loop (ticket execution)

**Deliverables**

- `agent_jobs` queue; worker binary; inference via **provider registry** (Ollama only enabled).
- **Context pack (MVP):** company, workspace, ticket, thread, **people list**, **manager and direct reports** (from Phase 3.5)—**no** `knowledge_chunk` / RAG (that is Phase 9).
- JSON **action schema**; transactional apply; **`propose_hire`** produces pending contracts aligned with Phase 3.
- Agent run history visible on ticket; manual **run / batch** from UI.

**Exit criteria:** Assign a ticket (e.g. “Hire CEO”) to the **co-founder**, run the agent; model updates ticket/comments and can **submit a hire proposal**; founder completes the contract in the inbox; **new person can be assignee** on the next ticket. Repeat for CEO-driven hiring tickets once Phase 5 exists. Agent prompts can rely on **org context** from Phase 3.5.

---

## Phase 5 — Roles: CEO & CTO + scheduler

**Deliverables**

- Bind **CEO / CTO** (and other roles) to `Person` records; role-specific prompts and **action allowlists** (including hiring-related actions where policy allows).
- **Scheduler** (cron or periodic worker tick) so executives **claim or advance** tickets without a manual click—demo-friendly deterministic script counts as exit criteria.

**Exit criteria:** In a scripted demo, **CEO** (once hired) processes at least one ticket autonomously (e.g. a hiring or prioritization ticket); co-founder and executives **do not** share one undifferentiated prompt.

---

## Phase 6 — Founder inbox: decisions

**Deliverables**

- `DecisionRequest` entity + APIs + UI.
- Blocking semantics on tickets; scheduler respects blocks.

**Exit criteria:** Escalated decision blocks work; founder answer unblocks the ticket.

---

## Phase 7 — Autonomous structure expansion

**Deliverables**

- `create_workspace` and multi-ticket creation policies (where product allows).
- Activity feed from structured events; rate limits and clearer failure UX **per provider** (local vs cloud, when Phase 10 adds cloud).

**Exit criteria:** From a seed, company can add structure and tickets with minimal founder input except approvals and contracts.

---

## Phase 8 — Hardening and release (v1)

**Deliverables**

- Backup/restore docs (**where embedded data lives**; optional `pg_dump`-style export); encryption for secrets; basic integration tests; README runbook for **clean install** (no manual DB).
- Optional: OpenAPI codegen; CI on PR.

**Exit criteria:** Another machine can install and run the app **without** pre-provisioning Postgres, queue, or cache services. **This closes the MVP scope** above (through autonomous expansion + polish).

---

## Phase 9 — Git onboarding, private repos, and knowledge index (post–MVP)

**Deliverables**

- Onboarding/settings: `git_integration` CRUD, encrypted PAT, **test org access** ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).
- **GitHub adapter** (first): private org repo; `git_repository`; optional README scaffold.
- **Migrations:** `pgvector` + `knowledge_chunk`; indexer worker jobs; embed (Ollama embeddings first); status API.
- **RAG hook:** extend agent **context pack** with vector retrieval when chunks exist (graceful empty degrade).

**Exit criteria:** Founder can connect Git, index a repo, and agent runs show **grounded** context from indexed paths when chunks exist.

---

## Phase 10 — Additional AI providers (after v1)

**Deliverables**

- Enable-list **OpenAI, Anthropic, and/or Gemini** in `GET /v1/ai-providers`; adapters + encrypted key UX; optional model catalog ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).

**Exit criteria:** Two distinct `provider_kind` values usable in production with audit on `agent_run`.

---

## Risk register (short)

| Risk | Mitigation |
|------|------------|
| Model output variability | Strict JSON schema + repair pass or re-ask once; adapter quirks isolated |
| Token limits | Summarize threads; chunk context pack; **RAG in Phase 9** reduces raw paste |
| Hiring without human gate | Never insert `Person` from model alone—always **contract accept** ([07-hiring-and-approvals.md](./07-hiring-and-approvals.md)) |
| Scope creep | **No Git in v1**; ship Ollama path + hiring + **org chart** + agent loop before indexers |

---

## Definition of “done” for v1 (Phases 0–8)

Founder completes **onboarding** with **AI + co-founder**; **tickets** drive work including **hiring CEO/CTO and beyond**; **contracts** gate every hire; an **organization chart** (Phase 3.5) records **who reports to whom**; the **agent loop** executes ticket work and **`propose_hire`** with **org-aware** context; **CEO/CTO** run on a **schedule** with role prompts; optional **decisions** inbox and **autonomous expansion** per phases 6–7. **Inference is Ollama-first**; architecture stays multi-provider–ready. **Phase 9** adds Git + RAG; **Phase 10** adds cloud LLMs.
