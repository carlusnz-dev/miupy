//! Auxiliares compartilhados pelos testes unitários.

use crate::{Database, NewTrack, Track};

pub(crate) fn db() -> Database {
    let mut db = Database::open_in_memory().unwrap();
    // relógio crescente (1 ms por chamada) para ordenações estáveis
    let counter = std::sync::atomic::AtomicI64::new(1_000_000);
    db.set_clock(move || counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
    db
}

static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn new_local(title: &str, artist: &str, album: Option<&str>, duration_ms: i64) -> NewTrack {
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    NewTrack {
        source: crate::Source::Youtube,
        source_id: Some(format!("vid-{n}")),
        title: title.into(),
        artist_name: Some(artist.into()),
        album: album.map(Into::into),
        duration_ms,
        file_path: None,
        cover_path: None,
        codec: None,
        bitrate: None,
        thumbnail_url: None,
        genres: Vec::new(),
        release_year: None,
        mbid: None,
        isrc: None,
    }
}

pub(crate) fn add_track(db: &Database, title: &str, artist: &str, duration_ms: i64) -> Track {
    db.upsert_track(&new_local(title, artist, None, duration_ms))
        .unwrap()
        .track
}

pub(crate) fn add_track_album(
    db: &Database,
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: i64,
) -> Track {
    db.upsert_track(&new_local(title, artist, Some(album), duration_ms))
        .unwrap()
        .track
}

/// Cria a faixa e fixa `added_at` (o relógio do banco não é tocado).
pub(crate) fn add_track_album_at(
    db: &Database,
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: i64,
    added_at: i64,
) -> Track {
    let t = add_track_album(db, title, artist, album, duration_ms);
    db.conn
        .execute(
            "UPDATE tracks SET added_at = ?2 WHERE id = ?1",
            rusqlite::params![t.id, added_at],
        )
        .unwrap();
    db.get_track(&t.id).unwrap()
}

pub(crate) fn titles(tracks: &[Track]) -> Vec<String> {
    tracks.iter().map(|t| t.title.clone()).collect()
}
