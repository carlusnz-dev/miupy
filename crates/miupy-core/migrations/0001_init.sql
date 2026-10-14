-- miupy: esquema inicial (RNF-01).
-- IDs são UUID v7 em texto; datas são epoch em milissegundos (UTC); durações em ms.
-- Todas as tabelas são STRICT. Booleanos são INTEGER 0/1 com CHECK.

-- ---------------------------------------------------------------- artistas
CREATE TABLE artists (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    -- nome normalizado (minúsculas, sem acento, espaços colapsados): identidade do artista
    name_key   TEXT NOT NULL UNIQUE,
    mbid       TEXT,
    country    TEXT,
    genres     TEXT NOT NULL DEFAULT '[]',   -- array JSON de strings
    bio        TEXT,
    image_path TEXT,
    created_at INTEGER NOT NULL
) STRICT;

-- ------------------------------------------------------------------ faixas
CREATE TABLE tracks (
    id              TEXT PRIMARY KEY NOT NULL,
    title           TEXT NOT NULL,
    artist_id       TEXT REFERENCES artists(id) ON DELETE SET NULL,
    album           TEXT,
    duration_ms     INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
    file_path       TEXT,                    -- NULL até o download terminar
    cover_path      TEXT,
    source          TEXT NOT NULL CHECK (source IN ('youtube', 'local')),
    source_id       TEXT,                    -- id do vídeo no YouTube
    codec           TEXT,
    bitrate         INTEGER,
    thumbnail_url   TEXT,                    -- miniatura remota, usada para baixar a capa
    download_status TEXT NOT NULL DEFAULT 'none'
                    CHECK (download_status IN ('none', 'queued', 'downloading', 'done', 'error')),
    added_at        INTEGER NOT NULL
) STRICT;

-- deduplicação: o mesmo vídeo do YouTube só entra uma vez
CREATE UNIQUE INDEX ux_tracks_source ON tracks(source, source_id) WHERE source_id IS NOT NULL;
CREATE UNIQUE INDEX ux_tracks_file_path ON tracks(file_path) WHERE file_path IS NOT NULL;
CREATE INDEX ix_tracks_artist ON tracks(artist_id);
CREATE INDEX ix_tracks_added_at ON tracks(added_at);
CREATE INDEX ix_tracks_download_status ON tracks(download_status);

-- ---------------------------------------------------------------- favoritos
CREATE TABLE favorites (
    track_id   TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL
) STRICT;

-- --------------------------------------------------------------- playlists
CREATE TABLE playlists (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    description TEXT,
    cover_path  TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
) STRICT;

CREATE TABLE playlist_tracks (
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id    TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL CHECK (position >= 0),   -- 0..n-1, contíguo
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, track_id)
) STRICT;
-- sem UNIQUE em (playlist_id, position): o SQLite confere unicidade linha a linha e
-- reordenar com UPDATE em massa falharia no meio; a contiguidade é garantida pelo código.
CREATE INDEX ix_playlist_tracks_position ON playlist_tracks(playlist_id, position);
CREATE INDEX ix_playlist_tracks_track ON playlist_tracks(track_id);

-- ------------------------------------------------------------------- letras
CREATE TABLE lyrics (
    track_id   TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    plain      TEXT,
    synced     TEXT,                         -- formato LRC
    updated_at INTEGER NOT NULL
) STRICT;

-- ---------------------------------------------------------------- downloads
CREATE TABLE download_jobs (
    id         TEXT PRIMARY KEY NOT NULL,
    track_id   TEXT NOT NULL UNIQUE REFERENCES tracks(id) ON DELETE CASCADE,
    status     TEXT NOT NULL DEFAULT 'queued'
               CHECK (status IN ('queued', 'downloading', 'done', 'error')),
    progress   REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
    error      TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX ix_download_jobs_status ON download_jobs(status, created_at);

-- ------------------------------------------------- eventos de reprodução (RF-13)
CREATE TABLE play_events (
    id          TEXT PRIMARY KEY NOT NULL,
    track_id    TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    playlist_id TEXT REFERENCES playlists(id) ON DELETE SET NULL,  -- contexto, opcional
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER,                     -- NULL enquanto o evento está aberto
    listened_ms INTEGER NOT NULL DEFAULT 0 CHECK (listened_ms >= 0),
    completed   INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    skipped     INTEGER NOT NULL DEFAULT 0 CHECK (skipped IN (0, 1))
) STRICT;
CREATE INDEX ix_play_events_started ON play_events(started_at);
CREATE INDEX ix_play_events_track ON play_events(track_id, started_at);

-- ------------------------------------------- log genérico de ações do usuário
CREATE TABLE user_actions (
    id          TEXT PRIMARY KEY NOT NULL,
    kind        TEXT NOT NULL,               -- ex.: track_added, playlist_created
    entity_type TEXT,                        -- track | playlist | ...
    entity_id   TEXT,                        -- sem FK: o log sobrevive à remoção da entidade
    payload     TEXT,                        -- JSON opcional
    created_at  INTEGER NOT NULL
) STRICT;
CREATE INDEX ix_user_actions_created ON user_actions(created_at);
CREATE INDEX ix_user_actions_kind ON user_actions(kind, created_at);

-- ---------------------------------------------------------- perfil e config.
CREATE TABLE profile (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    display_name TEXT NOT NULL DEFAULT '',
    username     TEXT NOT NULL DEFAULT '',
    email        TEXT NOT NULL DEFAULT '',
    phone        TEXT NOT NULL DEFAULT '',
    avatar_path  TEXT,
    updated_at   INTEGER NOT NULL DEFAULT 0
) STRICT;
INSERT INTO profile (id) VALUES (1);

CREATE TABLE settings (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    lava_theme TEXT NOT NULL DEFAULT 'orange' CHECK (lava_theme IN ('orange', 'purple', 'blue')),
    volume     REAL NOT NULL DEFAULT 0.8 CHECK (volume >= 0 AND volume <= 1),
    updated_at INTEGER NOT NULL DEFAULT 0
) STRICT;
INSERT INTO settings (id) VALUES (1);

-- -------------------------------------------------- busca textual (FTS5, RF-08)
-- Tabela FTS própria (não external-content): guarda título, nome do artista e álbum.
-- rowid = tracks.rowid. unicode61 com remove_diacritics 2 ignora caixa e acento.
CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title, artist, album,
    tokenize = 'unicode61 remove_diacritics 2',
    prefix = '2 3'
);

CREATE TRIGGER tracks_fts_ai AFTER INSERT ON tracks BEGIN
    INSERT INTO tracks_fts (rowid, title, artist, album)
    VALUES (new.rowid, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), ''),
            COALESCE(new.album, ''));
END;

CREATE TRIGGER tracks_fts_ad AFTER DELETE ON tracks BEGIN
    DELETE FROM tracks_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER tracks_fts_au AFTER UPDATE OF title, artist_id, album ON tracks BEGIN
    DELETE FROM tracks_fts WHERE rowid = old.rowid;
    INSERT INTO tracks_fts (rowid, title, artist, album)
    VALUES (new.rowid, new.title,
            COALESCE((SELECT name FROM artists WHERE id = new.artist_id), ''),
            COALESCE(new.album, ''));
END;

-- renomear o artista reindexa as faixas dele
CREATE TRIGGER artists_fts_au AFTER UPDATE OF name ON artists BEGIN
    UPDATE tracks_fts SET artist = new.name
    WHERE rowid IN (SELECT rowid FROM tracks WHERE artist_id = new.id);
END;
