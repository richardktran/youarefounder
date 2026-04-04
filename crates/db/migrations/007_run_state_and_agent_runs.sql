-- Phase 4: company simulation run state + agent run history

-- ─── Company run state ────────────────────────────────────────────────────────
-- run_state: stopped | running | terminated
ALTER TABLE companies ADD COLUMN run_state TEXT NOT NULL DEFAULT 'stopped';

-- ─── Agent run history ────────────────────────────────────────────────────────
-- Records each agent run against a specific ticket.
CREATE TABLE agent_run_history (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_job_id        UUID        NOT NULL REFERENCES agent_jobs(id) ON DELETE CASCADE,
    ticket_id           UUID        NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    person_id           UUID        NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    prompt_tokens       INT,
    completion_tokens   INT,
    raw_response        TEXT,
    actions_applied     JSONB       NOT NULL DEFAULT '[]',
    error               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX agent_run_history_ticket_id_idx ON agent_run_history(ticket_id);
CREATE INDEX agent_run_history_job_id_idx ON agent_run_history(agent_job_id);
