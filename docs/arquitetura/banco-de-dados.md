# Banco de dados

SQLite local (RNF-01), via `rusqlite` (`bundled`, que já traz FTS5) e `rusqlite_migration`.
Código em [`crates/miupy-core`](../../crates/miupy-core/README.md); o esquema vive em
`crates/miupy-core/migrations/0001_init.sql`, `0002_recommendations.sql` e
`0003_dna_shuffle_images.sql`. A versão do esquema fica em `PRAGMA user_version`. As migrações
antigas não foram alteradas; 0003 acrescenta colunas à versão 2 sem reescrever as tabelas.

## Convenções

- IDs: UUID v7 em texto (ordenáveis por tempo de criação). Datas: epoch em ms (UTC). Durações em ms.
- Todas as tabelas são `STRICT`. Booleanos são `INTEGER` 0/1 com `CHECK`. Enums são `TEXT` com `CHECK`.
- Na abertura: `foreign_keys = ON`, `journal_mode = WAL`, `synchronous = NORMAL`,
  `busy_timeout = 5 s`, e a collation `miupy_nocase` (ordem sem caixa e sem acento).
- Operações compostas rodam em `SAVEPOINT` (aninhável): ou tudo, ou nada.

## Diagrama

```
artists 1─* tracks 1─* playlist_tracks *─1 playlists
                 │ 1─1 favorites
                 │ 1─1 lyrics
                 │ 1─1 download_jobs
                 └ 1─* play_events *─0..1 playlists (contexto)
user_actions (log solto, sem FK)        profile, settings (linha única)
tracks_fts (FTS5)  ←  triggers em tracks e artists
recommendation_candidates (source_id único; sem FK para a faixa candidata)
```

## Tabelas

### `artists`
`id`, `name`, `name_key` (UNIQUE), `mbid`, `country`, `genres` (array JSON), `bio`, `image_path`,
`image_credit` (desde 0003; autor e licença da foto), `created_at`. A identidade é `name_key`
(aparado, espaços colapsados, minúsculas, sem acento):
"Sigur Rós" e "sigur  ros" são o mesmo artista. O nome exibido é o da primeira ocorrência.
Artistas que ficam sem faixas **não são apagados** (preserva o enriquecimento do MusicBrainz); só
deixam de aparecer em listagens e contagens.

### `tracks`
`id`, `title`, `artist_id` (FK `ON DELETE SET NULL`), `album`, `duration_ms`, `file_path`
(anulável: vazio até o download), `cover_path`, `source` (`youtube`|`local`), `source_id`, `codec`,
`bitrate`, `thumbnail_url`, `download_status` (`none|queued|downloading|done|error`), `added_at`.
Desde a migração 0002 também: `genres` (array JSON normalizado para minúsculas, sem repetição,
padrão `[]`), `release_year` (1..9999 ou NULL), `mbid` e `isrc` opcionais. A normalização ocorre
na API de criação/edição; o banco valida que `genres` é um array JSON.

- `UNIQUE (source, source_id) WHERE source_id IS NOT NULL`: o mesmo vídeo do YouTube entra uma vez.
- `UNIQUE (file_path) WHERE file_path IS NOT NULL`: dois registros não apontam para o mesmo arquivo.
- Índices: `artist_id`, `added_at`, `download_status`.
- `thumbnail_url` não está no tipo `Track` do ipc (é só para o downloader baixar a capa);
  leia com `Database::track_thumbnail_url`.
- Faixa com `file_path` na criação (local) nasce `done`; do YouTube nasce `none`.

### `favorites`
`track_id` (PK, FK cascade), `created_at`. Favorito é presença da linha; `Track.isFavorite` vem de
um `LEFT JOIN`.

### `playlists` e `playlist_tracks`
`playlists`: `id`, `name`, `description`, `cover_path`, `created_at`, `updated_at`.
`playlist_tracks`: PK `(playlist_id, track_id)` (uma faixa aparece uma vez por playlist),
`position` (0-based), `added_at`; ambas as FKs com `ON DELETE CASCADE`.

- `position` é **contígua** (`0..n-1`) depois de qualquer operação (adicionar, remover, mover,
  apagar a faixa da biblioteca). Não há `UNIQUE (playlist_id, position)` de propósito: o SQLite
  confere unicidade linha a linha e um `UPDATE` em massa de reordenação falharia no meio. A
  contiguidade é mantida em código (`move_playlist_track` reescreve a ordem; `renumber_playlist`
  recompacta com `ROW_NUMBER()`).
- Índice `(playlist_id, position)` e `(track_id)`.
- `trackCount` e `totalDurationMs` são calculados na consulta, não guardados.

### `lyrics`
`track_id` (PK, FK cascade), `plain`, `synced` (LRC), `updated_at`. Sem linha = sem letra.

### `download_jobs`
`id`, `track_id` (UNIQUE, FK cascade), `status` (`queued|downloading|done|error`), `progress`
(0..1), `error`, `created_at`, `updated_at`. **Um job por faixa**: tentar de novo reaproveita o
job (volta a `queued`). `tracks.download_status` e `download_jobs.status` mudam juntos, na mesma
transação. Cancelar apaga o job e devolve a faixa a `none`; quem baixa descobre o cancelamento
quando `update_download` devolve `not_found`. O sucesso passa por `mark_downloaded`, que grava
`file_path`/`codec`/`bitrate`/capa e fecha o job.

### `play_events` (RF-13)
`id`, `track_id` (FK cascade), `playlist_id` (FK `SET NULL`, contexto opcional), `started_at`,
`ended_at` (NULL = aberto), `listened_ms` (tempo realmente ouvido, acumulado), `completed`,
`skipped`. Índices em `started_at` e `(track_id, started_at)`.

### `recommendation_candidates` (RF-19)

`source_id` (PK), `title`, `artist`, `duration_ms`, `thumbnail_url`, `genres` (array JSON),
`seed_kind` (`similarTrack|artist|genre`), `seed_label`, `seed_track_id` (FK opcional, `SET NULL`),
`fetched_at` e `dismissed` (0/1). A candidata não está em `tracks`: o upsert recusa `source_id`
já presente. Um trigger remove candidatas ativas quando uma faixa com o mesmo `source_id` entra
em `tracks`. A linha dispensada permanece como lápide para impedir reaparição futura. O índice
`(dismissed, fetched_at)` auxilia a lista. Fórmula e seleção em
[`recomendacao.md`](recomendacao.md).

### `user_actions`
Log genérico: `id`, `kind` (ex.: `track_added`, `track_removed`, `track_favorited`,
`track_unfavorited`, `playlist_created`, `playlist_deleted`, `playlist_tracks_added`,
`playlist_tracks_removed`), `entity_type`, `entity_id`, `payload` (JSON), `created_at`. Sem FK:
o log sobrevive à remoção da entidade. As operações do núcleo registram as suas; ações novas usam
`Database::log_action`.

### `profile` e `settings`
Linha única cada (`id INTEGER PRIMARY KEY CHECK (id = 1)`), criada pela migração.
`profile`: `display_name`, `username`, `email`, `phone` (textos, vazios por padrão),
`avatar_path`. `settings`: `lava_theme` (`orange` por padrão), `volume` (0..1, padrão 0.8) e,
desde 0003, `shuffle_mode` (`off|random|smart`, padrão `off`) e `gpu_mode` (`auto|on|off`,
padrão `auto`). O modo aleatório é lido à parte do tipo `Settings` do IPC; o de GPU vira
`Settings.gpuMode`.
Validação em código: e-mail e telefone podem ser vazios; preenchidos, passam por conferência
tolerante (e-mail: uma `@`, domínio com ponto, sem espaços; telefone: 8 a 15 dígitos, aceitando
`+ - . ( )` e espaços; usuário: até 32 letras/dígitos/`_.-`).

### `tracks_fts` (busca)
Tabela FTS5 própria (`title`, `artist`, `album`), `rowid = tracks.rowid`, tokenizer
`unicode61 remove_diacritics 2` (ignora caixa **e** acento sem pré-processar nada) e índice de
prefixo (2 e 3). Triggers mantêm o índice: `AFTER INSERT/DELETE/UPDATE OF title, artist_id, album`
em `tracks` e `AFTER UPDATE OF name` em `artists` (renomear o artista reindexa as faixas).
A consulta do usuário é quebrada em palavras alfanuméricas, cada uma vira prefixo entre aspas e
todas precisam casar (`"nao"* "va"*`): entrada com aspas, `*`, `OR` ou parênteses não quebra a
sintaxe. Entrada só com pontuação devolve lista vazia; entrada em branco não filtra.

## Regra de "play"

Um evento de reprodução conta como **play** se foi **concluído** (`completed = 1`) **ou** teve
**pelo menos 30 s** (`PLAY_THRESHOLD_MS`) de tempo realmente ouvido.

| Campo | Considera |
|---|---|
| `totalPlays`, `plays` (rankings), `distinctTracks`, `distinctArtists` | só plays |
| `totalListenedMs`, `listenedMs`, `listenedMsByMonth`, `busiestDay` | todo o tempo ouvido, inclusive de eventos que não chegaram a ser play |

Rankings (`TOP_LIMIT` = 10): por plays desc, depois tempo ouvido desc, depois título/nome. Faixas
e artistas sem nenhum play ficam fora. `artistCount` conta artistas com ao menos uma faixa.

## Fuso horário

Mês e dia vêm do horário **local**, mas o núcleo nunca lê o relógio nem o fuso do sistema:
`wrapped(year, tz_offset_minutes)` recebe o deslocamento em minutos a leste de UTC (BRT = -180; no
JS, `-new Date().getTimezoneOffset()`). Um evento pertence ao dia/mês/ano do seu **início**
(`started_at`); um evento que cruza a meia-noite não é partido. `tracksAdded` usa `added_at` no
mesmo critério. Limitação: um único deslocamento vale para o ano todo (horário de verão no meio
do ano não é modelado).

## Migrações reservadas para a v0.3

Várias frentes criam migrações ao mesmo tempo. Para não haver duas `0004`, cada frente usa o
número reservado abaixo, mesmo que outra entre antes. Se uma frente for integrada antes de uma de
número menor, a orquestração renumera antes do merge: a lista em `db.rs` é aplicada na ordem e
uma migração já aplicada no banco do autor não pode mudar de posição.

| Número | Frente | Conteúdo |
|---|---|---|
| `0004_loudness.sql` | `feat/normalizacao-volume` | `tracks.loudness_lufs`, `tracks.true_peak_db`; `settings.normalization` |
| `0005_equalizer.sql` | `feat/equalizador` | `eq_presets` (personalizados); `settings.eq_preset_id` |
| `0006_sidebar_width.sql` | `feat/visual-flat` | `settings.sidebar_width` |
| `0007_favorites_playlist.sql` | `feat/favoritas-playlist` | `settings.favorites_playlist_id`; cria "Favoritas" a partir de `favorites` |
| `0008_search_history.sql` | `feat/busca-vazia` | `search_history` |
| `0009_auto_download.sql` | `feat/playlist-baixar-automatico` | `playlists.auto_download` |
| `0010_artist_refresh.sql` | `feat/skeleton-cache` | `artists.fetched_at`; `settings.artist_auto_refresh` |
| `0011_system_logs.sql` | `feat/registros-sistema` | `system_logs`; `settings.debug_mode` |

`feat/imagens-comprimidas` não cria migração: a troca de `.jpg` para `.webp` é uma tarefa da
casca na abertura, que atualiza os caminhos já existentes.

## Decisões

- **Apagar uma faixa apaga o histórico dela** (`play_events` em cascata): o ranking precisa da
  faixa para existir, e o usuário escolheu removê-la. Consequência: o tempo total ouvido diminui.
  Se isso incomodar, a alternativa é `SET NULL` + snapshot de artista/título no evento.
- **Sem `UNIQUE` em posição** de playlist (ver acima).
- **Um job de download por faixa** (`UNIQUE track_id`): histórico de tentativas não é guardado.
- **Perfil e configurações como linha tipada única**, não chave-valor: o `CHECK` vale no banco; uma
  configuração nova exige migração (aceitável neste porte).
- **`name_key` do artista dobra acentos**: "Beyoncé" e "Beyonce" (resultados diferentes do
  YouTube) caem no mesmo artista.
- **Faixas locais** (`source = 'local'`) são deduplicadas por `file_path`; as do YouTube por
  `(source, source_id)`. Reinserir uma faixa existente só preenche lacunas (miniatura, duração
  zerada) e nunca sobrescreve título/artista editados.
- **Ordenação** por título/artista/álbum usa a collation `miupy_nocase` (dobra caixa e as letras
  latinas acentuadas comuns); ordenação por locale completo (ICU) não é usada.
