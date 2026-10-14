# Agente media — correção de metadados e serviços do YouTube

- **Data:** 2026-10-14
- **Autor:** Codex
- **Modelo:** GPT-6 (esforço não informado)
- **Escopo:** `crates/miupy-media` — RF-09, RF-20, RF-23, RF-26 e RF-27

## O que foi feito

- Corrigi a leitura de títulos com artista, álbum e faixa. Os três vídeos dos Racionais foram
  buscados com yt-dlp e seus JSONs foram capturados como fixtures. O parser usa `track`, `artist`,
  `album` e `creator` quando presentes, reconhece o artista confirmado pelo canal e conserva o
  comportamento anterior em títulos de três partes ambíguos. Incluí um JSON de controle do vídeo
  “Get Lucky”. A API de reprocessamento consulta novamente o vídeo por `sourceId` e aceita um
  `MetadataProvider` opcional para corrigir o registro no MusicBrainz.
- Filtrei buscas e mixes por duração acima de dez minutos e por termos de coletânea. `search` e
  mixes pedem até três vezes o limite ao yt-dlp para compensar resultados filtrados. “Mix” curto
  continua aceito.
- Implementei sugestões do YouTube, com consulta vazia sem rede, timeout de três segundos e erro
  tipado `network`.
- Implementei o gerenciador do binário oficial do yt-dlp: release do GitHub, asset por plataforma,
  checksum SHA-256, instalação temporária seguida de rename, progresso, status persistido, intervalo
  de verificação de 24 horas e override `MIUPY_YTDLP`. A variante `update_with_status` emite os
  estados completos `downloading`, `ready` e `error` no callback.
- Implementei foto de artista por MBID através de MusicBrainz, Wikidata e Commons, com thumbnail de
  500 px, bytes, extensão e crédito em texto.
- Mantive as assinaturas existentes de configuração. Os endpoints Wikimedia são internos ao
  crate; `MetadataConfig` não ganhou campos obrigatórios.

## API pública para a casca

- `pub async fn YtDlpResolver::reprocess_track(&self, source_id: &str, provider: Option<&dyn MetadataProvider>) -> Result<CorrectedTrack>`
- `pub struct CorrectedTrack { pub title: String, pub artist: String, pub album: Option<String>, pub release_year: Option<u32> }`
- `pub fn corrected_track(video: &serde_json::Value) -> CorrectedTrack`
- `pub fn is_song_result(entry: &SearchResult) -> bool`
- `pub async fn suggest(query: &str) -> Result<Vec<String>>`
- `pub async fn suggest::suggest_from(endpoint: &str, query: &str) -> Result<Vec<String>>` (endpoint injetável para testes)
- `pub async fn OnlineMetadata::artist_photo(&self, mbid: Option<&str>) -> Result<Option<ArtistPhoto>>`
- `pub struct ArtistPhoto { pub bytes: Vec<u8>, pub extension: String, pub credit: String }`
- `pub enum YtDlpState { Missing, Downloading, Ready, Error }`
- `pub struct YtDlpStatus { pub state: YtDlpState, pub version: Option<String>, pub path: Option<PathBuf>, pub checked_at: Option<u64>, pub progress: f32, pub error: Option<String> }`
- `pub fn YtDlpManager::new(dir: impl Into<PathBuf>) -> Result<Self>`
- `pub fn YtDlpManager::with_release_url(dir: impl Into<PathBuf>, release_url: impl Into<String>) -> Result<Self>` (endpoint injetável para testes)
- `pub fn YtDlpManager::binary_path(&self) -> PathBuf`
- `pub async fn YtDlpManager::status(&self) -> YtDlpStatus`
- `pub async fn YtDlpManager::ensure_current(&self, progress: Option<ProgressFn>) -> Result<YtDlpStatus>`
- `pub async fn YtDlpManager::update(&self, force: bool, progress: Option<ProgressFn>) -> Result<YtDlpStatus>`
- `pub type YtDlpStatusFn = Arc<dyn Fn(YtDlpStatus) + Send + Sync>`
- `pub async fn YtDlpManager::ensure_current_with_status(&self, callback: Option<YtDlpStatusFn>) -> Result<YtDlpStatus>`
- `pub async fn YtDlpManager::update_with_status(&self, force: bool, callback: Option<YtDlpStatusFn>) -> Result<YtDlpStatus>`

O caminho retornado por `binary_path` ou `YtDlpStatus.path` pode ser passado a
`YtDlpResolver::with_binary`. O callback de progresso usa o `ProgressFn` já exportado pelo crate.

## Decisões

- Sugestões usam `https://suggestqueries.google.com/complete/search`: a variante
  `suggestqueries-clients6.youtube.com` com `client=firefox` retornou HTTP 400 na verificação real.
  O endpoint escolhido respondeu com JSON e acentos UTF-8.
- A data de upload não é tratada como ano de lançamento. Sem campo de lançamento, o reprocessamento
  pode usar `MetadataProvider` para buscar álbum e ano no MusicBrainz.
- O binário oficial `2026.08.19` realizou busca real sem argumento explícito de runtime JS. Nesta
  máquina estão disponíveis `/usr/bin/deno` e `/usr/bin/node`; se for necessário fixar o runtime,
  o yt-dlp aceita `--js-runtimes deno`.
- Não atualizei `docs/arquitetura/arquitetura.json`, `docs/arquitetura/ipc.md` nem rodei
  `npm run docs:matriz`: o briefing limitou os arquivos editáveis à crate media e a este relatório.

## Verificação

### Verificado rodando

- Busca real com `/usr/bin/yt-dlp` confirmou os vídeos `q_xeCu3P9hU`, `fgysUhl98As` e
  `pL4bzW2qGqQ`; a extração direta confirmou títulos, canais e ausência dos campos musicais que
  acionam o parser. O controle “Get Lucky” foi consultado diretamente e salvo como fixture.
- `cargo test -p miupy-media`: 63 testes passaram e 1 doctest passou.
- `cargo fmt --all`: concluído.
- `cargo clippy --workspace --all-targets`: concluído sem warnings na execução final.
- `cargo test --workspace`: 227 testes passaram, 13 foram ignorados, e 1 doctest passou.
- O teste `falha_do_processo_vira_erro_tipado_e_limpa` passou 200/200 execuções. Repeti o loop
  enquanto os 63 testes da crate executavam em paralelo; ambos passaram. Não reproduzi ETXTBSY,
  então não alterei o lançamento do processo sem evidência da causa.
- `cargo test -p miupy-media -- --ignored --nocapture` foi executado. Na primeira execução, 9/11
  testes passaram; a sugestão recebeu HTTP 400 no endpoint inicial e a busca do binário oficial
  ficou vazia porque a consulta ampla só retornou coletâneas. Após corrigir endpoint e consulta,
  a execução posterior aprovou 8/11: sugestões, binário oficial baixado e busca real, MusicBrainz
  na primeira execução, e busca/download/mix/playlist/stream do yt-dlp passaram. A foto Wikimedia
  também passou na primeira execução, resolvendo ao vivo o MBID `056e4f3e-d505-4dad-8ec1-d04f521cbb56`
  e baixando 34.998 bytes com crédito `Sony Music Entertainment — CC BY 4.0`.
- Nas duas execuções posteriores, três verificações contra serviços externos falharam com erro de
  envio: MusicBrainz, Cover Art Archive e a busca inicial ao MusicBrainz do teste de foto. Um
  `curl` direto aos hosts MusicBrainz e Cover Art Archive confirmou EOF durante a conexão TLS.
  Não foi possível concluir 11/11 por essa indisponibilidade externa; os testes locais de fixtures
  e os demais testes de rede passaram.
- O binário gerenciado baixado do GitHub foi a versão `2026.08.19` e executou uma busca real no
  YouTube sem `--js-runtimes` explícito. O teste foi repetido com sucesso após trocar a consulta
  por uma busca de faixa avulsa.

### Verificado em testes locais

- As rotas de checksum correto/incorreto, download truncado e reaproveitamento da mesma versão foram
  exercitadas por servidor HTTP local, sem rede externa. O callback também foi testado para estados
  `downloading`, `ready` e `error`; os testes unitários do gerenciador passaram.
- As fixtures de MusicBrainz, Wikidata e Commons validam o encadeamento dos parsers localmente.

### Só implementado, sem caso de teste dedicado

- O override `MIUPY_YTDLP` e o intervalo de 24 horas estão implementados, mas não têm testes
  unitários próprios. O reaproveitamento de versão foi verificado por fixture local.

## Pendências e problemas conhecidos

- Os testes ignorados de MusicBrainz, Cover Art Archive e foto dependem de conexões TLS externas
  que falharam intermitentemente nesta sessão; a foto completou com sucesso na primeira execução.
- Os arquivos de arquitetura e a matriz não foram alterados por restrição explícita do briefing.

## Próximos passos

- A casca pode encaminhar os eventos de `update_with_status`/`ensure_current_with_status` ao
  frontend no formato IPC de `YtDlpStatus`.
- Repetir os testes de rede de MusicBrainz/Cover Art quando os hosts aceitarem conexões TLS.
