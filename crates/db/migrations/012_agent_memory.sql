-- Founder-written memory injected into every agent run (tickets + decisions).
ALTER TABLE companies
    ADD COLUMN IF NOT EXISTS agent_ticket_memory TEXT,
    ADD COLUMN IF NOT EXISTS agent_decision_memory TEXT;

-- Per-ticket sticky instructions from the founder (in addition to company-wide ticket memory).
ALTER TABLE tickets
    ADD COLUMN IF NOT EXISTS founder_memory TEXT;
