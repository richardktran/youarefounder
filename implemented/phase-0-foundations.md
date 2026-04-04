# Phase 0 — Foundations (completed)

Reference: [plans/10-implementation-phases.md](../plans/10-implementation-phases.md) (Phase 0).

## Summary

The repo is a **monorepo** with a **Next.js** web app, a **Rust (Axum) API**, **SQLx migrations**, and **embedded PostgreSQL** for local runs with no manual database install. There is **no authentication**; the UI uses **bootstrap** to choose onboarding vs the main app shell.

## Deliverables (as implemented)

### Monorepo scaffold

- **`apps/web`** — Next.js 15 (App Router), React Query, Tailwind-style UI primitives under `src/components/ui/`.
- **`crates/api`** — HTTP server, CORS, tracing, `/v1` routes.
- **`crates/db`** — migrations and typed query modules.
- **`crates/domain`** — shared structs/enums (companies, products, people, jobs, etc.).

### App-managed PostgreSQL

- **`crates/api/src/embedded_postgres.rs`** — `pg-embed` starts Postgres under the resolved **data directory** (see `config.resolved_data_dir()`), persists port in `db.port`, cluster in `pgdata/`.
- **`crates/api/src/config.rs`** — If **`DATABASE_URL`** is set, embedded PG is skipped (e.g. Docker Compose for developers).
- **`crates/api/src/main.rs`** — Connects pool, runs migrations, serves API.

### Migrations and schema (Phase 0 scope)

- **`crates/db/migrations/001_initial.sql`** — `companies`, `products`, `people` (with nullable `ai_profile_id` placeholder), **`agent_jobs`** queue table (Postgres-backed, no Redis).

### Job queue (data plane)

- **`agent_jobs`** table with status, payload JSONB, scheduling fields — ready for a worker; no separate message broker.

### In-process cache

- **`AppState`** in `crates/api/src/state.rs` includes a **Moka** async cache (reserved for hot reads; no Redis in default build).

### No auth

- No register, login, JWT, or sessions. Routes assume a single local trust boundary.

### Product flows

- **Companies:** create, list, get, patch; slug generated on create.
- **Products:** nested under company; CRUD via `/v1/companies/:id/products`.
- **`GET /v1/bootstrap`** — returns whether onboarding is complete and optional `company_id` (see `crates/api/src/routes/bootstrap.rs`, `db::company::get_bootstrap_status`).
- **Onboarding UI** — wizard creates company (+ first product) and calls **`POST /v1/companies/:id/complete-onboarding`** (Phase 0 required: company + product; Phase 1 tightened gates — see Phase 1 doc).
- **Main app shell** — routes under `apps/web/src/app/app/[companyId]/` with dashboard, settings, team, workspaces, inbox (several areas are **“coming soon”** placeholders).

## Exit criteria (Phase 0)

| Criterion | How it is met |
|-----------|----------------|
| First launch: onboarding completes; company + product created | Wizard + API create company and product; onboarding completion endpoint exists. |
| Second launch: data persists without installing Postgres yourself | Embedded PG stores data under the OS app data path (macOS uses `~/Library/Application Support/com.youarefounder.youarefounder/` with current `directories` layout). |
| No login screen, no JWT, no auth | Bootstrap drives onboarding vs app; no auth layer. |

## Notes

- **`README.md`** “Data directory” paths may not match the exact folder name on macOS; the authoritative path comes from `directories::ProjectDirs` + `config.data_dir` override.
- Phase 0 README checklist items may still be unchecked in the root README; this file is the phase completion record.
