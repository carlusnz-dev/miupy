-- @req RF-04 RF-30 RNF-01
-- A playlist marcada passa a ser a fonte única das favoritas.
ALTER TABLE settings ADD COLUMN favorites_playlist_id TEXT REFERENCES playlists(id) ON DELETE SET NULL;

INSERT INTO playlists (id, name, created_at, updated_at)
SELECT substr(printf('%012x', CAST(unixepoch('subsec') * 1000 AS INTEGER)), 1, 8) || '-' ||
       substr(printf('%012x', CAST(unixepoch('subsec') * 1000 AS INTEGER)), 9, 4) || '-7' ||
       substr(lower(hex(randomblob(2))), 1, 3) || '-8' ||
       substr(lower(hex(randomblob(2))), 1, 3) || '-' || lower(hex(randomblob(6))),
       'Favoritas', CAST(unixepoch('subsec') * 1000 AS INTEGER),
       CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE NOT EXISTS (SELECT 1 FROM playlists WHERE name COLLATE NOCASE = 'Favoritas');

UPDATE settings SET favorites_playlist_id = (
    SELECT id FROM playlists WHERE name COLLATE NOCASE = 'Favoritas'
    ORDER BY created_at, id LIMIT 1
) WHERE id = 1;

INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at)
SELECT (SELECT favorites_playlist_id FROM settings WHERE id = 1), f.track_id,
       (SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_tracks
        WHERE playlist_id = (SELECT favorites_playlist_id FROM settings WHERE id = 1))
       + ROW_NUMBER() OVER (ORDER BY f.created_at, f.track_id) - 1,
       f.created_at
FROM favorites f
WHERE NOT EXISTS (
    SELECT 1 FROM playlist_tracks pt
    WHERE pt.playlist_id = (SELECT favorites_playlist_id FROM settings WHERE id = 1)
      AND pt.track_id = f.track_id
)
ORDER BY f.created_at, f.track_id;

DROP TABLE favorites;
CREATE VIEW favorites (track_id, created_at) AS
SELECT pt.track_id, pt.added_at FROM playlist_tracks pt
JOIN settings s ON s.favorites_playlist_id = pt.playlist_id AND s.id = 1;
