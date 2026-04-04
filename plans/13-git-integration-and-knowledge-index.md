# Git Integration & Knowledge Index (Onboarding Credentials, Private Repos, RAG)

## Goals

1. **Onboarding:** collect **Git hosting credentials** (with clear least-privilege guidance) so the product can act on the founder’s behalf within a chosen **organization / group / namespace**.
2. **Automation:** **create private repositories** for the company (e.g. product codebase, docs, or agent-generated artifacts) under that org without manual clicks in the host UI.
3. **Knowledge:** **index source code and repository-hosted knowledge** (README, docs/, ADRs, markdown) into a **searchable embedding index** so agent runs can **retrieve relevant chunks** (RAG) and “maximize” grounded use of the real codebase.

This doc complements [02-domain-model.md](./02-domain-model.md), [05-ai-runtime.md](./05-ai-runtime.md), and [09-security-and-compliance.md](./09-security-and-compliance.md).

**Implementation schedule:** Git + index + RAG land in **Phase 9** (after v1); the MVP agent loop does not depend on this—see [10-implementation-phases.md](./10-implementation-phases.md).

## Git host abstraction (extensible)

| `git_host_kind` | First implementation | Notes |
|-------------------|----------------------|--------|
| `github` | Yes | PAT or GitHub App later; org slug + repo create |
| `gitlab` | Later | Group path + project create |
| `gitea` / self-hosted | Optional | Same pattern: base URL + token + API |

Store **`git_host_kind`** on the integration row; add new enum values as adapters ship.

## Credentials (onboarding)

**Phase 1 (pragmatic):** **Personal Access Token (PAT)** entered once in onboarding (or **Skip** and configure later in settings—product policy choice). Token is sent **only to the local Rust API**, stored **encrypted**, never echoed back to the client.

**Document required OAuth scopes** per host in UI (e.g. GitHub: `repo` for private repos under user; for **org-owned** private repos the token must be authorized for that org, or use a fine-grained PAT scoped to the org/repositories).

**Later:** OAuth device flow or web callback for hosted multi-user product—same `GitIntegration` row, different acquisition path.

## Domain concepts

### `git_integration` (per company)

- `company_id` (unique per company for MVP)
- `git_host_kind`
- `api_base_url` (optional override for Enterprise Server / self-hosted)
- `organization_login` or `namespace_path` — where **new private repos** are created (e.g. GitHub org name)
- `credentials_encrypted` — token or app private key material
- `status`: `inactive` | `verified` | `error` (last error message non-secret)
- `created_at`, `updated_at`

### `git_repository` (linked remote)

- `company_id`, `git_integration_id`
- `remote_name`, `full_name` (e.g. `acme-corp/product-rd`), `clone_url` (https with token injected only server-side, never persisted in plain text)
- `default_branch`
- `purpose`: `product_code` | `docs` | `artifacts` | …
- `created_by`: `onboarding` | `agent` | `founder`
- `last_indexed_commit` (nullable until first index)

### `knowledge_chunk` (index row for RAG)

- `company_id`
- `source_kind`: `repo_file` | `repo_path_aggregate` | future: `uploaded_document`
- `git_repository_id` (nullable when not from git)
- `path`, `commit_sha`, `content_sha256` (idempotency / skip unchanged)
- `language` or `mime` hint (for syntax-aware chunking later)
- `text` (or TOAST/off-row for large blobs)
- `embedding` — **pgvector** column (`vector(n)`); dimension fixed per embedding model
- `metadata` JSONB (e.g. start/end line, symbol name)

**Deletions:** on re-index, remove chunks whose `(repo_id, path)` no longer exists or whose `content_sha256` changed.

## Auto-create private repository

Flow (worker or synchronous API with enqueue):

1. Validate token can access `organization_login`.
2. `POST` host API to create repo: **private**, name from template (e.g. slugify company + product + suffix), description from product one-liner.
3. Persist `git_repository` row; optionally push an initial commit (README scaffold) via **git2** + HTTPS + token, or host API “create file” endpoint.
4. Enqueue **`index_repository`** job.

**Naming collisions:** if repo exists, surface error or offer “link existing repo” flow.

## Indexing pipeline

1. **Fetch tree:** Git host **Contents API** (preferred for serverless) or **shallow clone** to temp dir (for large repos or monorepos with sparse rules later).
2. **Filter:** respect `.gitignore`-like denylist + max file size + allowed extensions (`.rs`, `.ts`, `.tsx`, `.md`, `.toml`, …); configurable per company.
3. **Chunk:** semantic or sliding window with overlap; keep path + line range in metadata.
4. **Embed:** call **embedding adapter** (phase 1: **Ollama** `nomic-embed-text` or equivalent via HTTP; same extensibility idea as chat—trait `EmbeddingProvider`).
5. **Upsert:** write `knowledge_chunk` rows; use `content_sha256` to skip unchanged files.

**Jobs:** `index_repository_full`, `index_repository_incremental` (on webhook or poll `default_branch` SHA); queue in Postgres like agent jobs.

**Webhooks (later):** register push webhook on repo for near-real-time re-index.

## Retrieval (RAG) at agent time

When building the **context pack** ([05-ai-runtime.md](./05-ai-runtime.md)):

1. Embed a **short query** built from ticket title + description + recent comments (or multi-query).
2. **Vector search** over `knowledge_chunk` filtered by `company_id` (and optionally `git_repository_id`).
3. Inject top-k chunks into the prompt as **“Retrieved from repository”** with path + line refs; instruct model to cite paths when suggesting code changes.

**Fallback:** if index empty or embedding fails, agents still run without RAG (degraded mode).

## Onboarding UX (summary)

- Step after AI profile (or parallel): **“Connect Git (optional)”** vs **required** per product decision.
- Fields: host (GitHub first), org/namespace, PAT paste, **Test access** (lists org or creates nothing).
- Toggle: **“Create a private repository for this company now”** (name preview).
- Success: show repo URL (non-secret); background **indexing** progress in dashboard.

## PostgreSQL: pgvector

Embedded Postgres build must **`CREATE EXTENSION vector`** (or ship a build that includes pgvector). Plan disk growth for embeddings; cap chunks per company in MVP if needed.

## API surface (illustrative)

- `GET /v1/git-hosts` — enabled kinds + required field schema (mirror AI providers pattern)
- `PUT /companies/:id/git-integration` — upsert non-secret fields + submit new token (replace)
- `POST /companies/:id/git-integration/test` — verify token + org access
- `POST /companies/:id/git-repositories` — create private repo from template (or link existing by full name)
- `GET /companies/:id/git-repositories`
- `POST /git-repositories/:rid/reindex`
- `GET /companies/:id/knowledge-index/status` — last run, chunk count, errors

## Summary

- **Onboarding** captures **Git credentials** and **target org**; backend stores encrypted tokens and talks to Git host APIs.
- **Private repos** are created via host API under that org; **indexing** fills **pgvector**-backed chunks for **RAG** in agent runs—extensible to more hosts and embedding providers without changing ticket/agent action schema.
