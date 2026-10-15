-- @req RF-24
-- rusqlite_migration executa cada arquivo em uma transação. A view depende de settings.
DROP VIEW favorites;
CREATE TABLE settings_surfle (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    lava_theme TEXT NOT NULL DEFAULT 'orange' CHECK (lava_theme IN ('orange', 'purple', 'blue')),
    volume REAL NOT NULL DEFAULT 0.8 CHECK (volume >= 0 AND volume <= 1),
    updated_at INTEGER NOT NULL DEFAULT 0,
    shuffle_mode TEXT NOT NULL DEFAULT 'off' CHECK (shuffle_mode IN ('off', 'random', 'surfle')),
    gpu_mode TEXT NOT NULL DEFAULT 'auto' CHECK (gpu_mode IN ('auto', 'on', 'off')),
    normalization TEXT NOT NULL DEFAULT 'normal' CHECK (normalization IN ('off', 'quiet', 'normal', 'loud')),
    eq_preset_id TEXT NOT NULL DEFAULT 'flat',
    sidebar_width INTEGER NOT NULL DEFAULT 280 CHECK (sidebar_width BETWEEN 220 AND 400),
    favorites_playlist_id TEXT REFERENCES playlists(id) ON DELETE SET NULL
) STRICT;
INSERT INTO settings_surfle (id, lava_theme, volume, updated_at, shuffle_mode, gpu_mode,
                             normalization, eq_preset_id, sidebar_width, favorites_playlist_id)
SELECT id, lava_theme, volume, updated_at,
       CASE shuffle_mode WHEN 'smart' THEN 'surfle' ELSE shuffle_mode END,
       gpu_mode, normalization, eq_preset_id, sidebar_width, favorites_playlist_id
FROM settings;
DROP TABLE settings;
ALTER TABLE settings_surfle RENAME TO settings;
CREATE VIEW favorites (track_id, created_at) AS
SELECT pt.track_id, pt.added_at FROM playlist_tracks pt
JOIN settings s ON s.favorites_playlist_id = pt.playlist_id AND s.id = 1;
