-- Optional workspaces to add the new hire to when a proposal is accepted.
ALTER TABLE hiring_proposals
    ADD COLUMN workspace_ids UUID[] NULL;

-- Backfill: every AI co-founder should appear in every workspace of their company.
INSERT INTO workspace_members (workspace_id, person_id, role)
SELECT w.id, p.id, 'member'
FROM workspaces w
JOIN people p ON p.company_id = w.company_id
WHERE p.kind = 'ai_agent'
  AND p.role_type = 'co_founder'
ON CONFLICT (workspace_id, person_id) DO NOTHING;
