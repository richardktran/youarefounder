-- Phase 6: DecisionRequest — structured founder inbox for escalated decisions.
-- An agent can emit `request_decision` to create one of these, which blocks the
-- parent ticket until the founder answers. The scheduler skips blocked tickets.

CREATE TABLE decision_requests (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id            UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    ticket_id             UUID        NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    raised_by_person_id   UUID        REFERENCES people(id) ON DELETE SET NULL,
    -- The question the agent is asking the founder.
    question              TEXT        NOT NULL,
    -- Optional extra context the agent provides to help the founder decide.
    context_note          TEXT,
    -- 'pending_founder' until the founder answers; then 'answered'.
    status                TEXT        NOT NULL DEFAULT 'pending_founder'
                              CHECK (status IN ('pending_founder', 'answered')),
    -- The founder's response — populated on answer.
    founder_answer        TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX decision_requests_company_id_idx ON decision_requests(company_id);
CREATE INDEX decision_requests_ticket_id_idx  ON decision_requests(ticket_id);
CREATE INDEX decision_requests_status_idx     ON decision_requests(status);
