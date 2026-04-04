# Implementation Phases (Start to End)

**Phase sequencing, dependency graph, quality gates, and parallel tracks** are designed in [14-implementation-phases-design.md](./14-implementation-phases-design.md). This file is the **detailed deliverables and exit criteria** checklist.

---

## Phase 0 — Foundations (week-scale for a solo dev; adjust to team)

**Deliverables**

- Monorepo scaffold: Next app, Rust API, Postgres migrations.
- **App-managed PostgreSQL** bootstrap (first-run init + migrations + shutdown hook); optional Docker Compose **for developers only**, not a requirement for end users.
- **Job queue** as Postgres tables from day one (no separate broker).
- **In-process cache** where needed; no Redis dependency for default build.
- **No auth in phase 1:** app opens to **onboarding** (or existing company) directly.
- Create/list company; create product; basic settings page.

**Exit criteria:** On first launch, user completes onboarding and creates a company and product; on second launch, data is still there—**without register/login/JWT and without having installed PostgreSQL themselves** (see [11-embedded-runtime-data.md](./11-embedded-runtime-data.md)).

---

## Phase 1 — Onboarding + Ollama (multi-provider–ready codebase)

**Deliverables**

- Onboarding wizard UI + APIs (**company, product, AI**; Git step may be **stub or “coming soon”** until Phase 2.5).
- **`AIProfile` CRUD** with **`provider_kind` + `model_id` + JSONB `provider_config`**; only **`ollama`** enabled in UI and `GET /v1/ai-providers`.
- **`InferenceProvider` trait + registry + Ollama adapter**; test-connection runs through registry.
- Assign **co-founder** person linked to profile.

**Exit criteria:** Fresh user completes onboarding; test connection succeeds against local Ollama; adding a new vendor later is **adapter + schema + enable-list**, not a redesign. (Full **Git + index** path validated in Phase 2.5.)

---

## Phase 2 — Workspaces & tickets

**Deliverables**

- Workspaces CRUD; ticket CRUD; comments; status updates; seed default workspaces.
- Basic list UI and ticket detail UI.

**Exit criteria:** Founder can manually manage a small project structure like Jira-lite.

---

## Phase 2.5 — Git onboarding, private repos, and knowledge index

**Deliverables**

- Onboarding **Git step** + settings: `git_integration` CRUD, encrypted PAT, **test org access** ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).
- **GitHub adapter** (first): create **private** repo under specified org; persist `git_repository`; optional README scaffold.
- **Migrations:** `pgvector` + `knowledge_chunk` (and related tables).
- **Indexer worker jobs:** tree fetch, filter, chunk, **embed** (Ollama embedding model first), upsert chunks; **status API** for UI.
- **RAG hook:** Phase 3+ agent context pack includes vector retrieval when chunks exist.

**Exit criteria:** From onboarding, founder can verify Git access, create a private org repo, and see **indexing complete** with non-zero chunk count; reindex endpoint works.

---

## Phase 3 — Worker + first agent loop

**Deliverables**

- `agent_jobs` queue; worker binary; inference via **provider registry** (Ollama only enabled).
- **Context pack** includes **vector retrieval** from `knowledge_chunk` when Phase 2.5 index exists (degrades gracefully if empty).
- Single **co-founder** prompt + JSON action schema; apply actions transactionally.
- Agent run history visible on ticket.

**Exit criteria:** Assign ticket to co-founder, click run/batch; model updates ticket/comment in verifiable way; with Git index populated, run logs or comments show **grounding** from repo paths (or internal debug confirms chunks injected).

---

## Phase 4 — Roles: CEO & CTO

**Deliverables**

- Hire CEO/CTO flows (founder-driven placement in MVP—agents propose later).
- Role-specific prompts and action allowlists.
- Simple **scheduler** loop per company (cron or periodic worker tick).

**Exit criteria:** Without user interaction, at least one autonomous cycle advances tickets across roles (deterministic demo script).

---

## Phase 5 — Founder inbox: decisions

**Deliverables**

- `DecisionRequest` entity + APIs + UI.
- Blocking semantics on tickets; scheduler respects blocks.

**Exit criteria:** CEO opens decision; founder answers; blocked tickets resume.

---

## Phase 6 — Hiring proposals & contracts

**Deliverables**

- Proposal + contract entities; agent `propose_hire` action; inbox UI; accept/decline with reason.
- Materialize new `Person` on accept.

**Exit criteria:** End-to-end hire approved by founder creates a new agent that can be assigned work.

---

## Phase 7 — Autonomous structure expansion

**Deliverables**

- `create_workspace` and multi-ticket creation policies.
- Activity feed composed from structured events.
- Rate limits and better failure UX **per provider** (local vs cloud).

**Exit criteria:** Demo: from one seed idea, company spawns workspaces/tickets without founder except approvals.

---

## Phase 8 — Hardening and release

**Deliverables**

- Backup/restore docs (including **where embedded data lives** and `pg_dump`-style export if exposed in UI); encryption for secrets; basic integration tests; README runbook covering **clean install** (no manual DB).
- Optional: OpenAPI codegen; CI on PR.

**Exit criteria:** Another machine can install/run the app **without pre-provisioning Postgres, queue, or cache services**.

---

## Phase 9 — Additional AI providers (after v1)

**Deliverables**

- Enable-list **OpenAI, Anthropic, and/or Gemini** in `GET /v1/ai-providers`; implement adapters + encrypted key UX; optional model catalog ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).

**Exit criteria:** Two distinct `provider_kind` values usable in production (e.g. Ollama + OpenAI) with clear audit on `agent_run`.

---

## Risk register (short)

| Risk | Mitigation |
|------|------------|
| Model output variability | Strict JSON schema + repair pass or re-ask once; provider-specific quirks in adapters |
| Token limits | Summarize threads; chunk context pack |
| User trusts hallucinated research | UX labeling + prompts + citations discipline (future) |
| Scope creep | Ship **one** cloud provider only when Ollama path is solid; keep registry + trait from day one ([12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)) |

---

## Definition of “done” for the whole vision (v1)

Founder completes onboarding (including optional **Git org + private repo + index** when enabled); agents run on a schedule with **RAG from indexed code/docs** when chunks exist; tickets and workspaces evolve; CEO/CTO escalate decisions; hiring always passes founder contract approval; full traceability in activity history—**phase 1 inference is Ollama-first**, **architecture supports additional providers** without breaking profiles or runs. **Phase 9** covers turning on cloud providers in the enable-list.
