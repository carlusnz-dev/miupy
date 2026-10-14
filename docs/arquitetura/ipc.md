# Contrato de IPC (frontend ⇄ Rust)

Contrato entre `src/` (Svelte) e `src-tauri/` (comandos Tauri). Mudou aqui, muda dos dois lados.

Convenções:

- Comandos em `snake_case`, prefixados pelo domínio. Argumentos e campos em `camelCase`
  (`#[serde(rename_all = "camelCase")]`).
- IDs são `string` (UUID v7). Datas são `number` (epoch em milissegundos, UTC). Durações em ms.
- Erros voltam como `{ code: string, message: string }`. Códigos: `not_found`, `invalid_input`,
  `conflict`, `io`, `network`, `unavailable`, `internal`.

## Tipos

```ts
type LavaTheme = 'orange' | 'purple' | 'blue';
type SortDir = 'asc' | 'desc';
type TrackSort = 'title' | 'artist' | 'album' | 'duration' | 'addedAt' | 'position';
type DownloadStatus = 'none' | 'queued' | 'downloading' | 'done' | 'error';
type PlayerStatus = 'stopped' | 'loading' | 'playing' | 'paused';

interface Track {
  id: string;
  title: string;
  artistId: string | null;
  artistName: string | null;
  album: string | null;
  durationMs: number;
  filePath: string | null;      // null enquanto não baixada
  coverPath: string | null;     // caminho local da capa
  source: 'youtube' | 'local';
  sourceId: string | null;      // id do vídeo no YouTube
  codec: string | null;
  bitrate: number | null;
  isFavorite: boolean;
  downloadStatus: DownloadStatus;
  addedAt: number;
}

interface Playlist {
  id: string;
  name: string;
  description: string | null;
  coverPath: string | null;
  trackCount: number;
  totalDurationMs: number;
  createdAt: number;
  updatedAt: number;
}

interface TrackQuery { search?: string; sortBy?: TrackSort; sortDir?: SortDir; favoritesOnly?: boolean; }

interface PlayerState {
  status: PlayerStatus;
  track: Track | null;
  positionMs: number;
  durationMs: number;
  volume: number;               // 0.0 a 1.0
  muted: boolean;
  queue: string[];              // ids das faixas
  queueIndex: number;           // -1 se vazia
}

interface SearchResult {
  sourceId: string;
  title: string;
  artist: string;
  durationMs: number;
  thumbnailUrl: string | null;
  inLibrary: boolean;
}

interface DownloadJob {
  id: string;
  trackId: string;
  status: DownloadStatus;
  progress: number;             // 0.0 a 1.0
  error: string | null;
}

interface ArtistInfo {
  id: string;
  name: string;
  mbid: string | null;
  country: string | null;
  genres: string[];
  bio: string | null;
  imagePath: string | null;
  trackCount: number;
}

interface Lyrics { trackId: string; plain: string | null; synced: string | null; }

interface Profile {
  displayName: string;
  username: string;
  email: string;
  phone: string;
  avatarPath: string | null;
}

interface Settings { lavaTheme: LavaTheme; volume: number; }

interface RankedTrack { track: Track; plays: number; listenedMs: number; }
interface RankedArtist { artistId: string; name: string; plays: number; listenedMs: number; }

interface LibraryStats {
  trackCount: number;
  downloadedCount: number;
  playlistCount: number;
  artistCount: number;
  favoriteCount: number;
  totalDurationMs: number;
  totalListenedMs: number;
  totalPlays: number;
  topTracks: RankedTrack[];
}

interface Wrapped {
  year: number;
  totalListenedMs: number;
  totalPlays: number;
  distinctTracks: number;
  distinctArtists: number;
  tracksAdded: number;
  topTracks: RankedTrack[];
  topArtists: RankedArtist[];
  listenedMsByMonth: number[];  // 12 posições, jan a dez
  busiestDay: { date: string; listenedMs: number } | null;  // date em YYYY-MM-DD
}
```

## Comandos

| Comando | Argumentos | Retorno | Requisitos |
|---|---|---|---|
| `player_get_state` | — | `PlayerState` | RF-01 |
| `player_play` | `{ trackId, queue?: string[] }` | `PlayerState` | RF-01, RF-03 |
| `player_pause` | — | `PlayerState` | RF-01 |
| `player_resume` | — | `PlayerState` | RF-01 |
| `player_stop` | — | `PlayerState` | RF-01 |
| `player_seek` | `{ positionMs }` | `PlayerState` | RF-03 |
| `player_next` | — | `PlayerState` | RF-03 |
| `player_previous` | — | `PlayerState` | RF-03 |
| `player_set_volume` | `{ volume }` | `PlayerState` | RF-02 |
| `player_set_muted` | `{ muted }` | `PlayerState` | RF-02 |
| `library_list_tracks` | `{ query?: TrackQuery }` | `Track[]` | RF-05, RF-08 |
| `library_get_track` | `{ trackId }` | `Track` | RF-05 |
| `library_set_favorite` | `{ trackId, favorite }` | `Track` | RF-04 |
| `library_delete_track` | `{ trackId, deleteFile }` | `void` | RF-06 |
| `playlist_list` | — | `Playlist[]` | RF-06 |
| `playlist_create` | `{ name, description? }` | `Playlist` | RF-06 |
| `playlist_update` | `{ playlistId, name?, description? }` | `Playlist` | RF-06 |
| `playlist_delete` | `{ playlistId }` | `void` | RF-06 |
| `playlist_get_tracks` | `{ playlistId, query?: TrackQuery }` | `Track[]` | RF-07, RF-08 |
| `playlist_add_tracks` | `{ playlistId, trackIds }` | `Playlist` | RF-06 |
| `playlist_remove_tracks` | `{ playlistId, trackIds }` | `Playlist` | RF-06 |
| `playlist_move_track` | `{ playlistId, trackId, newPosition }` | `void` | RF-07 |
| `youtube_search` | `{ query, limit? }` | `SearchResult[]` | RF-09 |
| `youtube_add_to_library` | `{ sourceIds, playlistId? }` | `Track[]` | RF-09 |
| `download_enqueue` | `{ trackIds }` | `DownloadJob[]` | RF-10 |
| `download_enqueue_playlist` | `{ playlistId }` | `DownloadJob[]` | RF-11 |
| `download_list` | — | `DownloadJob[]` | RF-10 |
| `download_cancel` | `{ jobId }` | `void` | RF-10 |
| `metadata_refresh_track` | `{ trackId }` | `Track` | RF-12 |
| `metadata_get_artist` | `{ artistId }` | `ArtistInfo` | RF-12 |
| `metadata_get_lyrics` | `{ trackId }` | `Lyrics` | RF-12 |
| `stats_library` | — | `LibraryStats` | RF-14 |
| `stats_wrapped` | `{ year }` | `Wrapped` | RF-15 |
| `profile_get` | — | `Profile` | RF-16 |
| `profile_update` | `{ profile: Profile }` | `Profile` | RF-16 |
| `settings_get` | — | `Settings` | RF-17 |
| `settings_update` | `{ settings: Partial<Settings> }` | `Settings` | RF-17 |

## Eventos (Rust → frontend)

| Evento | Payload | Quando |
|---|---|---|
| `player://state` | `PlayerState` | Mudança de status, faixa, volume ou fila. |
| `player://progress` | `{ positionMs, durationMs }` | A cada ~500 ms enquanto toca. |
| `download://progress` | `DownloadJob` | Mudança de status ou progresso de um download. |
| `library://changed` | `{ reason: string }` | Faixas ou playlists criadas, alteradas ou removidas. |

## Regras de comportamento

- `player_play` em faixa sem arquivo local (`filePath` nulo) devolve erro `unavailable`.
- `coverPath`, `imagePath` e `avatarPath` são caminhos locais. O frontend converte com
  `convertFileSrc`, então `src-tauri` habilita o protocolo `asset` com escopo no diretório de dados.
- `stats_wrapped` agrupa por mês e dia no horário local. O fuso é calculado pela casca (relógio do
  sistema), não enviado pelo frontend.
- Conta como "play" o evento concluído ou com pelo menos 30 s ouvidos.
- "Tentar de novo" em um download com erro é um novo `download_enqueue` da mesma faixa.

## Lacunas conhecidas

Levantadas pelo frontend na sessão 01 e ainda sem comando próprio: faixas por artista (hoje filtra
`library_list_tracks` no cliente), fila com `Track[]` em vez de ids, anos disponíveis para a
retrospectiva, limpar downloads concluídos e envio de avatar.

O registro de atividade (RF-13) não tem comando próprio: o backend grava os eventos de reprodução
sozinho, a partir do estado do player.
