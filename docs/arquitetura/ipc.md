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
type ShuffleMode = 'off' | 'random' | 'surfle';   // RF-24

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
  genres: string[];             // minúsculas, sem repetição; vazio se desconhecido (RF-20)
  releaseYear: number | null;   // RF-20
  loudnessLufs: number | null;  // loudness integrado EBU R128; null se ainda não medida (RF-28)
  thumbnailUrl: string | null;  // miniatura remota (YouTube); capa de quem ainda não tem coverPath
}

interface Playlist {
  id: string;
  name: string;
  description: string | null;
  coverPath: string | null;     // capa escolhida pelo usuário; null usa o mosaico (RF-22)
  mosaicPaths: string[];        // 0 a 4 caminhos locais, um por artista, para o mosaico padrão (RF-22)
  trackCount: number;
  totalDurationMs: number;
  autoDownload: boolean;        // baixa sozinha o que falta e o que entrar (RF-31)
  isFavorites: boolean;         // é a playlist marcada como favoritas (RF-30)
  createdAt: number;
  updatedAt: number;
}

interface TrackQuery { search?: string; sortBy?: TrackSort; sortDir?: SortDir; favoritesOnly?: boolean; }

interface QueueContext { kind: 'playlist' | 'artist' | 'library' | 'other'; id: string | null; label: string; }

interface PlayerState {
  status: PlayerStatus;
  track: Track | null;
  positionMs: number;
  durationMs: number;
  volume: number;               // 0.0 a 1.0
  muted: boolean;
  queue: string[];              // ids das faixas
  context: QueueContext | null;  // origem da fila nesta abertura; não é persistida
  queueIndex: number;           // -1 se vazia
  streaming: boolean;           // true quando a faixa atual toca da rede, sem arquivo local (RF-18)
  shuffle: ShuffleMode;         // RF-24
}

interface SearchResult {
  sourceId: string;
  title: string;
  artist: string;
  durationMs: number;
  thumbnailUrl: string | null;
  inLibrary: boolean;
  trackId: string | null;       // faixa da discoteca com este sourceId, se houver (RF-04)
  isFavorite: boolean;          // false se não está na discoteca (RF-04)
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
  imageCredit: string | null;   // autor e licença da foto (Wikimedia Commons), para exibir junto (RF-27)
  trackCount: number;
  fetchedAt: number | null;     // última atualização bem-sucedida dos dados online (RF-35)
}

interface Lyrics { trackId: string; plain: string | null; synced: string | null; }

interface Profile {
  displayName: string;
  username: string;
  email: string;
  phone: string;
  avatarPath: string | null;
}

type GpuMode = 'auto' | 'on' | 'off';   // RNF-09

type Normalization = 'off' | 'quiet' | 'normal' | 'loud';   // RF-28: −19, −14, −11 LUFS

interface Settings {
  lavaTheme: LavaTheme;
  volume: number;
  gpuMode: GpuMode;             // vale a partir da próxima abertura do app (RNF-09)
  normalization: Normalization;     // padrão 'normal' (RF-28)
  eqPresetId: string;                   // padrão 'flat' (RF-29)
  favoritesPlaylistId: string | null;   // só leitura aqui; muda por favorites_set_playlist (RF-30)
  artistAutoRefresh: boolean;           // padrão true (RF-35)
  debugMode: boolean;                   // padrão false (RF-37)
  sidebarWidth: number;                 // px, 220 a 400, padrão 280 (RF-33)
}

interface GpuStatus {
  mode: GpuMode;                // modo salvo
  envOverride: GpuMode | null;  // MIUPY_GPU, quando definida, manda sobre o salvo
  hardware: boolean;            // a sessão atual está compondo pela GPU
  reason: string;               // motivo da sessão, inclusive recuo após queda anterior
  restartRequired: boolean;     // o modo salvo difere do que está valendo agora
}

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

// ---- Recomendações (RF-19)
type RecommendationReasonKind = 'similarTrack' | 'artist' | 'genre';

interface RecommendationReason {
  kind: RecommendationReasonKind;
  label: string;                // título da faixa parecida, nome do artista ou gênero
}

interface Recommendation {
  sourceId: string;             // id do vídeo no YouTube; nunca está na discoteca
  title: string;
  artist: string;
  durationMs: number;
  thumbnailUrl: string | null;
  score: number;                // 0.0 a 1.0, maior é melhor
  reasons: RecommendationReason[];  // 1 a 3, da mais forte para a mais fraca
}

interface TasteProfile {
  topArtists: { artistId: string; name: string; score: number }[];
  topGenres: { genre: string; score: number }[];
  totalListenedMs: number;
  skipRate: number;             // 0.0 a 1.0: eventos pulados / eventos encerrados
}

// ---- Pontuação e DNA (RF-25)
interface TrackScore {
  trackId: string;
  score: number;                // pontuação da faixa (recomendacao.md); pode ser negativa
  plays: number;                // eventos que contam como "play"
  skips: number;
  lastPlayedAt: number | null;
}

interface DnaTrend {
  kind: 'artist' | 'genre';
  id: string | null;            // artistId para 'artist'; null para 'genre'
  label: string;
  current: number;              // saldo na janela atual
  previous: number;             // saldo na janela anterior
  delta: number;                // current - previous
}

interface Dna {
  windowDays: number;           // tamanho de cada janela
  topArtists: { artistId: string; name: string; score: number }[];   // janela atual, até 10
  topGenres: { genre: string; score: number }[];                     // janela atual, até 10
  skipRate: number;             // janela atual
  previousSkipRate: number;     // janela anterior
  listenedMs: number;           // janela atual
  previousListenedMs: number;
  rising: DnaTrend[];           // até 5, maior delta positivo primeiro
  falling: DnaTrend[];          // até 5, delta mais negativo primeiro
}

// ---- Imagens enviadas pelo usuário (RF-21, RF-22)
interface ImageUpload {
  dataBase64: string;           // conteúdo do arquivo, sem o prefixo "data:"
  mimeType: string;             // image/jpeg, image/png ou image/webp
}

// ---- yt-dlp gerenciado pelo app (RF-26)
type YtDlpState = 'missing' | 'downloading' | 'ready' | 'error';

interface YtDlpStatus {
  state: YtDlpState;
  version: string | null;       // ex.: "2026.08.19"
  path: string | null;          // binário em uso
  checkedAt: number | null;     // última verificação de versão nova
  progress: number;             // 0.0 a 1.0 enquanto 'downloading'
  error: string | null;
}

// ---- Equalizador (RF-29)
interface EqPreset {
  id: string;                   // prontos: 'flat', 'bass', 'treble', 'vocal', 'rock', 'pop',
                                // 'electronic', 'acoustic'; personalizados: UUID v7
  name: string;                 // prontos em pt-BR: Plano, Graves, Agudos, Vocal, Rock, Pop,
                                // Eletrônica, Acústico; personalizados: 1 a 30 caracteres
  builtin: boolean;
  gainsDb: number[];            // 8 valores, −12 a +12, nas bandas EQ_BANDS_HZ
}
// const EQ_BANDS_HZ = [60, 170, 310, 600, 1000, 3000, 6000, 12000];

interface EqPresetInput { id?: string; name: string; gainsDb: number[]; }   // sem id = criar

// ---- Reprocessamento de metadados (RF-32)
interface MetadataRefreshStatus {
  running: boolean;
  stopping: boolean;           // parada solicitada; a faixa atual ainda termina
  cancelled: boolean;          // lote encerrado por pedido de parada
  done: number;                 // faixas processadas, com ou sem sucesso
  total: number;
  failed: number;
  currentTrackId: string | null;
  startedAt: number | null;     // null se nunca rodou nesta abertura
  finishedAt: number | null;
}

// ---- Cache (RF-36)
type CacheKind = 'artistPhotos' | 'artistData' | 'lyrics' | 'recommendations' | 'webview';
interface CacheUsage {
  totalBytes: number;
  items: { kind: CacheKind; bytes: number }[];   // sempre os 5 tipos, nesta ordem
}

// ---- Registros do sistema (RF-37)
type LogKind = 'playback' | 'skip' | 'download' | 'metadata' | 'artist' | 'lyrics'
             | 'recommendations' | 'loudness' | 'images' | 'ytdlp' | 'cache' | 'error';
type LogLevel = 'info' | 'warn' | 'error';
interface LogEntry {
  id: number;                   // crescente
  at: number;
  kind: LogKind;
  level: LogLevel;
  message: string;              // pt-BR, curta, legível
  trackId: string | null;
  details: string | null;       // texto livre (ex.: código e mensagem do erro)
}
interface LogQuery { kinds?: LogKind[]; level?: LogLevel; beforeId?: number; limit?: number; }
```

## Comandos

| Comando | Argumentos | Retorno | Requisitos |
|---|---|---|---|
| `player_get_state` | — | `PlayerState` | RF-01 |
| `player_play` | `{ trackId, queue?: string[], context?: QueueContext }` | `PlayerState` | RF-01, RF-03, RF-42 |
| `player_pause` | — | `PlayerState` | RF-01 |
| `player_resume` | — | `PlayerState` | RF-01 |
| `player_stop` | — | `PlayerState` | RF-01 |
| `player_seek` | `{ positionMs }` | `PlayerState` | RF-03 |
| `player_next` | — | `PlayerState` | RF-03 |
| `player_previous` | — | `PlayerState` | RF-03 |
| `player_set_volume` | `{ volume }` | `PlayerState` | RF-02 |
| `player_set_muted` | `{ muted }` | `PlayerState` | RF-02 |
| `player_get_queue` | — | `Track[]` | RF-03 |
| `player_set_shuffle` | `{ mode: ShuffleMode }` | `PlayerState` | RF-24 |
| `equalizer_list_presets` | — | `EqPreset[]` | RF-29 |
| `equalizer_save_preset` | `{ preset: EqPresetInput }` | `EqPreset` | RF-29 |
| `equalizer_delete_preset` | `{ presetId }` | `void` | RF-29 |
| `library_list_tracks` | `{ query?: TrackQuery }` | `Track[]` | RF-05, RF-08 |
| `library_get_track` | `{ trackId }` | `Track` | RF-05 |
| `library_list_tracks_by_artist` | `{ artistId, query?: TrackQuery }` | `Track[]` | RF-05, RF-08 |
| `library_set_favorite` | `{ trackId, favorite }` | `Track` | RF-04, RF-30 |
| `favorites_set_playlist` | `{ playlistId }` | `Playlist` | RF-30 |
| `library_delete_track` | `{ trackId, deleteFile }` | `void` | RF-06 |
| `playlist_list` | — | `Playlist[]` | RF-06 |
| `playlist_create` | `{ name, description? }` | `Playlist` | RF-06 |
| `playlist_update` | `{ playlistId, name?, description?, autoDownload? }` | `Playlist` | RF-06, RF-31 |
| `playlist_delete` | `{ playlistId }` | `void` | RF-06 |
| `playlist_get_tracks` | `{ playlistId, query?: TrackQuery }` | `Track[]` | RF-07, RF-08 |
| `playlist_add_tracks` | `{ playlistId, trackIds }` | `Playlist` | RF-06 |
| `playlist_remove_tracks` | `{ playlistId, trackIds }` | `Playlist` | RF-06 |
| `playlist_move_track` | `{ playlistId, trackId, newPosition }` | `void` | RF-07 |
| `playlist_set_cover` | `{ playlistId, image: ImageUpload \| null }` | `Playlist` | RF-22 |
| `youtube_search` | `{ query, limit? }` | `SearchResult[]` | RF-09 |
| `youtube_suggest` | `{ query }` | `string[]` | RF-23 |
| `youtube_search_history` | — | `string[]` | RF-34 |
| `youtube_search_history_clear` | — | `void` | RF-34 |
| `youtube_add_to_library` | `{ sourceIds, playlistId? }` | `Track[]` | RF-09 |
| `ytdlp_status` | — | `YtDlpStatus` | RF-26 |
| `ytdlp_update` | — | `YtDlpStatus` | RF-26 |
| `download_enqueue` | `{ trackIds }` | `DownloadJob[]` | RF-10 |
| `download_enqueue_playlist` | `{ playlistId }` | `DownloadJob[]` | RF-11 |
| `download_list` | — | `DownloadJob[]` | RF-10 |
| `download_cancel` | `{ jobId }` | `void` | RF-10 |
| `download_clear_finished` | — | `number` | RF-10 |
| `metadata_refresh_track` | `{ trackId }` | `Track` | RF-12 |
| `metadata_get_artist` | `{ artistId }` | `ArtistInfo` | RF-12 |
| `metadata_get_lyrics` | `{ trackId }` | `Lyrics` | RF-12 |
| `metadata_refresh_library` | `{ onlyMissing?: boolean }` | `number` | RF-12, RF-20, RF-32 |
| `metadata_refresh_status` | — | `MetadataRefreshStatus` | RF-32 |
| `metadata_refresh_cancel` | — | `void` | RF-32 |
| `stats_library` | — | `LibraryStats` | RF-14 |
| `stats_wrapped` | `{ year }` | `Wrapped` | RF-15 |
| `stats_wrapped_years` | — | `number[]` | RF-15 |
| `stats_taste` | — | `TasteProfile` | RF-19 |
| `stats_track_scores` | `{ trackIds?: string[] }` | `TrackScore[]` | RF-25 |
| `stats_dna` | `{ windowDays?: number }` | `Dna` | RF-25 |
| `recommendations_list` | `{ playlistId?, limit? }` | `Recommendation[]` | RF-19 |
| `recommendations_refresh` | `{ playlistId? }` | `Recommendation[]` | RF-19 |
| `recommendations_dismiss` | `{ sourceId }` | `void` | RF-19 |
| `profile_get` | — | `Profile` | RF-16 |
| `profile_update` | `{ profile: Profile }` | `Profile` | RF-16 |
| `profile_set_avatar` | `{ image: ImageUpload \| null }` | `Profile` | RF-21 |
| `settings_get` | — | `Settings` | RF-17 |
| `settings_update` | `{ settings: Partial<Settings> }` | `Settings` | RF-17, RNF-09 |
| `system_gpu_status` | — | `GpuStatus` | RNF-09 |
| `system_gpu_startup_confirmed` | — | `void` | RNF-09 |
| `cache_usage` | — | `CacheUsage` | RF-36 |
| `cache_clear` | — | `CacheUsage` | RF-36 |
| `system_logs` | `{ query?: LogQuery }` | `LogEntry[]` | RF-37 |
| `system_logs_clear` | — | `number` | RF-37 |

## Eventos (Rust → frontend)

| Evento | Payload | Quando |
|---|---|---|
| `player://state` | `PlayerState` | Mudança de status, faixa, volume ou fila. |
| `player://progress` | `{ positionMs, durationMs }` | A cada ~500 ms enquanto toca. |
| `download://progress` | `DownloadJob` | Mudança de status ou progresso de um download. |
| `library://changed` | `{ reason: string }` | Faixas ou playlists criadas, alteradas ou removidas. |
| `recommendations://changed` | `{ count: number }` | A lista de candidatas mudou (atualização, dispensa ou faixa adicionada). |
| `player://error` | `{ trackId: string, code: string, message: string }` | A faixa pedida não pôde tocar depois de `player_play` já ter voltado (ex.: falha ao resolver o streaming). |
| `metadata://progress` | `MetadataRefreshStatus` | Andamento de `metadata_refresh_library`: no início, ao pedir parada (`stopping = true`), a cada faixa e no fim (`running = false`; `cancelled = true` se parado). Os campos `done` e `total` continuam como antes. |
| `artist://changed` | `{ artistId: string }` | Os dados ou a foto de um artista foram atualizados em segundo plano (RF-35). |
| `download://auto` | `{ playlistId: string, count: number, reason: 'enabled' \| 'added' \| 'retry' \| 'offline' }` | Downloads automáticos de uma playlist foram enfileirados, ou ficaram esperando a rede (RF-31). |
| `log://entry` | `LogEntry` | Um registro novo foi gravado; só com `debugMode` ligado (RF-37). |
| `ytdlp://status` | `YtDlpStatus` | Mudança de estado ou progresso do download do yt-dlp. |

## Regras de comportamento

- `player_play` em faixa sem arquivo local (`filePath` nulo): se a origem é `youtube`, a casca
  toca por streaming (`PlayerState.streaming = true`). **`player_play` volta na hora** com status
  `loading` e a faixa nova, sem esperar o yt-dlp: a URL é resolvida em segundo plano, sem segurar
  a trava do player, e o status passa a `playing` quando o primeiro áudio sai. Se a resolução
  falha (sem rede, vídeo indisponível), a casca emite `player://error` com o código (`network`,
  `unavailable`) e `player://state` com status `stopped`. Se outro `player_play` chega antes de a
  resolução terminar, o resultado da anterior é descartado. Faixa `local` sem arquivo devolve
  `unavailable` na hora. Streaming não grava arquivo nem muda `downloadStatus`; o registro de
  atividade conta igual. `player_seek` funciona em streaming.
- `player_pause` durante a resolução guarda o pedido e mantém `status = loading` até a faixa estar
  carregada; quando pronta, emite `player://state` com `status = paused` e posição 0, sem iniciar o
  áudio. `player_resume` durante a resolução cancela esse pedido e mantém `loading`; a faixa toca
  quando estiver pronta. `player_resume` depois de pronta e pausada inicia a reprodução.
- `coverPath`, `mosaicPaths`, `imagePath` e `avatarPath` são caminhos locais. O frontend converte
  com `convertFileSrc`, então `src-tauri` habilita o protocolo `asset` com escopo no diretório de
  dados (`audio/`, `covers/`, `avatars/`, `playlists/`). Ver "Capas na interface".
- `stats_wrapped` agrupa por mês e dia no horário local. O fuso é calculado pela casca (relógio do
  sistema), não enviado pelo frontend.
- Conta como "play" o evento concluído ou com pelo menos 30 s ouvidos.
- "Tentar de novo" em um download com erro é um novo `download_enqueue` da mesma faixa.
- `player_get_queue` devolve as faixas de `PlayerState.queue`, na mesma ordem. Ids de faixas que já
  foram apagadas ficam de fora.
- `player_play` com `queue` substitui a fila e seu contexto pelo `context` enviado, ou por `null`
  quando omitido. Sem `queue`, mantém o contexto se a faixa está na fila atual; uma faixa de fora
  cria fila unitária sem contexto. `next`, `previous`, `seek` e
  `set_shuffle` o mantêm; `stop` ou fila esvaziada o limpam. Renomear a playlist atualiza o
  `label`; apagá-la limpa o contexto. A fila e seu contexto não são persistidos.
- `stats_wrapped_years` devolve, em ordem decrescente, os anos (horário local) com pelo menos um
  evento de reprodução. Vazio se nunca tocou nada.
- `download_clear_finished` remove os jobs `done` e `error` e devolve quantos removeu. Não apaga
  arquivo nem faixa.
- `library_list_tracks_by_artist` aceita o mesmo `TrackQuery` de `library_list_tracks`.

### Aleatório e Surfle (RF-24)

A troca de `smart` por `surfle` em `ShuffleMode` quebra a compatibilidade com clientes antigos.

- `player_set_shuffle` preserva as faixas antes da posição atual na fila em vigor, na mesma ordem,
  e mantém a ocorrência atual tocando na mesma posição. Só reordena as ocorrências depois dela:
  são exatamente as faixas ainda não tocadas nessa fila, sem entradas ou perdas, mesmo com IDs
  repetidos. A regra vale também após várias trocas seguidas de modo. `PlayerState.queue` sempre
  traz a ordem em que as faixas vão tocar.
- `random`: embaralhamento uniforme do restante da fila.
- `surfle`: sorteio ponderado, uma faixa por vez. Evita repetir imediatamente o artista quando
  existe alternativa, considera artistas das últimas três posições, proximidade de gênero com a
  faixa anterior, afinidade com os gêneros mais pontuados no perfil de gosto e pontuação da faixa.
  Toda faixa mantém peso positivo e entra na fila; faixas muito puladas tendem ao fim.
  Fórmula em [`recomendacao.md`](recomendacao.md).
- Voltar para `off` põe somente as ocorrências restantes na ordem original de `player_play`;
  o prefixo já tocado e a ocorrência atual continuam onde estão.
- Um `player_play { trackId, queue }` novo com `shuffle` ligado embaralha a fila nova da mesma
  forma, mantendo `trackId` primeiro. O modo fica salvo entre aberturas do app.

### Pontuação e DNA (RF-25)

- `stats_track_scores` sem `trackIds` devolve todas as faixas da discoteca; faixa nunca tocada
  tem `plays = 0` e `score` só com o bônus de favorita, se for.
- `stats_dna`: `windowDays` padrão 30, entre 7 e 365. A janela atual é `[agora − windowDays,
  agora]`; a anterior, os `windowDays` antes dela. `rising` e `falling` comparam o saldo de cada
  artista e gênero entre as duas janelas e ignoram variações menores que 0,1. Sem histórico, listas
  vazias e taxas zero (não é erro).
- Favoritar uma faixa conta no gosto mesmo que ela nunca tenha tocado (`stats_taste`, `stats_dna`
  e recomendações).

### Imagens enviadas (RF-21, RF-22)

- `profile_set_avatar` e `playlist_set_cover` recebem a imagem inteira em base64, até 10 MB. A
  casca decodifica (JPEG, PNG ou WebP; outro formato é `invalid_input`), recorta o centro em
  quadrado, reduz para 512×512 e grava em WebP com perda, qualidade 82 (RNF-12), em `avatars/`
  ou `playlists/`, com um nome novo a
  cada envio para não reaproveitar cache. O arquivo anterior é apagado. `image: null` remove a
  imagem (o avatar volta ao padrão; a playlist volta ao mosaico).
- `Playlist.mosaicPaths`: os 4 artistas com mais faixas na playlist (empate pelo nome); para cada
  um, a foto do artista (`ArtistInfo.imagePath`) ou, sem foto, a capa de uma faixa dele na
  playlist. Artista sem nenhuma das duas fica de fora. Com menos de 4, o frontend decide o
  arranjo (1 capa inteira, 2 ou 3 em grade).

### Aceleração de hardware (RNF-09)

- `Settings.gpuMode` (padrão `auto`) fica no banco. A casca lê o modo **antes de criar a janela**,
  então a troca só vale na próxima abertura; `GpuStatus.restartRequired` avisa a interface.
- `MIUPY_GPU=on|off|auto` continua funcionando e, quando definida, manda sobre o modo salvo
  (`envOverride`).
- Em Wayland + NVIDIA, `auto` usa GPU com WebKitGTK carregado anterior a 2.54 e software a partir
  de 2.54 ou quando a versão é desconhecida. Fora dessa combinação, a escolha automática não muda.
  `on` força a GPU. Quando a GPU é escolhida nessa combinação pelo modo salvo `on` ou por `auto`,
  a casca grava `gpu-startup-pending` no diretório de dados antes da janela. O frontend chama
  `system_gpu_startup_confirmed` cinco segundos após montar para apagar a sentinela; uma saída
  normal também a apaga. Se a próxima abertura encontrar a sentinela e voltaria a usar GPU, a
  casca abre em software, restaura `auto` no banco e informa o recuo em `GpuStatus.reason`.
  `MIUPY_GPU` explícita mantém precedência e não arma sentinela. Uma nova escolha na tela de
  Configurações descarta uma sentinela antiga, permitindo outra tentativa voluntária.

### Capas guardadas no disco (RF-05, RF-12)

- A capa do álbum vem do Cover Art Archive (pelo álbum identificado no MusicBrainz) e é gravada
  **uma vez por álbum** em `covers/albuns/<chave>.webp`, onde a chave é o MBID do *release group*
  ou, sem ele, um hash de artista + álbum normalizados. Todas as faixas do álbum apontam
  `coverPath` para esse arquivo. Sem capa de álbum, fica a miniatura do vídeo em
  `covers/<trackId>.webp`.
- A foto do artista fica em `covers/artistas/<artistId>.webp` (ver "Fotos de artista").
- Imagem já gravada não é baixada de novo; nada disso usa rede para exibir. Apagar uma faixa não
  apaga a capa do álbum enquanto outra faixa a usar.

### Fotos de artista (RF-27)

- A foto vem do Wikimedia Commons, pela propriedade "imagem" (P18) do item do Wikidata ligado ao
  MBID do artista no MusicBrainz. Sem MBID, sem item ou sem imagem: `imagePath` nulo.
- A casca busca a foto em segundo plano depois de um download ou de `metadata_get_artist`, grava
  em `covers/artistas/<artistId>.webp` e guarda `imageCredit` (autor e licença do arquivo no
  Commons), que a interface mostra junto da foto.
- Se o artista ainda não tiver MBID, `metadata_get_artist` tenta obtê-lo antes da foto, na mesma
  consulta. Falhas temporárias de rede ou tempo esgotado permitem nova tentativa após 10 s;
  ausência real de dados mantém o intervalo de 5 min.

### Ritmo das APIs de metadados (RF-12)

- MusicBrainz compartilha um limite de uma chamada por segundo em toda a instância da casca.
  LRCLIB tem fila própria, com intervalo mínimo de 2 s. Respostas 503/429 e falhas de transporte
  recebem até três tentativas, com espera crescente e respeito a `Retry-After`. Uma indicação de
  espera acima de 8 s encerra a consulta sem antecipar nova chamada; cada operação de API tem
  limite total de 15 s no provedor, e a casca corta antes as leituras de letra, artista e foto
  (12 s). O lote avança para a faixa seguinte quando uma consulta falha.
- As APIs de imagem seguem a mesma política pelo mesmo mecanismo: Wikidata, Wikimedia Commons
  (consulta e download da foto, que dividem a fila) e Cover Art Archive têm cada uma fila própria
  com intervalo mínimo de 1 s entre chamadas, até três tentativas em 503/429 e falha de
  transporte, espera crescente com `Retry-After`, espera acima de 8 s encerrando sem nova chamada
  e limite total de 15 s por chamada. Só o download de imagens de outras origens (miniaturas do
  YouTube) fica fora da fila.
- Letra e foto não encontradas (e erros definitivos, como MBID inválido) são consultadas de novo
  após 5 min. Só falhas temporárias — rede, 503/429 e tempo esgotado — permitem nova tentativa
  após 10 s; os dados locais são preservados.

### Busca no YouTube (RF-09, RF-23)

- `youtube_search` e `recommendations_refresh` descartam vídeos com mais de 10 minutos e títulos
  de coletânea ("full album", "álbum completo", "mix", "playlist", "coletânea", "greatest hits",
  "1 hour" e parecidos).
- `youtube_suggest` devolve até 10 sugestões de texto para a busca, do serviço de sugestões do
  YouTube. Consulta vazia devolve `[]`. Sem rede, `network`; a interface ignora o erro e não
  mostra sugestões.

### yt-dlp gerenciado pelo app (RF-26)

- O app usa o binário oficial do yt-dlp (release do GitHub `yt-dlp/yt-dlp`), baixado para
  `bin/` no diretório de dados e conferido contra o `SHA2-256SUMS` da mesma release. Não usa o
  yt-dlp do sistema. Exceção para desenvolvimento: a variável `MIUPY_YTDLP=<caminho>` força um
  binário.
- Na abertura, sem binário, a casca baixa em segundo plano (`ytdlp://status`). Com binário, verifica
  versão nova no máximo uma vez por dia, sem bloquear. `ytdlp_update` força a verificação.
- Enquanto o yt-dlp não está pronto, busca, download e streaming devolvem `unavailable` com uma
  mensagem dizendo que o yt-dlp está sendo baixado.

### Metadados em lote (RF-12, RF-20)

- `metadata_refresh_library` volta na hora com o número de faixas enfileiradas e roda em segundo
  plano, uma faixa por vez, respeitando o limite do MusicBrainz. Faz o mesmo que
  `metadata_refresh_track`. Sem gravação com o artista, ou sem artista salvo, a busca só por
  título exige pontuação mínima 95 e duração conhecida a até 10 s; dela usa apenas MBID, ISRC,
  gêneros e ano.
  Só corrige o artista quando o salvo é igual ao canal do vídeo e há uma única candidata
  nessa tolerância. A associação usa o artista por nome,
  sem renomear um registro compartilhado. O álbum só é preenchido quando está vazio, tanto
  com MusicBrainz quanto com YouTube; um álbum existente é preservado.
  A consulta de letra não segura o lote; continua disponível em
  `metadata_refresh_track` e na leitura individual de letra.
  `onlyMissing` (padrão `true`) pega só faixas sem gêneros, sem ano ou sem álbum. Emite
  `metadata://progress` e `library://changed` ao fim. Pode receber um pedido de parada por
  `metadata_refresh_cancel`, que preserva a requisição da faixa atual.
- Só pode haver um lote ativo. Enquanto ele roda, outro pedido devolve `conflict`.

### Recomendações (RF-19)

- A pontuação é calculada localmente, a partir de `play_events` e das favoritas (playlist marcada,
  RF-30): tempo ouvido e
  faixas concluídas somam, pulos subtraem, favorito soma, e o que foi ouvido há pouco pesa mais. A
  fórmula e os pesos ficam em [`recomendacao.md`](recomendacao.md).
- `recommendations_list` nunca usa rede: ordena as candidatas já guardadas no banco contra o gosto
  atual. Com `playlistId`, o gosto considerado é o das faixas daquela playlist. `limit` padrão 20,
  máximo 100. Sem histórico ou sem candidatas, devolve lista vazia (não é erro).
- `recommendations_refresh` usa rede: a partir das faixas, artistas e gêneros mais bem pontuados,
  busca candidatas no YouTube, guarda no banco e devolve o mesmo que `recommendations_list`. Sem
  rede, devolve `network` e as candidatas antigas continuam valendo.
- Uma candidata nunca é uma faixa que já está na discoteca nem uma que foi dispensada.
  `recommendations_dismiss` é permanente para aquele `sourceId`.
- Adicionar uma recomendação é `youtube_add_to_library { sourceIds, playlistId? }`; a casca tira a
  candidata da lista e emite `recommendations://changed`. Fora do cache de busca (app reaberto), os
  dados vêm da candidata gravada; só um `sourceId` que não está no cache, na discoteca nem entre as
  candidatas não dispensadas dá `not_found` ("Resultado da busca expirado").
- `Track.genres` e `Track.releaseYear` são preenchidos no download e em `metadata_refresh_track`.

### Capas na interface (RF-05)

- Toda imagem local passa por `api.resolveAsset` (`convertFileSrc`) e precisa carregar tanto no
  `npx tauri dev` (página em `http://localhost:1420`) quanto no app empacotado.

### Imagens compactas (RNF-12)

- Toda imagem gravada pela casca é WebP com perda, qualidade 82. Capa de álbum, miniatura de vídeo
  e foto de artista: no máximo 600 px no maior lado, sem aumentar imagem menor e sem recortar.
  Avatar e capa de playlist: 512×512, recorte central (ver "Imagens enviadas").
- Na abertura, uma tarefa em segundo plano converte as imagens antigas (`.jpg`, `.png`) em
  `covers/`, `avatars/` e `playlists/`, atualiza os caminhos no banco e só então apaga o original.
  É idempotente: um arquivo que falhar fica como está e é tentado na próxima abertura. Emite
  `library://changed` ao fim, se algo mudou.

### Normalização de volume (RF-28)

- Alvo por modo: `quiet` −19 LUFS, `normal` −14 LUFS, `loud` −11 LUFS; `off` não aplica ganho.
  Padrão `normal`. O ganho é por faixa: `alvo − loudnessLufs`, limitado a [−20, +12] dB.
- Em `quiet` e `normal`, o ganho positivo é cortado para o pico verdadeiro (*true peak*) da faixa
  não passar de −1 dBTP. Em `loud`, o ganho vale inteiro e um limitador no motor segura a saída em
  −1 dBTP. O limitador fica sempre no fim da cadeia (também protege o equalizador) e é de pico
  verdadeiro (com sobreamostragem), não de amostra.
- A medição (loudness integrado e pico verdadeiro, EBU R128; o pico fica no banco e não vai no
  `Track`) roda em `miupy-audio` sobre o arquivo
  local: logo depois do download e, para faixas antigas sem medida, numa fila de baixa prioridade
  em segundo plano, uma faixa por vez, que nunca segura o player. Faixa sem medida (inclusive por
  streaming) toca a 0 dB. A medida fica no banco; `Track.loudnessLufs` a expõe.
- Trocar o modo vale para a faixa atual na hora, sem cortar o áudio. O ganho da normalização é
  separado do volume do usuário (`PlayerState.volume` não muda).

### Equalizador (RF-29)

- 8 bandas em 60, 170, 310, 600, 1000, 3000, 6000 e 12000 Hz: *low shelf* em 60 Hz, *high shelf*
  em 12 kHz e 6 *peaking* entre elas, aplicadas em `miupy-audio` antes do limitador. O motor
  aplica um pré-ganho igual ao maior ganho positivo do perfil, com sinal trocado, para não clipar.
- Perfis prontos (dB, da banda mais grave à mais aguda), fixos e não editáveis:

  | id | Nome | Ganhos |
  |---|---|---|
  | `flat` | Plano | 0, 0, 0, 0, 0, 0, 0, 0 |
  | `bass` | Graves | +6, +5, +3, +1, 0, 0, 0, 0 |
  | `treble` | Agudos | 0, 0, 0, 0, +1, +3, +5, +6 |
  | `vocal` | Vocal | −2, −1, 0, +2, +4, +4, +2, 0 |
  | `rock` | Rock | +4, +3, −1, −2, −1, +2, +4, +5 |
  | `pop` | Pop | −1, +1, +3, +4, +3, 0, −1, −1 |
  | `electronic` | Eletrônica | +5, +4, +1, 0, −1, +2, +3, +4 |
  | `acoustic` | Acústico | +3, +2, +1, +1, +2, +2, +3, +2 |

- `equalizer_list_presets` devolve os 8 prontos, nessa ordem, e depois os personalizados, por data
  de criação. Até 3 personalizados: criar o quarto devolve `conflict`. Nome vazio ou acima de 30
  caracteres, `gainsDb` sem exatamente 8 valores ou fora de [−12, +12], ou `id` de perfil pronto:
  `invalid_input`. Nome repetido (sem diferenciar caixa) é `conflict`. `id` de personalizado que
  não existe, em salvar ou apagar: `not_found`.
- `settings_update { eqPresetId }` com id inexistente devolve `not_found`. A troca vale na hora,
  sem cortar o áudio. Salvar o perfil ativo aplica os novos ganhos na hora. Apagar o perfil ativo
  volta para `flat`.

### Favoritas em playlist (RF-04, RF-30)

- A playlist marcada (`Settings.favoritesPlaylistId`) é a **fonte única** das favoritas.
  `Track.isFavorite` é verdadeiro quando a faixa está nela; `TrackQuery.favoritesOnly` filtra por
  ela; `LibraryStats.favoriteCount` conta as faixas dela.
- `library_set_favorite { favorite: true }` adiciona a faixa ao fim dessa playlist; `false` a
  remove. Sem playlist marcada (nunca houve, ou a marcada foi apagada), o primeiro
  `favorite: true` cria uma playlist nova "Favoritas" e a marca.
- A migração cria "Favoritas" com as favoritas atuais (ordem de `favorites.created_at`) e a marca.
  Se já existir uma playlist com esse nome (sem diferenciar caixa), a migração usa essa, somando as
  favoritas ao fim sem repetir. A tabela `favorites` vira uma *view* sobre as faixas da playlist
  marcada (`track_id`, `created_at = added_at`), para a pontuação, o gosto e o DNA continuarem
  contando favoritas como antes.
- `favorites_set_playlist { playlistId }` marca outra playlist: as faixas da marcada antes são
  copiadas para o fim da nova, sem repetir as que ela já tem; a antiga continua como playlist comum.
  Marcar a mesma playlist não faz nada; `playlistId` inexistente é `not_found`. Apagar a playlist
  marcada deixa `favoritesPlaylistId` nulo (a interface avisa antes de apagar).
- Favoritar um resultado da busca no YouTube que não está na discoteca é
  `youtube_add_to_library { sourceIds: [id] }` seguido de `library_set_favorite`. `youtube_search`
  preenche `trackId` e `isFavorite` dos resultados que já estão na discoteca.
- Emite `library://changed` com `reason: 'favorites'`.

### Baixar playlist automaticamente (RF-31)

- `playlist_update { autoDownload: true }` enfileira na hora todas as faixas da playlist ainda sem
  arquivo (`downloadStatus` `none` ou `error`) e, se houver alguma, emite `download://auto` com `reason: 'enabled'`.
  Desligar não cancela o que já está na fila.
- Com a opção ligada, toda faixa nova que entra na playlist (`playlist_add_tracks`,
  `youtube_add_to_library` com `playlistId`, favoritar na playlist de favoritas) é enfileirada, com
  um `download://auto` por operação (`reason: 'added'`).
- Download automático que falha por rede fica esperando, sem novo toast para cada faixa: um único
  `download://auto` com `reason: 'offline'`. A casca tenta de novo a cada 5 minutos enquanto houver
  pendentes e na próxima abertura (`reason: 'retry'` apenas nessa abertura). Os ciclos do timer
  são silenciosos enquanto a rede continuar indisponível. O job aguardando rede usa
  `status: 'error'` e `error` começando por `auto_offline:`; a interface mostra “Aguardando rede”
  e permite cancelar. Ao perder o vínculo com todas as playlists automáticas, esse job passa a
  erro comum e pode ser removido por “Limpar concluídos”. Outros erros ficam como `error`, como
  nos downloads manuais.

### Reprocessamento de metadados (RF-32)

- `metadata_refresh_cancel` pede a parada sem argumentos e não falha sem lote ativo. A requisição
  da faixa em andamento termina normalmente; o lote encerra antes da próxima. Enquanto isso,
  `running = true` e `stopping = true`; no evento final, `running = false`, `stopping = false` e
  `cancelled = true`. Um lote novo limpa os dois estados de parada.
- Um aviso fixo acima do player acompanha o lote em qualquer tela. Fechá-lo esconde apenas o aviso
  deste lote; a seção de Configurações continua mostrando o andamento e o botão Parar.
- `metadata_refresh_status` devolve o lote atual ou o último desta abertura. Na Configurações, a
  interface lê esse estado ao abrir a seção e acompanha `metadata://progress`; com `running`, o
  botão de iniciar fica desativado, o botão Parar fica disponível até `stopping = true`, e o
  andamento mostra `done` de `total`. Ao fim, mostra o resumo de conclusão ou “Parado em X de Y”.
  O `conflict` de um segundo pedido continua valendo.

### Busca vazia (RF-34)

- Com a caixa de busca vazia, a tela do YouTube mostra até 10 buscas recentes e as recomendações de
  `recommendations_list` (sem rede), com os mesmos botões da Home (adicionar, dispensar).
- `youtube_search` grava a consulta (sem espaços nas pontas, não vazia) no histórico. Repetir uma
  consulta a sobe para o topo, sem duplicar (sem diferenciar caixa). Guarda as 50 mais recentes;
  `youtube_search_history` devolve as 10 mais recentes. `youtube_suggest` não grava.

### Dados de artista em segundo plano (RF-35)

- `ArtistInfo.fetchedAt` é a data da última busca **concluída**, com ou sem dados (artista sem
  MBID, sem foto ou não encontrado também conta). Falha temporária (rede, 503/429, tempo esgotado)
  não muda `fetchedAt` e segue os intervalos de "Ritmo das APIs de metadados" (10 s; 5 min para
  erro definitivo), sem nova tentativa antes disso.
- `metadata_get_artist` sempre volta na hora com o que está no banco (nunca espera rede). Se
  `artistAutoRefresh` está ligado e `fetchedAt` é nulo ou passa de 30 dias, a casca agenda a busca
  em segundo plano e emite `artist://changed` quando algo muda.
- Com `artistAutoRefresh` desligado, nada de artista é buscado sozinho, nem depois de download;
  `metadata_refresh_track` e o lote continuam funcionando quando o usuário pede.

### Cache (RF-36)

- `cache_usage` soma: `artistPhotos` (`covers/artistas/`), `artistData` (país, gêneros,
  biografia e crédito guardados no banco, estimado pelo tamanho do texto), `lyrics` (letras
  guardadas), `recommendations` (candidatas guardadas) e `webview` (cache HTTP do WebView, que
  guarda as miniaturas da busca e das recomendações).
- `cache_clear` apaga tudo isso e devolve o uso novo. Não apaga áudio, capas de álbum, miniaturas
  de faixas da discoteca, avatar, capas de playlist, histórico de reprodução, recomendações
  dispensadas nem o `localStorage` do WebView. Depois de limpar, os artistas ficam com
  `fetchedAt = null` e são buscados de novo pela regra do RF-35.

### Registros do sistema (RF-37)

- Só grava com `debugMode` ligado. A tabela guarda até 30 dias e no máximo 5.000 registros; a
  limpeza roda na abertura e a cada 500 gravações.
- Registra pelo menos: faixa iniciada e encerrada (`playback`, com tempo ouvido), pulo (`skip`),
  download enfileirado, concluído e com erro (`download`), início, faixa com falha e fim do lote de
  metadados (`metadata`), busca de dados e foto de artista (`artist`), letra (`lyrics`),
  atualização de recomendações (`recommendations`), medição de loudness (`loudness`), conversão de
  imagens (`images`), estado do yt-dlp (`ytdlp`), limpeza do cache (`cache`) e todo erro devolvido
  ao frontend com código `internal` ou `io` (`error`).
- `system_logs` devolve do mais novo para o mais antigo; `limit` padrão 200, máximo 1000;
  `beforeId` pagina. `system_logs_clear` apaga tudo e devolve quantos apagou.
- A tela "Registros do sistema" só aparece no menu com `debugMode` ligado; desligar o modo não
  apaga os registros.

### Menu lateral (RF-33)

- `Settings.sidebarWidth` fora de [220, 400] é cortado para o limite mais próximo (não é erro). A
  interface salva a largura ao soltar a borda, não durante o arraste.

### Carregamento sem espera (RNF-11)

- Regra do frontend, sem comando novo: cada store guarda a última resposta em memória durante a
  sessão. Uma tela com cache mostra o cache na hora e pede os dados de novo em segundo plano; sem
  cache, mostra *skeleton* no formato do conteúdo. Eventos (`library://changed`,
  `recommendations://changed`, `artist://changed`, `download://progress`) invalidam ou atualizam o
  cache. Spinner só dentro de botão com ação em andamento.
- Implementação: `lib/stores/resource.svelte.ts` (`Resource`: cache por chave de consulta,
  `invalidate(tags)`, no máximo uma consulta a cada 500 ms por recurso) e `lib/components/Skeleton*.svelte`.
  Uma resposta igual à anterior mantém as mesmas referências (`lib/util/diff.ts`), então nada é repintado.
- `library://changed` é filtrado pelo `reason` (`lib/util/libraryEvents.ts`). Cada motivo atualiza só os
  grupos de tela que dependem dele; motivo desconhecido ou ausente atualiza todos (o seguro). Motivo
  novo no backend: acrescente a linha na tabela do arquivo e aqui.

  | `reason` | Grupos atualizados |
  |---|---|
  | `favorite`, `favorites` | faixas, playlists, estatísticas (e a pontuação) |
  | `download_queued`, `download_failed`, `download_cancelled` | faixas |
  | `download_done` | faixas, estatísticas |
  | `playlist_created`, `playlist_deleted` | playlists, estatísticas |
  | `playlist_updated`, `playlist_cover_updated` | playlists |
  | `playlist_tracks_added`, `playlist_tracks_removed` | playlists, faixas, estatísticas |
  | `playlist_reordered` | playlists, faixas |
  | `artist_photo_updated` | artista, playlists |
  | `artist_updated` | artista, playlists, faixas |
  | qualquer outro (`tracks_added`, `track_deleted`, `images_migrated`, `metadata_*`, ...) | todos |

## Lacunas conhecidas

Ainda sem comando próprio: tocar uma recomendação ou um resultado de busca antes de adicionar à
discoteca (hoje é preciso adicionar primeiro).

O registro de atividade (RF-13) não tem comando próprio: o backend grava os eventos de reprodução
sozinho, a partir do estado do player.

## Áudio no Android (RF-44)

O contrato frontend ⇄ Rust do player permanece igual. Na casca Android, `AudioBackend`
encaminha comandos ao plugin `miupy-audio` por `run_mobile_plugin`; o plugin Kotlin controla
um ExoPlayer mantido por `MediaSessionService`. Um `Channel` Tauri devolve `state`, `progress`
(a cada 500 ms enquanto toca), `ended` (somente fim natural) e `error`. A casca traduz essas
mensagens para `AudioEvent`; `state()` consulta um espelho em memória, sem bloquear a Activity.
Cada `load`, `loadUrl` e `stop` leva um número de geração crescente, e o plugin carimba os
eventos com ele: a casca descarta o que vier de outra geração (um `progress` ou `ended` atrasado
da faixa anterior não vira `TrackEnded` da nova). O `load` deixa o ExoPlayer com
`playWhenReady = false`, como no desktop (pausado na posição 0).

`load` recebe o caminho absoluto do arquivo privado e o serviço monta um URI `file://`.
`loadUrl` recebe URL e cabeçalhos HTTP do `StreamOptions`; a duração conhecida fica no espelho
até o ExoPlayer apurá-la. `volume` aplica a curva perceptual `volume²` e a normalização somente
atenuante: `volume² × min(1, 10^(ganho_dB/20))`; mudo produz zero. Os oito ganhos de EQ ficam
guardados no backend nesta etapa; a aplicação pelo efeito nativo fica para a segunda metade.
A sessão Media3 fica registrada no serviço (`addSession`), que publica a notificação em primeiro plano. Um `ForwardingPlayer` anuncia próxima e anterior: na notificação, na tela de bloqueio e no fone, elas viram eventos `next`/`previous` com a geração atual, e a casca avança a fila com `Player::next`/`previous`. Tocar no cartão abre o app. Título, artista e capa vêm das tags do arquivo, lidas pelo
ExoPlayer; a capa enviada pela casca e o equalizador nativo entram na segunda metade.
