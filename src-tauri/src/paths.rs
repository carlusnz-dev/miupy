//! Diretórios de dados do app e preferências pequenas que o núcleo não guarda.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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

// @req RF-21 RF-22
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
    let side = image.width().min(image.height());
    if side == 0 {
        return Err(crate::error::AppError::invalid("Imagem vazia."));
    }
    let x = (image.width() - side) / 2;
    let y = (image.height() - side) / 2;
    let image = image.crop_imm(x, y, side, side).resize_exact(
        512,
        512,
        image::imageops::FilterType::Lanczos3,
    );
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("{stem}-{}.jpg", uuid::Uuid::now_v7()));
    image
        .save_with_format(&path, image::ImageFormat::Jpeg)
        .map_err(|e| crate::error::AppError::invalid(e.to_string()))?;
    Ok(path)
}

pub fn normalize_jpeg(path: &Path) -> AppResult<PathBuf> {
    let image = image::open(path)
        .map_err(|e| crate::error::AppError::invalid(format!("capa inválida: {e}")))?;
    let out = path.with_extension("jpg");
    image
        .save_with_format(&out, image::ImageFormat::Jpeg)
        .map_err(|e| crate::error::AppError::invalid(e.to_string()))?;
    if out != path {
        let _ = std::fs::remove_file(path);
    }
    Ok(out)
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
    fn upload_recorta_normaliza_jpeg_e_rejeita_entrada_invalida() {
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
        let decoded = image::open(path).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (512, 512));
        let bad = crate::dto::ImageUpload {
            data_base64: "%%%".into(),
            mime_type: "image/png".into(),
        };
        assert_eq!(
            save_upload(&bad, tmp.path(), "avatar").unwrap_err().code(),
            "invalid_input"
        );
    }
}
