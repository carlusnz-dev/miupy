# miupy-core

Domínio e persistência do miupy: modelos, banco SQLite com migrações, repositórios, busca,
estatísticas e retrospectiva anual. **Não depende de Tauri nem das outras crates** do workspace;
a camada `src-tauri` converte e expõe isto como os comandos de
[`docs/arquitetura/ipc.md`](../../docs/arquitetura/ipc.md). Esquema completo em
[`docs/arquitetura/banco-de-dados.md`](../../docs/arquitetura/banco-de-dados.md).

## Uso

```rust
let db = miupy_core::Database::open(app_data_dir.join("miupy.db"))?; // cria, migra
let t = db.upsert_track(&NewTrack::youtube("dQw4w9WgXcQ", "Titulo", Some("Artista".into()), 213_000, None))?;
db.set_favorite(&t.track.id, true)?;
```

A API é **síncrona**. `Database` é `Send` mas não `Sync`: guarde em um `Mutex` e chame de uma
thread de trabalho (`spawn_blocking`). Toda escrita composta roda em um SAVEPOINT (tudo ou nada).
Erros são `miupy_core::Error`; `err.code()` devolve o código do ipc (`not_found`,
`invalid_input`, `conflict`, `io`, `unavailable`, `internal`) e o erro serializa como
`{ code, message }`.

## Normalização de volume

`Database::set_loudness` salva loudness integrado e pico verdadeiro por faixa;
`Track.loudness_lufs` expõe só o loudness. `Settings.normalization` começa em `normal`.
`Normalization::gain_db` calcula o ganho por faixa e limita picos conforme o modo.

## API nova para a casca

```rust
Database::list_tracks_by_artist(&self, artist_id: &str, query: &TrackQuery) -> Result<Vec<Track>>
Database::get_tracks_by_ids(&self, track_ids: &[String]) -> Result<Vec<Track>>
Database::wrapped_years(&self, tz_offset_minutes: i32) -> Result<Vec<i32>>
Database::taste_profile(&self, playlist_id: Option<&str>) -> Result<TasteProfile>
Database::recommendation_seeds(&self, playlist_id: Option<&str>, track_limit: u32,
    artist_limit: u32, genre_limit: u32) -> Result<Vec<RecommendationSeed>>
Database::upsert_recommendation_candidate(&self, candidate: &RecommendationCandidate) -> Result<bool>
Database::dismiss_recommendation(&self, source_id: &str) -> Result<()>
Database::list_recommendations(&self, playlist_id: Option<&str>, limit: Option<u32>) -> Result<Vec<Recommendation>>
```

`list_recommendations` só lê o banco. A casca implementa `recommendations_refresh` com rede:
pede sementes, busca, grava candidatas e chama `list_recommendations`. O `bool` do upsert indica
se a candidata foi gravada; `false` significa que já está na discoteca ou foi dispensada.
Para uma semente `similarTrack`, copie `RecommendationSeed.track_id` para
`RecommendationCandidate.seed_track_id`. Os tipos expostos serializam em camelCase.

## Mapa dos módulos (`src/`)

| Arquivo | Conteúdo | Requisitos |
|---|---|---|
| `lib.rs` | Reexporta a API pública | — |
| `db.rs` | `Database`: abertura (arquivo/memória), PRAGMAs, migrações, relógio injetável, savepoints | RNF-01 |
| `error.rs` | `Error`, `Result`, `Error::code()` | RNF-01 |
| `model.rs` | Tipos de domínio (serde camelCase), `NewTrack`, patches | todos |
| `tracks.rs` | upsert/dedupe, `get_track`, `list_tracks` (busca + ordenação), favoritos, metadados, apagar | RF-04 05 08 09 |
| `artists.rs` | get-or-create por nome, `ArtistInfo`, atualização | RF-05 RF-12 |
| `playlists.rs` | CRUD, adicionar/remover/mover faixas, listagem com busca e ordenação | RF-06 07 08 |
| `downloads.rs` | fila de downloads, progresso, cancelamento, `mark_downloaded`, playlist inteira | RF-10 RF-11 |
| `activity.rs` | eventos de reprodução (`start_play` / `add_listened` / `finish_play`) e log de ações | RF-13 |
| `stats.rs` | `library_stats`, `wrapped(year, tz)`; regra de "play" documentada no cabeçalho | RF-14 RF-15 |
| `recommendations.rs` | perfil de gosto, sementes, candidatas e ranking local | RF-19 |
| `profile.rs` | perfil (com validação) e configurações | RF-16 RF-17 |
| `lyrics.rs` | letras (plain/synced) | RF-12 |
| `text.rs` | normalização (sem acento), montagem segura de consulta FTS5, collation | RF-08 |
| `time.rs` | limites do ano civil em ms, sem dependência de `chrono` | RF-15 |
| `testutil.rs` | auxiliares só de teste | — |
| `../migrations/*.sql` | migrações versionadas (`PRAGMA user_version`) | RNF-01 |

## Como testar

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p miupy-core            # banco em memória, sem rede nem arquivos
cargo clippy -p miupy-core --all-targets
cargo fmt -p miupy-core
```

Os testes usam `Database::open_in_memory()` e um relógio injetável (`Database::set_clock`);
estatísticas recebem o fuso por parâmetro, então nada depende da hora nem do fuso da máquina.
Só um teste toca o disco (abre um arquivo temporário para conferir WAL e reabertura).

## Migrações

Para mudar o esquema, **acrescente** um arquivo `migrations/000N_descricao.sql` e registre-o em
`migrations()` (`db.rs`). Nunca edite uma migração já publicada. O teste
`migracoes_sao_validas` confere a lista.
