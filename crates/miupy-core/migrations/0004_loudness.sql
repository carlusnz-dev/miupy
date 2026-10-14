-- @req RF-28
ALTER TABLE tracks ADD COLUMN loudness_lufs REAL;
ALTER TABLE tracks ADD COLUMN true_peak_db REAL;
ALTER TABLE settings ADD COLUMN normalization TEXT NOT NULL DEFAULT 'normal'
    CHECK (normalization IN ('off', 'quiet', 'normal', 'loud'));
