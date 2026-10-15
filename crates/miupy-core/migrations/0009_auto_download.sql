-- @req RF-31
ALTER TABLE playlists ADD COLUMN auto_download INTEGER NOT NULL DEFAULT 0
    CHECK (auto_download IN (0, 1));
