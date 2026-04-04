-- Phase 0 initial schema
-- All tables use UUIDs and timestamptz for portability.

-- ─── Companies ────────────────────────────────────────────────────────────────
CREATE TABLE companies (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name                 TEXT        NOT NULL,
    slug                 TEXT        NOT NULL UNIQUE,
    onboarding_complete  BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Products ─────────────────────────────────────────────────────────────────
CREATE TABLE products (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id   UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    description  TEXT,
    -- enum values: idea | discovery | spec | building | launched
    status       TEXT        NOT NULL DEFAULT 'idea',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX products_company_id_idx ON products(company_id);

-- ─── People ───────────────────────────────────────────────────────────────────
-- Unified table for human founders and AI agents.
-- kind: human_founder | ai_agent
-- role_type: co_founder | ceo | cto | specialist
CREATE TABLE people (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    kind            TEXT        NOT NULL,
    display_name    TEXT        NOT NULL,
    role_type       TEXT        NOT NULL,
    specialty       TEXT,
    ai_profile_id   UUID,       -- nullable; references ai_profiles (added Phase 1)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX people_company_id_idx ON people(company_id);

-- ─── Agent job queue ──────────────────────────────────────────────────────────
-- Postgres-backed queue; no Redis or separate broker required.
-- Worker claims jobs with: SELECT ... FOR UPDATE SKIP LOCKED
CREATE TABLE agent_jobs (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    kind          TEXT        NOT NULL,
    company_id    UUID        NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    payload       JSONB       NOT NULL DEFAULT '{}',
    -- status: pending | running | succeeded | failed
    status        TEXT        NOT NULL DEFAULT 'pending',
    run_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at    TIMESTAMPTZ,
    completed_at  TIMESTAMPTZ,
    error         TEXT,
    attempts      INT         NOT NULL DEFAULT 0,
    max_attempts  INT         NOT NULL DEFAULT 3,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partial index: only pending jobs need fast dequeue lookup.
CREATE INDEX agent_jobs_dequeue_idx
    ON agent_jobs (run_at ASC)
    WHERE status = 'pending';

CREATE INDEX agent_jobs_company_id_idx ON agent_jobs(company_id);
