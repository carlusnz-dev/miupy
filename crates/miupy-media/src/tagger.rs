//! Leitura e escrita de tags (título, artista, álbum, capa) com `lofty`.
//!
//! Funções síncronas e com E/S de disco: chame de `tokio::task::spawn_blocking` num contexto
//! assíncrono. Funciona em todos os formatos que o lofty suporta (M4A/MP4, MP3, FLAC, Ogg, Opus,
//! WAV...); o M4A/AAC é o formato padrão do projeto.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};

use crate::error::{MediaError, Result};

/// Imagem de capa embutida ou a embutir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    /// "image/jpeg", "image/png"...
    pub mime: String,
    pub data: Vec<u8>,
}

/// Tags e propriedades de um arquivo de áudio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genres: Vec<String>,
    pub release_year: Option<u32>,
    pub duration_ms: u64,
    /// "aac" (MP4/M4A; um M4A ALAC também aparece como "aac"), "mp3", "flac", "vorbis", "opus"...
    pub codec: String,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate: Option<u32>,
    pub cover: Option<Cover>,
}

/// Campos a gravar; `None` mantém o valor atual.
#[derive(Debug, Clone, Default)]
pub struct TagUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Lista vazia remove o gênero; `None` preserva a tag atual.
    pub genres: Option<Vec<String>>,
    pub release_year: Option<u32>,
    /// Bytes de uma imagem JPEG/PNG (formato detectado pelo conteúdo).
    pub cover: Option<Vec<u8>>,
}

fn codec_name(ft: FileType) -> &'static str {
    match ft {
        FileType::Mp4 | FileType::Aac => "aac",
        FileType::Mpeg => "mp3",
        FileType::Flac => "flac",
        FileType::Vorbis => "vorbis",
        FileType::Opus => "opus",
        FileType::Speex => "speex",
        FileType::Wav => "wav",
        FileType::Aiff => "aiff",
        FileType::Ape => "ape",
        FileType::Mpc => "musepack",
        FileType::WavPack => "wavpack",
        _ => "unknown",
    }
}

fn non_empty(s: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn normalize_genres<'a>(genres: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    genres
        .into_iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && seen.insert(s.clone()))
        .collect()
}

fn open(path: &Path) -> Result<lofty::file::TaggedFile> {
    Probe::open(path)
        .map_err(|e| MediaError::Io(format!("{}: {e}", path.display())))?
        .guess_file_type()
        .map_err(|e| MediaError::Io(format!("{}: {e}", path.display())))?
        .read()
        .map_err(|e| MediaError::Internal(format!("arquivo de áudio ilegível: {e}")))
}

// @req RF-05 RF-20
/// Lê tags e propriedades. Arquivo sem tags devolve campos `None` (não é erro).
pub fn read_tags(path: &Path) -> Result<AudioInfo> {
    let file = open(path)?;
    let props = file.properties();
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let cover = tag.and_then(|t| {
        t.pictures()
            .iter()
            .find(|p| p.pic_type() == PictureType::CoverFront)
            .or_else(|| t.pictures().first())
            .map(|p| Cover {
                mime: p
                    .mime_type()
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "application/octet-stream".into()),
                data: p.data().to_vec(),
            })
    });
    Ok(AudioInfo {
        title: tag.and_then(|t| non_empty(t.title())),
        artist: tag.and_then(|t| non_empty(t.artist())),
        album: tag.and_then(|t| non_empty(t.album())),
        genres: tag
            .and_then(|t| non_empty(t.genre()))
            .map(|s| normalize_genres(s.split(';')))
            .unwrap_or_default(),
        release_year: tag
            .and_then(|t| {
                t.get_string(ItemKey::RecordingDate)
                    .or_else(|| t.get_string(ItemKey::Year))
            })
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok()),
        duration_ms: props.duration().as_millis() as u64,
        codec: codec_name(file.file_type()).to_string(),
        bitrate_kbps: props.audio_bitrate().or(props.overall_bitrate()),
        sample_rate: props.sample_rate(),
        cover,
    })
}

// @req RF-05 RF-12 RF-20
/// Grava título/artista/álbum/capa no formato de tag nativo do arquivo.
pub fn write_tags(path: &Path, update: &TagUpdate) -> Result<()> {
    let mut file = open(path)?;
    let tag_type = file.primary_tag_type();
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(tag_type));
    }
    let tag = file
        .primary_tag_mut()
        .ok_or_else(|| MediaError::Internal("formato sem tag gravável".into()))?;

    if let Some(v) = &update.title {
        tag.set_title(v.clone());
    }
    if let Some(v) = &update.artist {
        tag.set_artist(v.clone());
    }
    if let Some(v) = &update.album {
        tag.set_album(v.clone());
    }
    if let Some(genres) = &update.genres {
        let genres = normalize_genres(genres.iter().map(String::as_str));
        if genres.is_empty() {
            tag.remove_genre();
        } else {
            tag.set_genre(genres.join("; "));
        }
    }
    if let Some(year) = update.release_year {
        if !(1000..=3000).contains(&year) {
            return Err(MediaError::InvalidInput(
                "ano de lançamento inválido".into(),
            ));
        }
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    if let Some(data) = &update.cover {
        let mut pic = Picture::from_reader(&mut data.as_slice())
            .map_err(|e| MediaError::InvalidInput(format!("capa não é uma imagem válida: {e}")))?;
        pic.set_pic_type(PictureType::CoverFront);
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(pic);
    }
    file.save_to_path(path, WriteOptions::default())
        .map_err(|e| MediaError::Io(format!("gravar tags em {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// PNG 1x1 mínimo válido.
    pub(crate) const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    /// Gera 1 s de tom com ffmpeg. `None` (com aviso) se não houver ffmpeg.
    fn make_audio(dir: &Path, name: &str, codec_args: &[&str]) -> Option<std::path::PathBuf> {
        let out = dir.join(name);
        let status = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
            ])
            .args(codec_args)
            .arg(&out)
            .status();
        match status {
            Ok(s) if s.success() => Some(out),
            _ => {
                eprintln!("PULADO: ffmpeg indisponível ou falhou ao gerar {name}");
                None
            }
        }
    }

    // @test RF-05 RF-20
    #[test]
    fn m4a_le_propriedades_e_ida_e_volta_de_tags() {
        let dir = tempfile::tempdir().unwrap();
        let Some(f) = make_audio(dir.path(), "t.m4a", &["-c:a", "aac", "-b:a", "128k"]) else {
            return;
        };
        let info = read_tags(&f).unwrap();
        assert_eq!(info.codec, "aac");
        assert!(
            (900..=1200).contains(&info.duration_ms),
            "{}",
            info.duration_ms
        );
        assert!(info.bitrate_kbps.is_some());
        assert_eq!(info.title, None);
        assert_eq!(info.cover, None);

        write_tags(
            &f,
            &TagUpdate {
                title: Some("Música".into()),
                artist: Some("Artista".into()),
                album: Some("Álbum".into()),
                cover: Some(PNG_1X1.to_vec()),
                genres: Some(vec!["Rock".into(), "Pop".into(), "rock".into()]),
                release_year: Some(2020),
            },
        )
        .unwrap();
        let info = read_tags(&f).unwrap();
        assert_eq!(info.title.as_deref(), Some("Música"));
        assert_eq!(info.artist.as_deref(), Some("Artista"));
        assert_eq!(info.album.as_deref(), Some("Álbum"));
        assert_eq!(info.genres, ["rock", "pop"]);
        assert_eq!(info.release_year, Some(2020));
        let cover = info.cover.unwrap();
        assert_eq!(cover.mime, "image/png");
        assert_eq!(cover.data, PNG_1X1);
        assert!((900..=1200).contains(&info.duration_ms));

        // atualização parcial mantém o resto
        write_tags(
            &f,
            &TagUpdate {
                title: Some("Outro".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let info = read_tags(&f).unwrap();
        assert_eq!(info.title.as_deref(), Some("Outro"));
        assert_eq!(info.artist.as_deref(), Some("Artista"));
        assert_eq!(info.genres, ["rock", "pop"]);
        assert_eq!(info.release_year, Some(2020));
        assert!(info.cover.is_some());
    }

    // @test RF-05 RF-20
    #[test]
    fn mp3_e_flac_tambem_funcionam() {
        let dir = tempfile::tempdir().unwrap();
        for (name, args) in [
            ("t.mp3", &["-c:a", "libmp3lame", "-b:a", "128k"][..]),
            ("t.flac", &["-c:a", "flac"][..]),
        ] {
            let Some(f) = make_audio(dir.path(), name, args) else {
                continue;
            };
            write_tags(
                &f,
                &TagUpdate {
                    title: Some("T".into()),
                    artist: Some("A".into()),
                    album: Some("B".into()),
                    cover: Some(PNG_1X1.to_vec()),
                    genres: Some(vec!["Jazz".into()]),
                    release_year: Some(1998),
                },
            )
            .unwrap();
            let info = read_tags(&f).unwrap();
            assert_eq!(info.title.as_deref(), Some("T"), "{name}");
            assert_eq!(info.artist.as_deref(), Some("A"), "{name}");
            assert!(info.cover.is_some(), "{name}");
            assert_eq!(info.genres, ["jazz"], "{name}");
            assert_eq!(info.release_year, Some(1998), "{name}");
        }
    }

    // @test RF-05
    #[test]
    fn erros_tipados() {
        let dir = tempfile::tempdir().unwrap();
        let e = read_tags(&dir.path().join("nao-existe.m4a")).unwrap_err();
        assert!(matches!(e.code(), "io" | "not_found"), "{e}");
        let lixo = dir.path().join("lixo.m4a");
        std::fs::write(&lixo, b"isto nao e audio nenhum, so texto").unwrap();
        assert!(read_tags(&lixo).is_err());

        let Some(f) = make_audio(dir.path(), "t.m4a", &["-c:a", "aac"]) else {
            return;
        };
        let e = write_tags(
            &f,
            &TagUpdate {
                cover: Some(b"nao e imagem".to_vec()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), "invalid_input");
    }
}
