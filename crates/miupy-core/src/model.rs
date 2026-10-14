//! Tipos de domínio. Espelham `docs/arquitetura/ipc.md`: serde em camelCase, IDs UUID v7 em
//! texto, datas em epoch ms (UTC), durações em ms.

use rusqlite::ToSql;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use serde::{Deserialize, Deserializer, Serialize};

/// Enum textual guardado como TEXT no SQLite (ToSql/FromSql) e em JSON (serde).
macro_rules! text_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename = $text)] $variant),+ }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }
        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(self.as_str().into())
            }
        }
        impl FromSql for $name {
            fn column_result(v: ValueRef<'_>) -> FromSqlResult<Self> {
                match v.as_str()? {
                    $($text => Ok(Self::$variant),)+
                    other => Err(FromSqlError::Other(
                        format!("valor inválido para {}: {other}", stringify!($name)).into(),
                    )),
                }
            }
        }
    };
}

text_enum!(
    /// Cor da lava (RF-17). `Orange` é o padrão.
    LavaTheme { Orange => "orange", Purple => "purple", Blue => "blue" }
);
text_enum!(
    /// Origem da faixa.
    Source { Youtube => "youtube", Local => "local" }
);
text_enum!(
    /// Estado do download de uma faixa ou de um job.
    DownloadStatus {
        None => "none",
        Queued => "queued",
        Downloading => "downloading",
        Done => "done",
        Error => "error",
    }
);

// @req RF-24
text_enum!(
    /// Ordem de reprodução salva separadamente de `Settings` do IPC.
    ShuffleMode { Off => "off", Random => "random", Smart => "smart" }
);

// @req RNF-09
text_enum!(
    /// Aceleração de hardware escolhida pelo usuário; a casca aplica na próxima abertura.
    GpuMode { Auto => "auto", On => "on", Off => "off" }
);

#[allow(clippy::derivable_impls)] // o macro de enum não emite `#[default]`
impl Default for LavaTheme {
    fn default() -> Self {
        LavaTheme::Orange
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackSort {
    Title,
    Artist,
    Album,
    Duration,
    AddedAt,
    /// Só faz sentido dentro de uma playlist; na biblioteca vira `AddedAt`.
    Position,
}

/// Deserializa `Option<Option<T>>`: campo ausente = `None` (não mexe), `null` = `Some(None)`
/// (limpa), valor = `Some(Some(v))`.
pub(crate) fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

// ------------------------------------------------------------------ faixas

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist_id: Option<String>,
    pub artist_name: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    /// `None` enquanto a faixa não foi baixada.
    pub file_path: Option<String>,
    pub cover_path: Option<String>,
    pub source: Source,
    pub source_id: Option<String>,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    pub is_favorite: bool,
    pub download_status: DownloadStatus,
    pub added_at: i64,
    // @req RF-20
    pub genres: Vec<String>,
    pub release_year: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackQuery {
    pub search: Option<String>,
    pub sort_by: Option<TrackSort>,
    pub sort_dir: Option<SortDir>,
    pub favorites_only: Option<bool>,
}

/// Dados para criar (ou localizar) uma faixa. Faixas do YouTube são deduplicadas por
/// `(source, source_id)`; faixas locais por `file_path`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTrack {
    pub source: Source,
    pub source_id: Option<String>,
    pub title: String,
    pub artist_name: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub file_path: Option<String>,
    pub cover_path: Option<String>,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    /// Miniatura remota (resultado do YouTube); serve para baixar a capa depois.
    pub thumbnail_url: Option<String>,
    // @req RF-20
    pub genres: Vec<String>,
    pub release_year: Option<i32>,
    pub mbid: Option<String>,
    pub isrc: Option<String>,
}

impl NewTrack {
    /// Faixa vinda de um resultado de busca do YouTube, ainda sem arquivo.
    pub fn youtube(
        source_id: impl Into<String>,
        title: impl Into<String>,
        artist: Option<String>,
        duration_ms: i64,
        thumbnail_url: Option<String>,
    ) -> Self {
        NewTrack {
            source: Source::Youtube,
            source_id: Some(source_id.into()),
            title: title.into(),
            artist_name: artist,
            album: None,
            duration_ms,
            file_path: None,
            cover_path: None,
            codec: None,
            bitrate: None,
            thumbnail_url,
            genres: Vec::new(),
            release_year: None,
            mbid: None,
            isrc: None,
        }
    }

    /// Arquivo local já existente.
    pub fn local(file_path: impl Into<String>, title: impl Into<String>) -> Self {
        NewTrack {
            source: Source::Local,
            source_id: None,
            title: title.into(),
            artist_name: None,
            album: None,
            duration_ms: 0,
            file_path: Some(file_path.into()),
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
}

/// Resultado de [`crate::Database::upsert_track`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Upserted {
    pub track: Track,
    /// `false` quando a faixa já existia (deduplicada).
    pub created: bool,
}

/// Alteração parcial de metadados. Campos anuláveis usam `Option<Option<_>>`:
/// ausente = mantém, `null` = limpa.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackPatch {
    pub title: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub artist_name: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub album: Option<Option<String>>,
    pub duration_ms: Option<i64>,
    #[serde(deserialize_with = "double_option")]
    pub cover_path: Option<Option<String>>,
    // @req RF-20
    pub genres: Option<Vec<String>>,
    #[serde(deserialize_with = "double_option")]
    pub release_year: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub mbid: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub isrc: Option<Option<String>>,
}

// @req RF-19
text_enum!(
    /// Origem de uma candidata encontrada pela casca.
    RecommendationReasonKind {
        SimilarTrack => "similarTrack",
        Artist => "artist",
        Genre => "genre",
    }
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationReason {
    pub kind: RecommendationReasonKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: i64,
    pub thumbnail_url: Option<String>,
    pub score: f64,
    pub reasons: Vec<RecommendationReason>,
}

/// Resultado externo que a casca obteve para uma semente de `recommendation_seeds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationCandidate {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: i64,
    pub thumbnail_url: Option<String>,
    pub genres: Vec<String>,
    pub seed_kind: RecommendationReasonKind,
    pub seed_label: String,
    pub seed_track_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationSeed {
    pub kind: RecommendationReasonKind,
    pub label: String,
    /// `sourceId` só existe em sementes de faixa.
    pub source_id: Option<String>,
    pub track_id: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasteArtist {
    pub artist_id: String,
    pub name: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasteGenre {
    pub genre: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasteProfile {
    pub top_artists: Vec<TasteArtist>,
    pub top_genres: Vec<TasteGenre>,
    pub total_listened_ms: i64,
    pub skip_rate: f64,
}

// @req RF-25
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackScore {
    pub track_id: String,
    pub score: f64,
    pub plays: i64,
    pub skips: i64,
    pub last_played_at: Option<i64>,
}

// @req RF-25
text_enum!(
    /// Tipo de tendência do DNA.
    DnaTrendKind { Artist => "artist", Genre => "genre" }
);

// @req RF-25
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnaTrend {
    pub kind: DnaTrendKind,
    pub id: Option<String>,
    pub label: String,
    pub current: f64,
    pub previous: f64,
    pub delta: f64,
}

// @req RF-25
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dna {
    pub window_days: u32,
    pub top_artists: Vec<TasteArtist>,
    pub top_genres: Vec<TasteGenre>,
    pub skip_rate: f64,
    pub previous_skip_rate: f64,
    pub listened_ms: i64,
    pub previous_listened_ms: i64,
    pub rising: Vec<DnaTrend>,
    pub falling: Vec<DnaTrend>,
}

// @req RF-24
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartTrack {
    pub track_id: String,
    pub score: f64,
    pub artist_id: Option<String>,
    pub genres: Vec<String>,
}

/// Dados do arquivo ao fim de um download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedFile {
    pub file_path: String,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    /// Só substitui a capa atual quando vier preenchido.
    pub cover_path: Option<String>,
    /// Só substitui a duração atual quando vier preenchido (> 0).
    pub duration_ms: Option<i64>,
}

/// O que ficou no disco de uma faixa apagada, para o chamador remover os arquivos.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedTrack {
    pub track_id: String,
    pub file_path: Option<String>,
    pub cover_path: Option<String>,
}

// ---------------------------------------------------------------- artistas

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub mbid: Option<String>,
    pub country: Option<String>,
    pub genres: Vec<String>,
    pub bio: Option<String>,
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
    pub mbid: Option<String>,
    pub country: Option<String>,
    pub genres: Vec<String>,
    pub bio: Option<String>,
    pub image_path: Option<String>,
    // @req RF-27
    pub image_credit: Option<String>,
    pub track_count: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArtistPatch {
    pub name: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub mbid: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub country: Option<Option<String>>,
    pub genres: Option<Vec<String>>,
    #[serde(deserialize_with = "double_option")]
    pub bio: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub image_path: Option<Option<String>>,
}

// --------------------------------------------------------------- playlists

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_path: Option<String>,
    // @req RF-22
    pub mosaic_paths: Vec<String>,
    pub track_count: i64,
    pub total_duration_ms: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// `name`/`description` seguem o ipc (`name?`, `description?`). Descrição vazia ou `null`
/// limpa o campo.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaylistPatch {
    pub name: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub cover_path: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub track_id: String,
    pub plain: Option<String>,
    pub synced: Option<String>,
}

// --------------------------------------------------------------- downloads

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub track_id: String,
    pub status: DownloadStatus,
    /// 0.0 a 1.0
    pub progress: f64,
    pub error: Option<String>,
}

/// Atualização de um job pelo orquestrador de downloads. `status` não pode ser `none` nem
/// `done` (o fim bem-sucedido passa por [`crate::Database::mark_downloaded`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadUpdate {
    pub status: DownloadStatus,
    pub progress: Option<f64>,
    pub error: Option<String>,
}

// --------------------------------------------------------------- atividade

/// Como um evento de reprodução terminou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayEnd {
    /// Tocou até o fim.
    Completed,
    /// O usuário trocou de faixa antes do fim.
    Skipped,
    /// Parou / fechou o app, sem concluir nem pular.
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEvent {
    pub id: String,
    pub track_id: String,
    pub playlist_id: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub listened_ms: i64,
    pub completed: bool,
    pub skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAction {
    pub id: String,
    pub kind: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub created_at: i64,
}

// ------------------------------------------------------------ estatísticas

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedTrack {
    pub track: Track,
    pub plays: i64,
    pub listened_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedArtist {
    pub artist_id: String,
    pub name: String,
    pub plays: i64,
    pub listened_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub track_count: i64,
    pub downloaded_count: i64,
    pub playlist_count: i64,
    pub artist_count: i64,
    pub favorite_count: i64,
    pub total_duration_ms: i64,
    pub total_listened_ms: i64,
    pub total_plays: i64,
    pub top_tracks: Vec<RankedTrack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusiestDay {
    /// `YYYY-MM-DD`, no fuso informado.
    pub date: String,
    pub listened_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wrapped {
    pub year: i32,
    pub total_listened_ms: i64,
    pub total_plays: i64,
    pub distinct_tracks: i64,
    pub distinct_artists: i64,
    pub tracks_added: i64,
    pub top_tracks: Vec<RankedTrack>,
    pub top_artists: Vec<RankedArtist>,
    /// 12 posições, janeiro a dezembro.
    pub listened_ms_by_month: Vec<i64>,
    pub busiest_day: Option<BusiestDay>,
}

// ---------------------------------------------------------- perfil e config.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub phone: String,
    pub avatar_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub lava_theme: LavaTheme,
    /// 0.0 a 1.0
    pub volume: f64,
}

/// `Partial<Settings>` do ipc.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub lava_theme: Option<LavaTheme>,
    pub volume: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-17
    #[test]
    fn settings_serializa_em_camel_case() {
        let s = Settings {
            lava_theme: LavaTheme::Purple,
            volume: 0.5,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "lavaTheme": "purple", "volume": 0.5 })
        );
        assert_eq!(LavaTheme::default(), LavaTheme::Orange);
    }

    // @test RF-08
    #[test]
    fn track_query_aceita_campos_ausentes_e_camel_case() {
        let q: TrackQuery = serde_json::from_str(
            r#"{"search":"x","sortBy":"addedAt","sortDir":"desc","favoritesOnly":true}"#,
        )
        .unwrap();
        assert_eq!(q.sort_by, Some(TrackSort::AddedAt));
        assert_eq!(q.sort_dir, Some(SortDir::Desc));
        assert_eq!(q.favorites_only, Some(true));
        let vazia: TrackQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(vazia, TrackQuery::default());
    }

    // @test RF-06
    #[test]
    fn patch_distingue_ausente_de_null() {
        let p: PlaylistPatch = serde_json::from_str(r#"{"name":"a"}"#).unwrap();
        assert_eq!(p.description, None);
        let p: PlaylistPatch = serde_json::from_str(r#"{"description":null}"#).unwrap();
        assert_eq!(p.description, Some(None));
        let p: PlaylistPatch = serde_json::from_str(r#"{"description":"x"}"#).unwrap();
        assert_eq!(p.description, Some(Some("x".into())));
    }
}
