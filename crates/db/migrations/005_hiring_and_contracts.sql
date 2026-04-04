-- Phase 3: Hiring proposals & contracts
--
-- Combined proposal + contract table for MVP.
-- status: pending_founder | accepted | declined | withdrawn
--
-- On accept: created_person_id is set to the newly created Person.
-- On decline: founder_response_text stores the reason.

CREATE TABLE hiring_proposals (
    id                      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id              UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    -- person who initiated the proposal (founder or AI agent); nullable for system-init
    proposed_by_person_id   UUID        REFERENCES people(id) ON DELETE SET NULL,
    -- contract terms
    employee_display_name   TEXT        NOT NULL,
    -- role_type: co_founder | ceo | cto | specialist
    role_type               TEXT        NOT NULL,
    specialty               TEXT,
    -- AI profile the new hire will use; chosen at proposal time
    ai_profile_id           UUID        REFERENCES ai_profiles(id) ON DELETE SET NULL,
    -- narrative fields
    rationale               TEXT,
    scope_of_work           TEXT,
    -- status: pending_founder | accepted | declined | withdrawn
    status                  TEXT        NOT NULL DEFAULT 'pending_founder',
    -- founder's note (required on decline, optional on accept)
    founder_response_text   TEXT,
    -- populated after accept; points at the newly created Person row
    created_person_id       UUID        REFERENCES people(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX hiring_proposals_company_id_idx ON hiring_proposals(company_id);
CREATE INDEX hiring_proposals_status_idx     ON hiring_proposals(company_id, status);
