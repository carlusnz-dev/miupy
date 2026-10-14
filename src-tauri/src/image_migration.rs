//! Conversão de imagens legadas após a abertura, fora da thread do player.

use std::path::Path;

use crate::db::Db;
use crate::events::Events;
use crate::paths::{Dirs, compact_image, write_webp};

// @req RNF-12
pub fn migrate(dirs: &Dirs, db: &Db) -> usize {
    let mut changed = 0;
    for (dir, square) in [
        (dirs.covers.clone(), false),
        (dirs.covers.join("albuns"), false),
        (dirs.covers.join("artistas"), false),
        (dirs.root.join("avatars"), true),
        (dirs.root.join("playlists"), true),
    ] {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let old = entry.path();
            if !is_legacy_image(&old) || !old.is_file() {
                continue;
            }
            match migrate_one(&old, square, db) {
                Ok(true) => changed += 1,
                Ok(false) => {}
                Err(error) => log::warn!("imagem antiga {}: {error}", old.display()),
            }
        }
    }
    changed
}

fn is_legacy_image(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("png"))
}

fn migrate_one(old: &Path, square: bool, db: &Db) -> Result<bool, String> {
    let new = old.with_extension("webp");
    let image = image::open(old).map_err(|e| e.to_string())?;
    if !new.is_file() {
        write_webp(&compact_image(image, square), &new).map_err(|e| e.to_string())?;
    }
    let old_text = old.to_string_lossy();
    let new_text = new.to_string_lossy();
    db.with(|db| db.replace_image_path(&old_text, &new_text))
        .map_err(|e| e.to_string())?;
    std::fs::remove_file(old).map_err(|e| e.to_string())?;
    Ok(true)
}

// @req RNF-12
pub fn spawn(dirs: Dirs, db: Db, events: std::sync::Arc<dyn Events>) {
    tauri::async_runtime::spawn_blocking(move || {
        let changed = migrate(&dirs, &db);
        if changed > 0 {
            events.library_changed("images_migrated");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RNF-12
    #[test]
    fn migra_arquivos_e_referencia_uma_vez_sem_parar_em_imagem_ruim() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::new(tmp.path());
        dirs.create_all().unwrap();
        let db = Db::new(miupy_core::Database::open(&dirs.db_file).unwrap());
        let good = dirs.covers.join("album.jpg");
        image::DynamicImage::new_rgb8(900, 300)
            .save_with_format(&good, image::ImageFormat::Jpeg)
            .unwrap();
        let old_path = good.to_string_lossy().into_owned();
        let (one, two, artist, playlist) = db.with(|db| {
            let mut first = miupy_core::NewTrack::youtube(
                "image-one",
                "Um",
                Some("Artista".into()),
                1000,
                None,
            );
            first.cover_path = Some(old_path.clone());
            let mut second = miupy_core::NewTrack::youtube("image-two", "Dois", None, 1000, None);
            second.cover_path = Some(old_path.clone());
            let one = db.upsert_track(&first).unwrap().track;
            let two = db.upsert_track(&second).unwrap().track;
            let artist = one.artist_id.unwrap();
            db.set_artist_image(&artist, Some(&old_path), None).unwrap();
            db.set_profile_avatar(Some(&old_path)).unwrap();
            let playlist = db.create_playlist("Lista", None).unwrap().id;
            db.set_playlist_cover(&playlist, Some(&old_path)).unwrap();
            (one.id, two.id, artist, playlist)
        });
        let bad = dirs.covers.join("broken.png");
        std::fs::write(&bad, b"bad").unwrap();
        let avatar = dirs.root.join("avatars/old.png");
        image::DynamicImage::new_rgb8(300, 600)
            .save_with_format(&avatar, image::ImageFormat::Png)
            .unwrap();
        assert_eq!(migrate(&dirs, &db), 2);
        assert!(!good.exists());
        assert!(good.with_extension("webp").exists());
        let new_path = good.with_extension("webp").to_string_lossy().into_owned();
        db.with(|db| {
            assert_eq!(
                db.get_track(&one).unwrap().cover_path.as_deref(),
                Some(new_path.as_str())
            );
            assert_eq!(
                db.get_track(&two).unwrap().cover_path.as_deref(),
                Some(new_path.as_str())
            );
            assert_eq!(
                db.get_artist_info(&artist).unwrap().image_path.as_deref(),
                Some(new_path.as_str())
            );
            assert_eq!(
                db.get_profile().unwrap().avatar_path.as_deref(),
                Some(new_path.as_str())
            );
            assert_eq!(
                db.get_playlist(&playlist).unwrap().cover_path.as_deref(),
                Some(new_path.as_str())
            );
        });
        assert!(bad.exists());
        let avatar_webp = avatar.with_extension("webp");
        assert!(!avatar.exists());
        let avatar_image = image::open(avatar_webp).unwrap();
        assert_eq!((avatar_image.width(), avatar_image.height()), (512, 512));
        assert_eq!(migrate(&dirs, &db), 0);
    }
}
