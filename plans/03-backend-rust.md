# Backend (Rust) Plan

## Responsibilities

1. **CRUD** for all domain entities with validation.
2. **Scoping** to the local dataset: phase 1 has **no JWT or login**—treat the API as **trusted on loopback** (bind `127.0.0.1` only); still keep **company_id** on rows for clean modeling and future multi-user auth.
3. **Job enqueue** for agent work; **idempotency** keys for flaky retries.
4. **Inference** via **`InferenceProvider` trait** and **registry** (Ollama implementation in phase 1; additional crates/adapters later)—see [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md).
5. **Git host clients** and **knowledge index** (create private repos, index files, vector search)—see [13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md).

**Later:** add authentication (cookies/JWT/OIDC) before exposing the API beyond localhost or supporting multiple human users.

## Suggested crates

| Concern | Options |
|---------|---------|
| HTTP server | `axum` |
| Serialization | `serde`, `serde_json` |
| DB | `sqlx` (async) + PostgreSQL migrations in-repo |
| Auth (phase 1) | **None** for end users—loopback-only API |
| Auth (later) | Sessions/JWT/OIDC when multi-user or hosted |
| Config | `figment` / `config` |
| HTTP client | `reqwest` (all HTTP-based providers) |
| AI core | `crates/ai-core` + `crates/ai-providers` + per-vendor crates (feature-gated optional) |
| Observability | `tracing`, `tracing-subscriber` |
| Background jobs | **Postgres-backed only** for default product: **`agent_jobs`** (or `pgmq` if you want a maintained queue-on-PG layer) with **`FOR UPDATE SKIP LOCKED`**—no Redis/SQS required |
| In-process cache | `moka` / `quick_cache` (optional); **no external cache** for zero-setup install |
| Embedded Postgres | Supervisor + bundled `postgres` per OS/arch (or vetted embedded approach); see [11-embedded-runtime-data.md](./11-embedded-runtime-data.md) |
| pgvector | Embeddings for code/doc RAG (`CREATE EXTENSION vector`) |
| Git + HTTP | `reqwest` + host-specific JSON APIs; optional `git2` for push/scaffold |

## API surface (illustrative)

Prefix: `/v1`

### Phase 1 note

- No `/auth/*` routes. Optional `GET /v1/bootstrap` or `GET /v1/status` returning `{ onboarding_complete, company_id? }` so the UI knows whether to show onboarding or the app shell.

### Company & onboarding

- `POST /companies` — create company + optional first product in one transaction
- `PATCH /companies/:id` — update name/settings
- `POST /companies/:id/onboarding/complete` — validate required fields and flip flag

### Git integration & repositories ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md))

- `GET /v1/git-hosts` — enabled hosts + field schema (like AI providers)
- `PUT /companies/:id/git-integration` — org/namespace + submit token (encrypted server-side)
- `POST /companies/:id/git-integration/test` — verify token and org access
- `POST /companies/:id/git-repositories` — create **private** repo under org (or link existing)
- `GET /companies/:id/git-repositories`
- `POST /git-repositories/:rid/reindex` — enqueue full/incremental index
- `GET /companies/:id/knowledge-index/status` — chunk counts, last index, errors

### AI profiles

- `GET /v1/ai-providers` — **enabled** provider kinds + JSON Schema (or DTO) of required **non-secret** fields for dynamic forms
- `POST /companies/:id/ai-profiles` — create profile (`provider_kind`, `model_id`, `provider_config`, secrets)
- `PATCH /companies/:id/ai-profiles/:pid`
- `POST /companies/:id/ai-profiles/:pid/test` — worker runs a tiny ping completion via the matching adapter

### People (workforce)

- `GET /companies/:id/people`
- `POST /companies/:id/people` — hire flow creates after contract accept (or admin seed for co-founder)

### Workspaces & tickets

- `GET /companies/:id/workspaces`
- `POST /companies/:id/workspaces`
- `GET /workspaces/:wid/tickets`
- `POST /workspaces/:wid/tickets`
- `PATCH /tickets/:tid` — status, assignee, fields
- `POST /tickets/:tid/comments`

### Agent execution

- `POST /tickets/:tid/runs` — founder-triggered run (optional)
- `POST /companies/:id/agent-cycle` — “run one batch” for MVP debugging
- `GET /tickets/:tid/runs`, `GET /runs/:rid`

### Decisions

- `GET /companies/:id/decisions?status=open`
- `POST /decisions/:id/respond`

### Hiring

- `GET /companies/:id/hiring-proposals`
- `POST /hiring-proposals` (founder or agent via internal service)
- `POST /hiring-proposals/:id/submit`
- `POST /contracts/:id/accept`
- `POST /contracts/:id/decline`

## Worker / scheduler behavior

1. Select **candidate tickets** (e.g. `todo` or `in_progress`, assignee is AI, no open blocking decision).
2. Build **context pack**: company summary, product, workspace purpose, ticket thread, related tickets, plus **top-k `knowledge_chunk` hits** from vector search when Git index is populated ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)).
3. Resolve **`InferenceProvider`** from profile → call **`complete(ChatCompletionRequest)`** (see [05-ai-runtime.md](./05-ai-runtime.md), [12-ai-provider-extensibility.md](./12-ai-provider-extensibility.md)).
4. Parse **agent output contract** (JSON schema) into actions:
   - update ticket fields
   - add comments
   - create child tickets
   - create workspace (if allowed)
   - open decision request
   - submit hiring proposal
5. Apply actions in a **DB transaction**; on partial validation failure, rollback and record error run.

## Testing strategy

- **Unit tests** for validation and action parsing.
- **Integration tests:** **testcontainers** Postgres and/or the **same embedded Postgres path** the app uses—goal is parity so “works in CI” matches “works on install.”
- **Provider integration tests** gated behind env vars (e.g. `OLLAMA_TEST=1`, later `OPENAI_TEST=1`); **fake provider** for CI without network.

## Performance notes

- Batch agent cycles; cap concurrent runs per company.
- Large transcripts: store off-row or object storage; keep list views summary-only.
