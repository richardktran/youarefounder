-- Explicit completion criteria for tickets (shown to agents and in UI).
ALTER TABLE tickets
    ADD COLUMN IF NOT EXISTS definition_of_done TEXT;
