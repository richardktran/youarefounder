-- Product brain: approved corpus + founder review queue.
-- Cross-ticket memory: explicit references between tickets (same company).

-- ─── Approved brain entries (injected into agent context after founder approval) ─
CREATE TABLE product_brain_entries (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id       UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    -- NULL = company-wide; otherwise scoped to this workspace's tickets.
    workspace_id     UUID        REFERENCES workspaces(id) ON DELETE CASCADE,
    body             TEXT        NOT NULL,
    source_ticket_id UUID        REFERENCES tickets(id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX product_brain_entries_company_workspace_idx
    ON product_brain_entries(company_id, workspace_id);

-- ─── Pending insights (queue for founder approve / reject) ─────────────────────
CREATE TABLE product_brain_pending (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id       UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    workspace_id     UUID        REFERENCES workspaces(id) ON DELETE CASCADE,
    body             TEXT        NOT NULL,
    source_ticket_id UUID        REFERENCES tickets(id) ON DELETE SET NULL,
    status           TEXT        NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'rejected', 'promoted')),
    proposed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at      TIMESTAMPTZ
);

CREATE INDEX product_brain_pending_company_status_idx
    ON product_brain_pending(company_id, status);

-- ─── Ticket cross-references (ticket A "calls" / remembers ticket B) ───────────
CREATE TABLE ticket_references (
    from_ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    to_ticket_id   UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    note           TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (from_ticket_id, to_ticket_id),
    CHECK (from_ticket_id <> to_ticket_id)
);

CREATE INDEX ticket_references_to_idx ON ticket_references(to_ticket_id);

-- ─── Optional outcome summary when a ticket is marked done (for snapshots) ───────
ALTER TABLE tickets
    ADD COLUMN IF NOT EXISTS outcome_summary TEXT;
