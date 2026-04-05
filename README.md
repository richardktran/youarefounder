# You Are Founder

**Run a company that keeps working when you step away.** You Are Founder is a local-first app for solo builders: a persistent simulation where named AI executives and staff own workspaces and tickets, propose hires you approve, and surface **decision requests** only when your judgment actually matters—so you spend less time prompting and more time steering.

No accounts. No cloud auth. Your data stays on your machine.

---

## Why this exists

Founders juggle product, ops, and endless chat threads. This project treats your startup as a **structured simulation**: org chart, workspaces, Kanban-style work, and agent runs tied to real tickets—so “the team” has continuity instead of one-off LLM replies.

---

## Highlights

| | |
| --- | --- |
| **Embedded PostgreSQL** | Ship and run without installing Postgres. Optional external DB for developers who want Docker. |
| **Named AI roles** | Configure profiles (e.g. a cofounder-style executive) backed by your local model stack. |
| **Workspaces & tickets** | Plan work in boards; run agents on tickets and stream job activity. |
| **Hiring proposals** | The simulation can suggest new roles; you accept or decline—expansion stays intentional. |
| **Decision inbox** | Escalations land where you answer, not scattered across chats. |
| **Modern stack** | Rust API ([Axum](https://github.com/tokio-rs/axum)) + [Next.js](https://nextjs.org/) 15 UI, [TanStack Query](https://tanstack.com/query), [Tailwind CSS](https://tailwindcss.com/). |

---

## Quick start

**Prerequisites:** [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) (for the web UI), and a local LLM runtime such as [Ollama](https://ollama.com/) (used during onboarding to connect your model).

**1. Start the API** (embedded database starts automatically if `DATABASE_URL` is unset):

```bash
cargo run -p api
```

The API listens on **http://127.0.0.1:3001** by default.

**2. Start the web app** (in another terminal):

```bash
cd apps/web
npm install
npm run dev
```

Open **http://localhost:3000**. Complete the onboarding wizard (company, product, AI connection), then you land in the app.

---

## Developer setup

### Option A — Embedded PostgreSQL (same as end users)

```bash
cargo run -p api
```

### Option B — Docker Postgres (faster iteration for some workflows)

```bash
docker compose up -d
cp .env.example .env
# Set DATABASE_URL in .env, e.g.:
# DATABASE_URL=postgres://yaf:yaf@localhost:5433/yaf
cargo run -p api
```

Environment variables are documented in `.env.example`.

---

## Architecture

```
apps/web          Next.js 15 (App Router) — rewrites `/api/*` to the Rust API in dev
crates/api        Axum HTTP API — embedded PG lifecycle, REST `/v1` routes, background workers
crates/db         SQLx migrations + typed queries
crates/domain     Pure domain types (companies, people, workspaces, tickets, hiring, …)
crates/ai-core    AI orchestration primitives
crates/ai-providers Provider adapters (e.g. Ollama)
```

Deeper design notes live under `plans/`.

---

## Data directory (embedded mode)

When not using an external `DATABASE_URL`, embedded PostgreSQL files are stored at:

| OS | Path |
| --- | --- |
| macOS | `~/Library/Application Support/youarefounder/` |
| Linux | `~/.local/share/youarefounder/` |
| Windows | `%APPDATA%\youarefounder\` |

Back up that directory to preserve your company state. (Built-in export is planned.)

---

## Roadmap snapshot (Phase 0 goals)

- First launch: onboarding completes; company + product created  
- Second launch: data persists with no manual Postgres install  
- No login screen, no JWT, no auth (local-first trust model)

---

## License

MIT (see `[workspace.package]` in the root `Cargo.toml`).
