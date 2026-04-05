-- Phase 7: Priority queue for agent jobs.
--
-- Adds a `priority` column so that co-founder work is always processed before
-- CEO/CTO tasks, which in turn are processed before specialist work.
-- Lower number = higher priority.
--
-- Priority tiers:
--   10  co_founder   (founding vision work)
--   20  ceo / cto    (executive leadership)
--   50  specialist   (domain execution)
--   50  default      (manually triggered or unknown)

ALTER TABLE agent_jobs
    ADD COLUMN priority SMALLINT NOT NULL DEFAULT 50;

CREATE INDEX agent_jobs_priority_run_at_idx
    ON agent_jobs (priority ASC, run_at ASC)
    WHERE status = 'pending';
