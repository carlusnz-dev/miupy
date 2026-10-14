-- @req RF-29
CREATE TABLE eq_presets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    gains_db TEXT NOT NULL CHECK (json_valid(gains_db)),
    created_at INTEGER NOT NULL,
    UNIQUE(name)
) STRICT;
ALTER TABLE settings ADD COLUMN eq_preset_id TEXT NOT NULL DEFAULT 'flat';
