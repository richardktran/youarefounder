# Embedded PostgreSQL, Queue, and Cache (Zero Pre-Setup Install)

## Goal

When someone **installs or runs this app**, they should **not** have to install or configure PostgreSQL, Redis, or any separate message broker. The application brings its own **persistence, job queue, and caching** (or operates fine without an external cache).

This matches a **local-first / desktop-style** distribution: double-click or one command, first launch initializes storage under a well-known **application data directory**.

## PostgreSQL (embedded / app-managed)

Keep **PostgreSQL as the SQL engine** (same schema and `sqlx` migrations as in [03-backend-rust.md](./03-backend-rust.md)), but **no user-installed server**.

Practical implementation patterns (pick one per platform strategy):

| Approach | Idea | Tradeoffs |
|----------|------|-----------|
| **Bundled `postgres` binary** | Ship official Postgres builds for each target OS/arch; on startup the Rust supervisor spawns `postgres` with `-D` pointed at a directory under the app’s data path (e.g. `%APPDATA%`, `~/Library/Application Support`, `XDG_DATA_HOME`). | Larger download; very compatible with full Postgres features; familiar ops (VACUUM, backups = copy data dir + tools). |
| **Embedded library build** | Use a crate or vendor workflow that links or loads a Postgres-compatible engine (ecosystem evolves; validate licensing and FFI story for your targets). | Smaller surface if it works; more integration risk. |

**Lifecycle**

- **First run:** create data directory, run `initdb` (or equivalent), apply migrations, optionally set a random superuser password stored only in app config (not shown to user).
- **Subsequent runs:** start DB if not running; health-check before accepting API traffic.
- **Shutdown:** graceful stop of the child `postgres` process on app exit.

**Connection string** is internal (e.g. `127.0.0.1` on a **dynamic or reserved high port** chosen at init and persisted in local config), not something the founder configures.

## PostgreSQL extensions (pgvector)

For **code and document embeddings** ([13-git-integration-and-knowledge-index.md](./13-git-integration-and-knowledge-index.md)), ship or build Postgres with **`pgvector`** available and run `CREATE EXTENSION vector` in migrations. Vector indexes increase **disk and memory** use—size the bundled Postgres tuning accordingly and document backup size growth.

## Job queue (no Redis, no separate broker)

Use **PostgreSQL as the queue**:

- Tables e.g. `agent_jobs` and **`index_jobs`** (or a shared `background_jobs` with `kind`) with `status`, `run_at`, `payload`, **lease / visibility timeout**, and **`FOR UPDATE SKIP LOCKED`** dequeue in the worker.
- **API** inserts rows; **worker** (same process or sidecar thread in the same binary) claims and processes jobs.

This satisfies “embedded queue”: **no extra infrastructure**, same backup story as the rest of the DB, transactional enqueue with domain writes when needed.

## Cache (no Redis)

Default: **in-process cache** only, e.g. `moka` or `quick_cache`:

- Session-scoped or short-TTL memoization (e.g. parsed config, feature flags, hot read models).
- **Ephemeral by design**: safe to lose on restart; all authoritative state remains in Postgres.

If you later need shared cache across multiple API processes, that implies a **hosted / multi-instance** deployment—not the zero-setup install path. Document that split when you add it.

## Developer workflow vs end-user install

| Audience | Data layer |
|----------|------------|
| **End user** | Embedded/app-managed Postgres + PG queue + in-process cache (this doc). |
| **Developers** | May still use **Docker Compose** or a local `postgres` for speed, **or** run the same embedded path as production for parity. CI can use **testcontainers** or embedded Postgres to run migrations and integration tests. |

## Backups and portability

- Document **where the data directory lives** and how to **export/import** (e.g. `pg_dump` / `pg_restore` invoked by the app’s “Backup” command).
- Uninstall policy: warn before deleting the data directory; optional “remove all data” checkbox.

## Security notes (local)

- Bind embedded Postgres to **loopback** only; random port + auth still required so other local users cannot connect trivially.
- See [09-security-and-compliance.md](./09-security-and-compliance.md) for secrets and filesystem permissions on the data directory.

## Summary

- **PostgreSQL:** yes, but **managed by the app**—no separate install step.
- **Queue:** **Postgres-backed** job table—no Redis.
- **Cache:** **in-process** (optional)—no Redis.
