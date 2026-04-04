# Phase 3 — Hiring Proposals & Contracts + Phase 3.5 — Org Chart

**Status:** Done

---

## Phase 3 — Hiring proposals & contracts

### Database migration (`005_hiring_and_contracts.sql`)

- `hiring_proposals` table: combined proposal + contract for MVP.
- Fields: `company_id`, `proposed_by_person_id`, `employee_display_name`, `role_type`, `specialty`, `ai_profile_id`, `rationale`, `scope_of_work`, `status` (`pending_founder | accepted | declined | withdrawn`), `founder_response_text`, `created_person_id` (set after accept), timestamps.

### Rust domain crate

- `crates/domain/src/hiring.rs`: `ProposalStatus` enum + `Display`/`FromStr`, `HiringProposal` struct, `CreateProposalInput`, `AcceptProposalInput`, `DeclineProposalInput`.
- Exported from `crates/domain/src/lib.rs`.

### Rust DB crate

- `crates/db/src/hiring.rs`: `list_proposals` (optional status filter), `get_proposal`, `create_proposal`, `accept_proposal` (transactional: creates `Person` + updates proposal atomically), `decline_proposal` (reason required, guards on `pending_founder` status).
- Exported from `crates/db/src/lib.rs`.

### Rust API routes

- `GET /v1/companies/:id/hiring-proposals?status=<…>` — list (optional filter)
- `POST /v1/companies/:id/hiring-proposals` — create (founder-initiated)
- `GET /v1/companies/:id/hiring-proposals/:proposal_id` — get
- `POST /v1/companies/:id/hiring-proposals/:proposal_id/accept` — accept → creates `Person`
- `POST /v1/companies/:id/hiring-proposals/:proposal_id/decline` — decline (reason required)

### Frontend

- `apps/web/src/lib/api.ts`: `ProposalStatus`, `HiringProposal` types; `listHiringProposals`, `getHiringProposal`, `createHiringProposal`, `acceptHiringProposal`, `declineHiringProposal` functions.
- `apps/web/src/app/app/[companyId]/inbox/page.tsx`: Full hiring inbox UI — pending/all tabs, proposal cards with expand-to-detail, inline accept (with optional note) and decline (reason required) flows, new proposal creation form with role/specialty/AI profile/rationale/scope fields.

---

## Phase 3.5 — Organization chart

### Database migration (`006_org_chart.sql`)

- `ALTER TABLE people ADD COLUMN reports_to_person_id UUID REFERENCES people(id) ON DELETE SET NULL` — nullable, at most one manager per person.
- Partial index on `reports_to_person_id WHERE NOT NULL`.

### Rust domain crate

- `Person` struct updated with `reports_to_person_id: Option<Uuid>`.
- `UpdatePersonInput` updated with `reports_to_person_id: Option<Option<Uuid>>` (same triple-state pattern as other nullable fields).

### Rust DB crate

- `crates/db/src/person.rs`: All queries updated to select/return `reports_to_person_id`. `update_person` handles the new field via `CASE WHEN` pattern. New `update_reporting_line` function with cycle detection via recursive CTE (`WITH RECURSIVE upchain`) and same-company guard.

### Rust API routes

- `GET /v1/companies/:id/org-chart` — flat list of `OrgNode` (id, display_name, role_type, specialty, kind, reports_to_person_id); client builds the tree.
- `PATCH /v1/companies/:id/people/:person_id/reporting-line` — set/clear manager; returns 400 if cycle detected.

### Frontend

- `apps/web/src/lib/api.ts`: `OrgNode` type; `getOrgChart`, `updateReportingLine` functions. `Person` type updated with `reports_to_person_id`.
- `apps/web/src/app/app/[companyId]/team/page.tsx`: New **Org chart** tab alongside Members tab. Shows all people with a manager dropdown (cycle-safe via API); also renders a simple indented reporting tree view.

---

## Exit criteria

| Criterion | How met |
|-----------|---------|
| Founder can submit a hiring proposal | `POST /hiring-proposals` + Inbox form |
| Accept → new Person created atomically | Transactional `accept_proposal` |
| Decline with reason stored | `decline_proposal` guards reason |
| Org chart editable, acyclic | Recursive CTE cycle check in `update_reporting_line` |
| Tree visible in UI | OrgTreeNode component in Team → Org chart tab |
