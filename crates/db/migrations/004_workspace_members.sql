-- Phase 1: Workspace members — per-workspace team permissions

CREATE TABLE workspace_members (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id  UUID        NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    person_id     UUID        NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    -- role: 'member' | 'lead'
    role          TEXT        NOT NULL DEFAULT 'member',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, person_id)
);

CREATE INDEX workspace_members_workspace_id_idx ON workspace_members(workspace_id);
CREATE INDEX workspace_members_person_id_idx    ON workspace_members(person_id);
