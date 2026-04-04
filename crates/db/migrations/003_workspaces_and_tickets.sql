-- Phase 2: Workspaces & Tickets

-- ─── Workspaces ────────────────────────────────────────────────────────────────
-- Jira "project" analog; company has multiple named workspaces.
CREATE TABLE workspaces (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id   UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    -- short prefix slug, e.g. "rd", "gtm"
    slug         TEXT        NOT NULL,
    description  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (company_id, slug)
);

CREATE INDEX workspaces_company_id_idx ON workspaces(company_id);

-- ─── Tickets ──────────────────────────────────────────────────────────────────
-- Atomic unit of work within a workspace.
-- type:     task | epic | research
-- status:   backlog | todo | in_progress | blocked | done | cancelled
-- priority: low | medium | high
CREATE TABLE tickets (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id        UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title               TEXT        NOT NULL,
    description         TEXT,
    ticket_type         TEXT        NOT NULL DEFAULT 'task',
    status              TEXT        NOT NULL DEFAULT 'backlog',
    priority            TEXT        NOT NULL DEFAULT 'medium',
    assignee_person_id  UUID        REFERENCES people(id) ON DELETE SET NULL,
    parent_ticket_id    UUID        REFERENCES tickets(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tickets_workspace_id_idx    ON tickets(workspace_id);
CREATE INDEX tickets_status_idx          ON tickets(status);
CREATE INDEX tickets_assignee_idx        ON tickets(assignee_person_id) WHERE assignee_person_id IS NOT NULL;

-- ─── Ticket comments ──────────────────────────────────────────────────────────
-- Append-only comments (human or agent-authored).
-- author_person_id is NULL for system-generated events.
CREATE TABLE ticket_comments (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id         UUID        NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    body              TEXT        NOT NULL,
    author_person_id  UUID        REFERENCES people(id) ON DELETE SET NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ticket_comments_ticket_id_idx ON ticket_comments(ticket_id);
