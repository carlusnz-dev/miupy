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
type ShuffleMode = 'off' | 'random' | 'smart';   // RF-24

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
}

interface Playlist {
  id: string;
  name: string;
  description: string | null;
  coverPath: string | null;     // capa escolhida pelo usuário; null usa o mosaico (RF-22)
  mosaicPaths: string[];        // 0 a 4 caminhos locais, um por artista, para o mosaico padrão (RF-22)
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

interface Settings {
  lavaTheme: LavaTheme;
  volume: number;
  gpuMode: GpuMode;             // vale a partir da próxima abertura do app (RNF-09)
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
| `player_get_queue` | — | `Track[]` | RF-03 |
| `player_set_shuffle` | `{ mode: ShuffleMode }` | `PlayerState` | RF-24 |
| `library_list_tracks` | `{ query?: TrackQuery }` | `Track[]` | RF-05, RF-08 |
| `library_get_track` | `{ trackId }` | `Track` | RF-05 |
| `library_list_tracks_by_artist` | `{ artistId, query?: TrackQuery }` | `Track[]` | RF-05, RF-08 |
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
| `playlist_set_cover` | `{ playlistId, image: ImageUpload \| null }` | `Playlist` | RF-22 |
| `youtube_search` | `{ query, limit? }` | `SearchResult[]` | RF-09 |
| `youtube_suggest` | `{ query }` | `string[]` | RF-23 |
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
| `metadata_refresh_library` | `{ onlyMissing?: boolean }` | `number` | RF-12, RF-20 |
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

## Eventos (Rust → frontend)

| Evento | Payload | Quando |
|---|---|---|
| `player://state` | `PlayerState` | Mudança de status, faixa, volume ou fila. |
| `player://progress` | `{ positionMs, durationMs }` | A cada ~500 ms enquanto toca. |
| `download://progress` | `DownloadJob` | Mudança de status ou progresso de um download. |
| `library://changed` | `{ reason: string }` | Faixas ou playlists criadas, alteradas ou removidas. |
| `recommendations://changed` | `{ count: number }` | A lista de candidatas mudou (atualização, dispensa ou faixa adicionada). |
| `player://error` | `{ trackId: string, code: string, message: string }` | A faixa pedida não pôde tocar depois de `player_play` já ter voltado (ex.: falha ao resolver o streaming). |
| `metadata://progress` | `{ done: number, total: number }` | Andamento de `metadata_refresh_library`; `done == total` no fim. |
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
  dados (`audio/`, `covers/`, `avatars/`, `playlists/`).
- `stats_wrapped` agrupa por mês e dia no horário local. O fuso é calculado pela casca (relógio do
  sistema), não enviado pelo frontend.
- Conta como "play" o evento concluído ou com pelo menos 30 s ouvidos.
- "Tentar de novo" em um download com erro é um novo `download_enqueue` da mesma faixa.
- `player_get_queue` devolve as faixas de `PlayerState.queue`, na mesma ordem. Ids de faixas que já
  foram apagadas ficam de fora.
- `stats_wrapped_years` devolve, em ordem decrescente, os anos (horário local) com pelo menos um
  evento de reprodução. Vazio se nunca tocou nada.
- `download_clear_finished` remove os jobs `done` e `error` e devolve quantos removeu. Não apaga
  arquivo nem faixa.
- `library_list_tracks_by_artist` aceita o mesmo `TrackQuery` de `library_list_tracks`.

### Aleatório e *smart random* (RF-24)

- `player_set_shuffle` muda só a ordem do que vem **depois** da faixa atual; a faixa atual continua
  tocando. `PlayerState.queue` sempre traz a ordem em que as faixas vão tocar.
- `random`: embaralhamento uniforme do restante da fila.
- `smart`: sorteio ponderado, uma faixa por vez. O peso de cada faixa restante combina a pontuação
  dela (`TrackScore.score`, normalizada) com a proximidade da faixa escolhida antes (mesmo artista
  pesa mais que gênero em comum). Toda faixa tem peso mínimo positivo, então todas tocam; faixas
  puladas muitas vezes só ficam mais para o fim. Fórmula em [`recomendacao.md`](recomendacao.md).
- Voltar para `off` restaura a ordem original do restante, a partir da faixa atual.
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
  quadrado, reduz para 512×512 e grava em JPEG em `avatars/` ou `playlists/`, com um nome novo a
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
- `auto` mantém o recuo para software onde a GPU derruba o app (Wayland + NVIDIA). `on` força a
  GPU mesmo assim. Quando `on` veio das configurações nessa combinação, a casca grava
  `gpu-startup-pending` no diretório de dados antes da janela. O frontend chama
  `system_gpu_startup_confirmed` cinco segundos após montar para apagar a sentinela; uma saída
  normal também a apaga. Se a próxima abertura encontrar a sentinela com modo salvo `on`, a casca
  abre em software, restaura `auto` no banco e informa o recuo em `GpuStatus.reason`.
  `MIUPY_GPU` explícita mantém precedência e não arma sentinela. Uma nova escolha na tela de
  Configurações descarta uma sentinela antiga, permitindo outra tentativa voluntária.

### Capas guardadas no disco (RF-05, RF-12)

- A capa do álbum vem do Cover Art Archive (pelo álbum identificado no MusicBrainz) e é gravada
  **uma vez por álbum** em `covers/albuns/<chave>.jpg`, onde a chave é o MBID do *release group*
  ou, sem ele, um hash de artista + álbum normalizados. Todas as faixas do álbum apontam
  `coverPath` para esse arquivo. Sem capa de álbum, fica a miniatura do vídeo em
  `covers/<trackId>.<ext>`, como hoje.
- A foto do artista fica em `covers/artistas/<artistId>.jpg` (ver "Fotos de artista").
- Imagem já gravada não é baixada de novo; nada disso usa rede para exibir. Apagar uma faixa não
  apaga a capa do álbum enquanto outra faixa a usar.

### Fotos de artista (RF-27)

- A foto vem do Wikimedia Commons, pela propriedade "imagem" (P18) do item do Wikidata ligado ao
  MBID do artista no MusicBrainz. Sem MBID, sem item ou sem imagem: `imagePath` nulo.
- A casca busca a foto em segundo plano depois de um download ou de `metadata_get_artist`, grava
  em `covers/artistas/<artistId>.jpg` e guarda `imageCredit` (autor e licença do arquivo no
  Commons), que a interface mostra junto da foto.

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
  `metadata_refresh_track`, inclusive refazer título, artista e álbum a partir dos dados do
  YouTube. `onlyMissing` (padrão `true`) pega só faixas sem gêneros, sem ano ou sem álbum. Emite
  `metadata://progress` e `library://changed` ao fim.
- Só pode haver um lote ativo. Enquanto ele roda, outro pedido devolve `conflict`.

### Recomendações (RF-19)

- A pontuação é calculada localmente, a partir de `play_events` e `favorites`: tempo ouvido e
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
  candidata da lista e emite `recommendations://changed`.
- `Track.genres` e `Track.releaseYear` são preenchidos no download e em `metadata_refresh_track`.

## Lacunas conhecidas

Ainda sem comando próprio: tocar uma recomendação ou um resultado de busca antes de adicionar à
discoteca (hoje é preciso adicionar primeiro).

O registro de atividade (RF-13) não tem comando próprio: o backend grava os eventos de reprodução
sozinho, a partir do estado do player.
