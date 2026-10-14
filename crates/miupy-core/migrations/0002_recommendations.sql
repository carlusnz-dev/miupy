-- @req RF-19 RF-20
-- Metadados da faixa e candidatas locais. A migração 0001 é imutável.
ALTER TABLE tracks ADD COLUMN genres TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(genres) AND json_type(genres) = 'array');
ALTER TABLE tracks ADD COLUMN release_year INTEGER CHECK (release_year BETWEEN 1 AND 9999);
ALTER TABLE tracks ADD COLUMN mbid TEXT;
ALTER TABLE tracks ADD COLUMN isrc TEXT;

CREATE TABLE recommendation_candidates (
    source_id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
    thumbnail_url TEXT,
    genres TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(genres) AND json_type(genres) = 'array'),
    seed_kind TEXT NOT NULL CHECK (seed_kind IN ('similarTrack', 'artist', 'genre')),
    seed_label TEXT NOT NULL,
    seed_track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL,
    fetched_at INTEGER NOT NULL,
    dismissed INTEGER NOT NULL DEFAULT 0 CHECK (dismissed IN (0, 1))
) STRICT;
CREATE INDEX ix_recommendation_candidates_visible ON recommendation_candidates(dismissed, fetched_at);

-- Também cobre inserções feitas fora de Database::upsert_track.
CREATE TRIGGER recommendation_track_added AFTER INSERT ON tracks
WHEN new.source_id IS NOT NULL
BEGIN
    DELETE FROM recommendation_candidates WHERE source_id = new.source_id AND dismissed = 0;
END;
