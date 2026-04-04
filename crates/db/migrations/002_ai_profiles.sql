-- Phase 1: AI provider profiles
-- Stores provider-agnostic config; provider_config JSONB carries vendor-specific fields.

CREATE TABLE ai_profiles (
    id                   UUID             PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id           UUID             NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    display_name         TEXT,
    -- stable slug: ollama | openai_api | anthropic | google_gemini | azure_openai
    provider_kind        TEXT             NOT NULL,
    model_id             TEXT             NOT NULL,
    -- vendor-specific config; schema_version inside JSON for forward compat
    provider_config      JSONB            NOT NULL DEFAULT '{}',
    default_temperature  DOUBLE PRECISION,
    default_max_tokens   INT,
    created_at           TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);

CREATE INDEX ai_profiles_company_id_idx ON ai_profiles(company_id);

-- Enforce the FK that was deferred in Phase 0.
ALTER TABLE people
    ADD CONSTRAINT people_ai_profile_id_fkey
    FOREIGN KEY (ai_profile_id) REFERENCES ai_profiles(id) ON DELETE SET NULL;
