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
        Ok(())
    }

    /// O caminho está dentro do diretório de dados? (só apagamos arquivos nossos)
    pub fn owns(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
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
}
