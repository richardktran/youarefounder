# You Are Founder

Give a solo founder a **persistent, autonomous "company simulation"** where named AI executives and staff plan work in workspaces and tickets, escalate only when a decision is truly needed, and expand the team through founder-approved hiring contracts.

## Quick start (end users)

```bash
cargo run -p api
# Open http://localhost:3001 (API) and http://localhost:3000 (UI)
```

No PostgreSQL install required — the app manages its own embedded database.

## Developer setup

### Option A: Embedded PostgreSQL (same as end users)

```bash
cargo run -p api
```

### Option B: Docker Compose (faster iteration)

```bash
docker compose up -d
cp .env.example .env
# Set DATABASE_URL in .env
cargo run -p api
```

### Frontend

```bash
cd apps/web
npm install
npm run dev
```

## Architecture

```
apps/web          Next.js 15 (App Router)
crates/api        Axum HTTP API — embedded PG lifecycle, REST routes
crates/db         SQLx migrations + typed query functions
crates/domain     Pure domain types, no I/O
```

See `plans/` for full architecture documentation.

## Phase 0 exit criteria

- [ ] First launch: onboarding wizard completes, company + product created
- [ ] Second launch: data persists without any manual PostgreSQL install
- [ ] No login screen, no JWT, no auth

## Data directory

The app stores its embedded PostgreSQL data at:

- **macOS:** `~/Library/Application Support/youarefounder/`
- **Linux:** `~/.local/share/youarefounder/`
- **Windows:** `%APPDATA%\youarefounder\`

To back up: copy this directory or use the built-in export (coming in Phase 8).
