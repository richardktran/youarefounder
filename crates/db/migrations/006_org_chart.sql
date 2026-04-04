-- Phase 3.5: Organization chart — reporting relationships
--
-- Each person can have at most one manager (reports_to_person_id).
-- Root nodes (co-founder, CEO) have NULL.
-- Cycle safety is enforced at the application layer via ancestor traversal.

ALTER TABLE people
    ADD COLUMN reports_to_person_id UUID REFERENCES people(id) ON DELETE SET NULL;

CREATE INDEX people_reports_to_idx ON people(reports_to_person_id)
    WHERE reports_to_person_id IS NOT NULL;
