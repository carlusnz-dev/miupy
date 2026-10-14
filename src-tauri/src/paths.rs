//! Diretórios de dados do app e preferências pequenas que o núcleo não guarda.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::error::AppResult;

/// Layout em disco, sob o diretório de dados do app (`app_data_dir()`).
#[derive(Debug, Clone)]
pub struct Dirs {
    pub root: PathBuf,
    pub db_file: PathBuf,
    pub audio: PathBuf,
    pub covers: PathBuf,
    pub prefs_file: PathBuf,
}

impl Dirs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Dirs {
            db_file: root.join("miupy.db"),
            audio: root.join("audio"),
            covers: root.join("covers"),
            prefs_file: root.join("prefs.json"),
            root,
        }
    }

    pub fn create_all(&self) -> AppResult<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.audio)?;
        std::fs::create_dir_all(self.covers.join("artistas"))?;
        std::fs::create_dir_all(self.covers.join("albuns"))?;
        std::fs::create_dir_all(self.root.join("avatars"))?;
        std::fs::create_dir_all(self.root.join("playlists"))?;
        std::fs::create_dir_all(self.root.join("bin"))?;
        Ok(())
    }

    /// O caminho está dentro do diretório de dados? (só apagamos arquivos nossos)
    pub fn owns(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}

// @req RF-05 RF-12
pub fn album_key(recording: &miupy_media::RecordingMatch) -> String {
    if let Some(group) = recording
        .release_group_mbid
        .as_ref()
        .filter(|s| !s.is_empty())
    {
        return group.clone();
    }
    let artist = recording
        .artist
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let album = recording
        .album
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let digest = sha2::Sha256::digest(format!("{artist}\0{album}").as_bytes());
    let mut key = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(key, "{byte:02x}").expect("escrita em String");
    }
    key
}

// @req RF-21 RF-22 RNF-12
pub fn save_upload(
    upload: &crate::dto::ImageUpload,
    directory: &Path,
    stem: &str,
) -> AppResult<PathBuf> {
    use base64::Engine;
    if !matches!(
        upload.mime_type.as_str(),
        "image/jpeg" | "image/png" | "image/webp"
    ) {
        return Err(crate::error::AppError::invalid(
            "Formato aceito: JPEG, PNG ou WebP.",
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&upload.data_base64)
        .map_err(|_| crate::error::AppError::invalid("Imagem em base64 inválida."))?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(crate::error::AppError::invalid("A imagem excede 10 MB."));
    }
    let image = image::load_from_memory(&bytes)
        .map_err(|_| crate::error::AppError::invalid("Arquivo de imagem inválido."))?;
    if image.width() == 0 || image.height() == 0 {
        return Err(crate::error::AppError::invalid("Imagem vazia."));
    }
    let image = compact_image(image, true);
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("{stem}-{}.webp", uuid::Uuid::now_v7()));
    write_webp(&image, &path)?;
    Ok(path)
}

// @req RNF-12
pub fn compact_image(image: image::DynamicImage, square: bool) -> image::DynamicImage {
    if square {
        let side = image.width().min(image.height());
        let image = image.crop_imm(
            (image.width() - side) / 2,
            (image.height() - side) / 2,
            side,
            side,
        );
        image.resize_exact(512, 512, image::imageops::FilterType::Lanczos3)
    } else if image.width().max(image.height()) <= 600 {
        image
    } else {
        image.resize(600, 600, image::imageops::FilterType::Lanczos3)
    }
}

// @req RNF-12
pub fn write_webp(image: &image::DynamicImage, path: &Path) -> AppResult<()> {
    let rgba = image::DynamicImage::ImageRgba8(image.to_rgba8());
    let encoder = webp::Encoder::from_image(&rgba)
        .map_err(|e| crate::error::AppError::invalid(e.to_string()))?;
    let bytes = encoder.encode(82.0);
    let temp = path.with_extension(format!("webp.{}.part", uuid::Uuid::now_v7()));
    std::fs::write(&temp, &*bytes)?;
    std::fs::rename(temp, path)?;
    Ok(())
}

// @req RNF-12
pub fn normalize_webp(path: &Path) -> AppResult<PathBuf> {
    let out = path.with_extension("webp");
    if !path.exists() && out.is_file() {
        return Ok(out);
    }
    let image = image::open(path)
        .map_err(|e| crate::error::AppError::invalid(format!("capa inválida: {e}")))?;
    write_webp(&compact_image(image, false), &out)?;
    if out != path {
        let _ = std::fs::remove_file(path);
    }
    Ok(out)
}

// @req RF-12
pub fn jpeg_for_tags(path: &Path) -> Option<Vec<u8>> {
    let image = image::open(path).ok()?.to_rgb8();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 82)
        .encode_image(&image)
        .ok()?;
    Some(bytes)
}

/// Preferências do player que o `Settings` do núcleo não tem (hoje só o mudo). Ficam num JSON
/// ao lado do banco; ausente ou corrompido vira o padrão.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub muted: bool,
}

impl Prefs {
    pub fn load(path: &Path) -> Prefs {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> AppResult<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(self)
            .map_err(|e| crate::error::AppError::internal(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-02
    #[test]
    fn prefs_persistem_e_toleram_arquivo_ruim() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("prefs.json");
        assert_eq!(Prefs::load(&f), Prefs::default());
        Prefs { muted: true }.save(&f).unwrap();
        assert!(Prefs::load(&f).muted);
        std::fs::write(&f, "{lixo").unwrap();
        assert_eq!(Prefs::load(&f), Prefs::default());
    }

    #[test]
    fn dirs_so_apagam_o_que_e_do_app() {
        let d = Dirs::new("/dados/miupy");
        assert!(d.owns(Path::new("/dados/miupy/audio/a.m4a")));
        assert!(!d.owns(Path::new("/home/user/Musicas/a.m4a")));
        assert_eq!(d.db_file, PathBuf::from("/dados/miupy/miupy.db"));
    }

    // @test RF-21 RF-22
    #[test]
    fn upload_recorta_normaliza_webp_e_rejeita_entrada_invalida() {
        use base64::Engine;
        let tmp = tempfile::tempdir().unwrap();
        let original = image::DynamicImage::new_rgb8(800, 400);
        let mut bytes = std::io::Cursor::new(Vec::new());
        original
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let upload = crate::dto::ImageUpload {
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes.into_inner()),
            mime_type: "image/png".into(),
        };
        let path = save_upload(&upload, tmp.path(), "avatar").unwrap();
        assert_eq!(path.extension().unwrap(), "webp");
        let decoded = image::open(path).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (512, 512));
        let playlist = save_upload(&upload, tmp.path(), "playlist").unwrap();
        assert_eq!(playlist.extension().unwrap(), "webp");
        assert_eq!(image::open(playlist).unwrap().width(), 512);
        let bad = crate::dto::ImageUpload {
            data_base64: "%%%".into(),
            mime_type: "image/png".into(),
        };
        assert_eq!(
            save_upload(&bad, tmp.path(), "avatar").unwrap_err().code(),
            "invalid_input"
        );
    }

    // @test RNF-12
    #[test]
    fn webp_com_perda_limita_dimensoes_e_ocupa_menos_que_jpeg() {
        let mut pixels = image::RgbImage::new(1200, 800);
        for (x, y, pixel) in pixels.enumerate_pixels_mut() {
            let n = ((x * 13 + y * 17 + (x ^ y) * 7) % 37) as u8;
            *pixel = image::Rgb([((x / 3) % 256) as u8, ((y / 2) % 256) as u8, n]);
        }
        let original = image::DynamicImage::ImageRgb8(pixels);
        let wide = compact_image(original.clone(), false);
        assert_eq!((wide.width(), wide.height()), (600, 400));
        let square = compact_image(original, true);
        assert_eq!((square.width(), square.height()), (512, 512));
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.webp");
        write_webp(&wide, &path).unwrap();
        let webp = std::fs::read(&path).unwrap();
        assert_eq!(&webp[8..12], b"WEBP");
        let decoded = image::open(&path).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (600, 400));
        assert_ne!(decoded.to_rgb8(), wide.to_rgb8());
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
            .encode_image(&wide)
            .unwrap();
        assert!(
            webp.len() < jpeg.len(),
            "WebP: {}, JPEG: {}",
            webp.len(),
            jpeg.len()
        );
    }

    // @test RF-05 RF-12
    #[test]
    fn chave_do_album_prefere_grupo_e_normaliza_artista_album() {
        let mut rec = miupy_media::RecordingMatch {
            mbid: "rec".into(),
            title: "Faixa".into(),
            artist: " Daft   Punk ".into(),
            artist_mbid: None,
            album: Some(" Random  Access Memories ".into()),
            release_mbid: Some("release".into()),
            release_group_mbid: None,
            release_mbids: vec![],
            duration_ms: None,
            score: 100,
            genres: vec![],
            release_year: None,
            isrc: None,
        };
        let hash = album_key(&rec);
        assert_eq!(hash.len(), 64);
        rec.artist = "daft punk".into();
        rec.album = Some("random access memories".into());
        assert_eq!(album_key(&rec), hash);
        rec.release_group_mbid = Some("grupo".into());
        assert_eq!(album_key(&rec), "grupo");
    }
}
