# Frente media — yt-dlp real, streaming e metadados

- **Data:** 2026-10-13
- **Autor:** agente media
- **Modelo:** gpt-6-sol (esforço high), via `codex exec`
- **Escopo:** verificar `miupy-media` contra serviços reais e entregar RF-18, RF-19 e RF-20 na crate.

## O que foi feito

- Exercitei busca e download com `/usr/bin/yt-dlp` 2026.08.19 e `ffmpeg`. Substituí as fixtures manuais de busca e playlist por JSON capturado, aparado ao necessário. Acrescentei captura do JSON do vídeo, linhas reais de progresso e mensagens de erro. Removi parâmetros transitórios de miniaturas e substituí a URL assinada de áudio por `audio.example.test` na fixture; o teste de rede obtém uma URL nova.
- `MediaResolver` ganhou as assinaturas `async fn resolve_stream(&self, video_id: &str) -> Result<StreamAudio>`, `async fn related_tracks(&self, video_id: &str, limit: usize) -> Result<Vec<SearchResult>>` e `async fn video_metadata(&self, video_id: &str) -> Result<VideoMetadata>`. `StreamAudio` traz `url`, `http_headers`, `codec`, `container`, `bitrate_kbps`, `duration_ms` e `expires_at` (segundos Unix). `VideoMetadata` traz `artist`, `track`, `album`, `release_year` e `genres`.
- Streaming usa o formato `140/bestaudio[ext=m4a]` e rejeita codecs fora de AAC/M4A. O teste ignorado resolveu o vídeo `jNQXAC9IVRw`, fez GET com `Range: bytes=0-4095` e obteve HTTP 206 e cabeçalho MP4 (`ftyp`).
- Faixas relacionadas vêm da playlist plana `RD<videoId>`, removendo a semente e duplicatas. Um mix ausente ou não visualizável devolve lista vazia. A busca por artista/gênero reutiliza `search`.
- `OnlineMetadata::find_recording` faz busca e consulta de detalhes da gravação pelo MusicBrainz, ambas sujeitas ao limitador existente. `RecordingMatch` ganhou `genres`, `release_year` e `isrc`; gêneros/tags são normalizados, deduplicados, ordenados por votos e limitados a 8. Com duração informada, uma gravação distante mais de 10 s deixa de ser aceita. `tagger::TagUpdate` ganhou `genres` e `release_year`; `AudioInfo` lê os mesmos campos. O JSON do yt-dlp fornece campos secundários pela nova `video_metadata`.
- A API pública é exportada por `miupy_media::{StreamAudio, VideoMetadata, RecordingMatch, TagUpdate, AudioInfo}` e pelos métodos acima. `MetadataProvider` mantém as mesmas assinaturas.

## Decisões

- O resolver usa um caminho configurável para o executável, sem decidir se o aplicativo usará o binário do sistema ou o distribuirá. Não foi adicionada fonte de biografia/foto nem dependência de Tauri ou das outras crates.
- Para evitar associar ano e gêneros de outra gravação homônima, a duração conhecida funciona como critério de rejeição. O MusicBrainz ainda pode devolver uma edição/compilação com duração igual; a casca deve tratar os dados como enriquecimento, não identidade garantida.
- `categories` e `tags` do yt-dlp são fontes secundárias, pois podem descrever o vídeo, não um gênero musical.

## Mudanças que quebram consumidores

- Todo fake de `MediaResolver` precisa implementar `resolve_stream`, `related_tracks` e `video_metadata`. Em especial, `src-tauri/src/downloads.rs` contém `impl MediaResolver for FakeResolver`.
- Literais de `TagUpdate` precisam declarar `genres` e `release_year` ou usar `..Default::default()`. Há literais em `src-tauri/src/downloads.rs` e `src-tauri/src/metadata.rs`; a casca deve preencher os novos campos quando tiver dados.
- O literal `RecordingMatch` de `src-tauri/src/metadata.rs` precisa dos campos `genres`, `release_year` e `isrc`.
- A seleção de `find_recording` com duração informada pode devolver `None` onde antes devolvia um resultado com duração incompatível. Uma correspondência retorna após duas requisições MusicBrainz, respeitando o intervalo configurado.

## Comportamento real do yt-dlp

- `--flat-playlist --dump-json` devolveu uma linha JSON por resultado de busca; `--dump-single-json` devolveu um objeto com `entries` para playlist. Os campos de duração são em segundos; miniaturas têm parâmetros transitórios.
- O download real em M4A produziu linhas `MIUPYPROG <baixado> <total> NA` e `MIUPYFILE\t<caminho>`. A contagem alcançou 309288 bytes; o arquivo final teve 309156 bytes. O parser existente de progresso já lidava com isso.
- `RDjNQXAC9IVRw` e `RDdQw4w9WgXcQ` responderam `This playlist type is unviewable`; mix indisponível é um caso normal. O código o classifica como não encontrado e o método público devolve lista vazia.
- Capturas reais de vídeo indisponível, formato pedido indisponível e falha de conexão via proxy local passaram pelos testes offline de classificação. Restrição por idade/região foi coberta com mensagens sintéticas, pois não houve captura real estável desses casos.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `cargo test -p miupy-media -- --ignored --nocapture --test-threads=1` antes das mudanças | 5/5 testes de rede passaram: MusicBrainz/artista, Cover Art Archive, LRCLIB, busca e download yt-dlp. |
| `yt-dlp` direto para busca, playlist, vídeo, download e erros | Busca, playlist pública, metadados e download M4A responderam; as saídas aparadas formam as fixtures. Mix retornou indisponível. |
| `cargo fmt --all` | passou. |
| `cargo clippy -p miupy-media --all-targets -- -D warnings` | passou, 0 warnings. Uma rodada anterior apontou `unwrap_or_else(BTreeMap::new)`; corrigi para `unwrap_or_default` e rodei novamente. |
| `cargo test -p miupy-media` | 56 testes unitários + 1 doc-test passaram; 8 testes de rede ignorados por padrão. |
| `cargo test -p miupy-media -- --ignored --nocapture --test-threads=1` após as mudanças | 8/8 testes de rede passaram: Cover Art Archive, LRCLIB, MusicBrainz/artista, busca, download, mix, playlist e GET parcial de streaming yt-dlp. |
| `cargo test -p miupy-media --test network ytdlp_playlist_real -- --ignored --nocapture` | 1/1 passou; `resolve_playlist` retornou entradas reais. |
| `git diff --check && git add -A && git diff --cached --check && git commit -m 'Implementa streaming e metadados reais na crate media'` | Falhou em `git add -A`: `Unable to create '/home/cabeto/Documentos/Projetos/miupy/.git/worktrees/s02-media/index.lock': Read-only file system`. Nenhum commit foi criado. |

## Pendências e problemas conhecidos

- Integração na casca e no player de streaming fica para as outras frentes. Esta frente só alterou `crates/miupy-media/**` e este relatório.
- O mix `RD<videoId>` pode permanecer indisponível conforme o YouTube; `related_tracks` então devolve lista vazia. A recomendação ainda pode usar `search` por artista e gênero.
- O MusicBrainz devolveu uma gravação de “Get Lucky” de 294 s com ano 2013 e tags `funk`/`happy`, mas a release associada era uma compilação; a seleção de álbum pelo MusicBrainz pode precisar de ajuste editorial futuro.
- Os cenários reais de restrição por idade e região não foram verificados contra vídeos restritos.
- O gitdir do worktree está somente para leitura neste sandbox. O commit pedido ficou pendente; as mudanças e este relatório estão no diretório de trabalho, sem stage. Não tentei contornar a restrição.

## Próximos passos

- A frente de integração deve adaptar os fakes e literais citados, usar `resolve_stream` no caminho de reprodução sem arquivo local e consumir os metadados novos no download/atualização.
- A orquestração deve rodar a matriz e as verificações do workspace ao reunir as frentes.
- A orquestração deve fazer `git add -A` e commit deste worktree em ambiente com escrita no gitdir.

## Nota da orquestração (revisão antes do merge)

O commit foi feito pela orquestração: o sandbox do agente não deixa escrever no diretório git do
worktree. A conclusão acima de que o mix `RD<videoId>` fica indisponível estava errada: só a URL
`playlist?list=RD<id>` responde "unviewable". Pela forma `watch?v=<id>&list=RD<id>` o mix devolve
faixas. `related_tracks` passou a usar essa URL, e o teste de rede do mix agora exige lista não
vazia (antes passava com lista vazia e não provava nada). Rodado pela orquestração depois da
correção: clippy com 0 warnings, 56 testes offline e os 8 de rede passando.
