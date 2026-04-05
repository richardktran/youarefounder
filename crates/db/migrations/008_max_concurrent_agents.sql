-- Add configurable concurrency limit for the agent worker.
-- Default of 1 preserves existing single-agent behaviour.
ALTER TABLE companies
    ADD COLUMN IF NOT EXISTS max_concurrent_agents INTEGER NOT NULL DEFAULT 1;
