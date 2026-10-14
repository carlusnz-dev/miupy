# Crate miupy-media: busca/download, metadados e tags

- **Data:** 2026-10-13
- **Autor:** agente media (subagente do Claude Code, orquestrado por Claude Opus 5.5 a pedido de Carlos Antunes)
- **Modelo:** Claude Sonnet, esforço medium
- **Escopo:** implementar `crates/miupy-media` (RF-05, RF-09, RF-10, RF-11, RF-12) sem tocar em outras partes do repositório.

## O que foi feito

Arquivos em `crates/miupy-media/` (`src/`, `tests/network.rs`, `tests/fixtures/`, `README.md`):

- `resolver.rs`: trait `MediaResolver` (`search`, `download_audio`, `resolve_playlist`) e tipos `SearchResult`, `PlaylistInfo`, `DownloadRequest` (progresso `Arc<dyn Fn(f32)>` e `CancellationToken`), `DownloadedAudio`.
- `ytdlp.rs`: `YtDlpResolver` (processo `yt-dlp`, binário configurável, `check_available()`). Busca com `--flat-playlist --dump-json ytsearchN:`; playlist com `--dump-single-json`; download com `-f 140/bestaudio[ext=m4a]`, progresso por `--progress-template`, cancelamento que mata o processo e apaga parciais, propriedades finais lidas do arquivo com lofty.
- `metadata.rs`: trait `MetadataProvider` e `OnlineMetadata` (MusicBrainz: gravação e artista; Cover Art Archive: capa; LRCLIB: letra simples e sincronizada; download de imagem genérico, usado também para a miniatura do YouTube). User-Agent `miupy/<versão> (<contato>)`, timeouts, erros tipados.
- `ratelimit.rs`: `RateLimiter` real (1 req/s no MusicBrainz), sobre `tokio::time`.
- `tagger.rs`: `read_tags` e `write_tags` (título, artista, álbum, capa) via lofty; testado em M4A, MP3 e FLAC.
- `title.rs`: `clean_title(título, canal)` -> artista, título e participações.
- `error.rs`: `MediaError` com `code()`.

## Decisões

- `Cancelled` é uma variante própria com `code() == "cancelled"`, fora da lista do `ipc.md` (cancelar não é erro para o usuário). A camada Tauri deve tratá-la antes de montar `{code, message}`.
- Sem fallback de formato: se não houver M4A, erro `unavailable` (Opus não toca no symphonia).
- Consulta de busca nunca vira opção: vai dentro de `ytsearchN:<consulta>`; ids de vídeo/playlist são validados e a URL é remontada pela crate; `--` antes de URLs; `--ignore-config`.
- `reqwest` 0.13 com feature `rustls` (usa `aws-lc-rs`, código C): funciona no desktop, vai exigir cuidado na compilação para Android.
- Recording do MusicBrainz: pontuação mínima 70; releases candidatos ordenados (oficiais e sem tipo secundário primeiro) em `release_mbids`.
- `rustypipe` não foi adicionado (GPL-3.0); só documentado no README e na doc da trait.
- Bio e foto de artista: campos `None`, ponto de extensão documentado.
- `ArtistInfo.trackCount`, `imagePath`, `inLibrary` etc. do IPC são da camada Tauri/core, não desta crate.

## Verificação (comandos rodados e resultado real)

```
$ cargo fmt -p miupy-media            -> sem saída (ok)
$ cargo clippy -p miupy-media --all-targets
    Checking miupy-media v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.54s   (zero warnings)
$ cargo test -p miupy-media
test result: ok. 53 passed; 0 failed; 0 ignored   (unit: src/lib.rs)
test result: ok. 0 passed; 0 failed; 5 ignored    (tests/network.rs)
test result: ok. 1 passed; 0 failed               (doc-tests)
```

Testes de rede reais, rodados uma vez (`cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1 musicbrainz coverart lrclib`):

```
test coverart_download ... "/tmp/.tmpFlU5Go/capa.jpg" 35043 bytes ... ok
test lrclib_letra ... plain? true synced? true ... ok
test musicbrainz_recording_e_artista ... ok   (Get Lucky / Daft Punk, mbid do artista, país FR,
                                               gêneros electronic, house, french house, dance, disco)
test result: ok. 3 passed
```

Achados reais nessa rodada: (1) o MusicBrainz respondeu 503 quando duas instâncias (cada uma com o seu limitador) bateram em sequência rápida; (2) o primeiro release devolvido pela busca de gravação de "Get Lucky" era uma coletânea sem capa no Cover Art Archive (por isso `release_mbids` e a ordenação); o teste de capa usa um release fixo da documentação do CAA.

O que cada grupo de testes cobre: fixtures de JSON (busca yt-dlp, playlist, MusicBrainz, LRCLIB), linhas de progresso, montagem de argumentos (incluindo consulta que começa com `-`), 15 casos de limpeza de título, limitador de taxa com tempo pausado do tokio, servidor HTTP local (User-Agent, 404/503, resposta inválida, conexão recusada, download de imagem), tagger com M4A/MP3/FLAC gerados pelo ffmpeg (que existe nesta máquina; sem ffmpeg o teste imprime "PULADO" e passa).

## Pendências e problemas conhecidos

- **O caminho real do yt-dlp NÃO foi exercitado.** O `yt-dlp` não está instalado nesta máquina. Nunca rodei busca, download, playlist, `--progress-template` nem `--print after_move:` contra o binário de verdade. O que existe: parsing contra fixtures escritas a partir do formato conhecido do yt-dlp (não capturadas de uma execução real), argumentos testados como lista de strings, e um processo falso (script de shell) que prova o laço de leitura, o progresso, o cancelamento com limpeza e a classificação de erro. Pontos que podem falhar com o binário real: sintaxe do `--progress-template`/`--print`, campos do JSON de busca (`channel`, `duration`, `thumbnails`), mensagens de erro usadas em `classify_stderr`. Os testes `ytdlp_busca_real` e `ytdlp_download_real` em `tests/network.rs` (`#[ignore]`) existem para validar isso assim que o yt-dlp estiver instalado.
- Se o `yt-dlp` baixar um arquivo existente com o mesmo nome, ele o pula; a crate não força sobrescrita.
- Cancelamento apaga só parciais (`.part`, `.ytdl`, `.temp`, `.fNNN.*`), não um arquivo final que já exista.
- O codec de um M4A com ALAC também aparece como "aac" (lofty via `FileType::Mp4`).
- `find_recording` acha o release pela gravação; para capa de qualidade pode valer uma busca de release por álbum+artista (não implementada). Letras: `GET /api/get` só é tentado quando álbum e duração são conhecidos; senão cai direto na busca aproximada.
- Cover Art Archive pede `front-500`; imagens maiores não são suportadas.
- O limitador é por instância de `OnlineMetadata`; o app deve manter uma só (não é global).
- Sem `rustypipe`: no Android, `MediaResolver` não tem implementação ainda.
- `npm run docs:matriz` e `arquitetura.json` não foram tocados (fora do meu escopo de escrita).

## Próximos passos

- Instalar o `yt-dlp` e rodar `cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1`; corrigir o que divergir das fixtures.
- Camada Tauri: converter `SearchResult` -> IPC (`durationMs.unwrap_or(0)`), traduzir `MediaError::code()`, tratar `cancelled`, gravar tags com `spawn_blocking(write_tags)` e salvar capa com `download_image`.
- Decidir licença e, se GPL, implementar o resolver `rustypipe` para o Android.
- Decidir a fonte de bio/foto de artista (Wikidata/TheAudioDB/Last.fm).
