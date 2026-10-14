# Crate miupy-core: domínio e persistência em SQLite

- **Data:** 2026-10-13
- **Autor:** agente core (subagente do Claude Code, orquestrado por Claude Opus 5.5 a pedido de Carlos Antunes)
- **Modelo:** Claude Sonnet, esforço high
- **Escopo:** `crates/miupy-core` inteira (modelos, erro, banco, repositórios, estatísticas, testes) e `docs/arquitetura/banco-de-dados.md`.

## O que foi feito

- Modelos serde em camelCase espelhando o `ipc.md` (`Track`, `Playlist`, `TrackQuery`, `Lyrics`, `Profile`, `Settings`, `DownloadJob`, `ArtistInfo`, `LibraryStats`, `Wrapped`, `RankedTrack`, `RankedArtist`…), mais tipos de entrada (`NewTrack`, `TrackPatch`, `PlaylistPatch`, `ArtistPatch`, `SettingsPatch`, `DownloadUpdate`, `DownloadedFile`).
- `Error` único com `code()` nos códigos do ipc (`not_found`, `invalid_input`, `conflict`, `io`, `unavailable`, `internal`) e serialização `{ code, message }`. Violação UNIQUE vira `conflict`, FK vira `invalid_input`, banco ocupado vira `unavailable`.
- `Database`: abertura por caminho (cria a pasta) e em memória, PRAGMAs (foreign_keys, WAL, synchronous NORMAL, busy_timeout 5 s), collation `miupy_nocase`, migração `0001_init.sql` via `rusqlite_migration`, relógio injetável, SAVEPOINT aninhável.
- Esquema: artists, tracks (file_path anulável, UNIQUE parcial `(source, source_id)`), favorites, playlists, playlist_tracks (cascata, posição contígua), lyrics, download_jobs, play_events, user_actions, profile, settings, `tracks_fts` (FTS5 `unicode61 remove_diacritics 2` + triggers).
- Operações por domínio (`impl Database`): faixas, artistas, playlists, downloads, atividade, estatísticas, perfil/configurações, letras. Lista completa em `crates/miupy-core/README.md`.
- 56 testes com banco em memória; anotações `// @req` nas implementações e `// @test` nos testes (RF-04 a RF-17 e RNF-01).
- `crates/miupy-core/README.md` e `docs/arquitetura/banco-de-dados.md` (esquema, decisões, regra de "play", fuso).

## Decisões

- **Regra de play:** evento concluído OU com pelo menos 30 s ouvidos. Plays alimentam contagens e rankings; o tempo ouvido soma tudo, inclusive eventos sub-30 s.
- **Fuso:** `wrapped(year, tz_offset_minutes)` e `started_at` local; o núcleo nunca lê o fuso do sistema. Evento pertence ao dia do início.
- **Apagar faixa apaga o histórico dela** (cascata em `play_events`); alternativa documentada no doc do banco.
- **Playlist sem UNIQUE em posição**, contiguidade garantida em código (UPDATE em massa quebraria uma restrição linha a linha).
- **Um job de download por faixa**; cancelar apaga o job e o downloader percebe pelo `not_found` em `update_download`.
- **Artista sem faixas não é apagado** (preserva o enriquecimento); some das listagens e contagens.
- **`thumbnail_url`** guardada na faixa mas fora do tipo `Track` (o ipc não a tem); leitura por `track_thumbnail_url`.
- Volume padrão 0.8, tema padrão `orange`. Posição fora do fim em `move_playlist_track` é limitada ao último lugar.
- Dependências: rusqlite 0.40.2 (`bundled`, `collation`), rusqlite_migration 2.6.0, serde, serde_json (gêneros, payload do log, testes), thiserror 2, uuid (v7).

## Verificação (comandos rodados e resultado real)

```
$ cargo fmt -p miupy-core -- --check
(sem saída, exit 0)

$ cargo clippy -p miupy-core --all-targets
    Checking miupy-core v0.1.0 (.../crates/miupy-core)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.60s
(0 warnings)

$ cargo test -p miupy-core
running 56 tests
...
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
Doc-tests miupy_core: 0 tests

$ cargo doc -p miupy-core --no-deps
Finished `dev` profile (sem warnings)
```

FTS5 confirmado no build bundled (teste `fts5_esta_disponivel` e todos os testes de busca passam). Não rodei `cargo clippy/test --workspace` nem `npm run docs:matriz` (fora do meu escopo de escrita: a matriz gera arquivos em `docs/rastreabilidade/`).

## Pendências e problemas conhecidos

- A matriz de rastreabilidade não foi regenerada; rodar `npm run docs:matriz` depois que os outros agentes terminarem.
- `docs/arquitetura/arquitetura.json` não foi atualizado (fora do meu escopo).
- Horário de verão no meio do ano não é modelado em `wrapped` (um deslocamento fixo por chamada). Para o Brasil atual não importa.
- Ordenação sem acento cobre as letras latinas comuns (tabela própria em `text.rs`), não é collation Unicode completa.
- Sem teste de concorrência (dois processos/conexões); a API assume uma conexão atrás de um `Mutex`.
- Sem teste contra um banco real com dados migrados de versão anterior (só existe a migração 0001).
- `library_stats` e `wrapped` fixam o top em 10 itens (`TOP_LIMIT`); não é parâmetro.
- Eventos de reprodução não são limitados à duração da faixa (reouvir um trecho com seek pode somar mais que a duração).

## Próximos passos

- A casca (`src-tauri`) cria `Database` no `app_data_dir()`, chama `requeue_interrupted_downloads` e `close_dangling_play_events` na abertura e embrulha tudo em `Mutex`.
- Mapear `Error` para o erro dos comandos Tauri (já serializa como `{ code, message }`) e `Wrapped`: passar `-new Date().getTimezoneOffset()` do frontend (ou o fuso do SO) como `tz_offset_minutes`.
- Calcular `SearchResult.inLibrary` com `Database::known_source_ids`.
