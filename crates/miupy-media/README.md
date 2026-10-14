# miupy-media

Tudo que fala com a internet ou mexe no arquivo de mídia. Não depende de Tauri nem das outras
crates do workspace: a camada `src-tauri` converte os tipos desta crate para o contrato do
`docs/arquitetura/ipc.md`. API assíncrona (tokio), exceto o `tagger` (síncrono, use
`spawn_blocking`).

## O que tem

| Módulo | Conteúdo | Requisitos |
|---|---|---|
| `resolver` | trait `MediaResolver` (`search`, `download_audio`, `resolve_playlist`, `resolve_stream`, `related_tracks`, `video_metadata`) e tipos de resultado | RF-09, RF-10, RF-11, RF-18, RF-19, RF-20 |
| `ytdlp` | `YtDlpResolver`: executa o binário `yt-dlp` | RF-09, RF-10, RF-11 |
| `metadata` | trait `MetadataProvider` e `OnlineMetadata` (MusicBrainz, Cover Art Archive, LRCLIB) | RF-12, RF-20 |
| `ratelimit` | `RateLimiter` (1 req/s no MusicBrainz) | RF-12 |
| `tagger` | `read_tags`, `write_tags` (lofty), inclusive gênero e ano | RF-05, RF-12, RF-20 |
| `title` | `clean_title`: "Artista - Música (Official Video)" -> artista + título | RF-12 |
| `error` | `MediaError` com `code()` (`network`, `unavailable`, `not_found`, `io`, `invalid_input`, `internal`; mais `cancelled`) | todos |

### Exemplo

```rust
let yt = YtDlpResolver::new();                       // ou ::with_binary("/caminho/yt-dlp")
let results = yt.search("daft punk get lucky", 10).await?;
let cancel = CancellationToken::new();
let audio = yt.download_audio(
    DownloadRequest::new(&results[0].video_id, "/musicas", "id-da-faixa")
        .with_progress(Arc::new(|p| println!("{:.0}%", p * 100.0)))
        .with_cancel(cancel.clone()),
).await?;                                            // /musicas/id-da-faixa.m4a

let meta = OnlineMetadata::new(MetadataConfig::new("meu@email.com"))?;
let parsed = clean_title(&results[0].title, &results[0].channel);
let rec = meta.find_recording(&parsed.title, parsed.artist.as_deref(), None).await?;
let stream = yt.resolve_stream(&results[0].video_id).await?; // URL temporária, cabeçalhos e expiração
let related = yt.related_tracks(&results[0].video_id, 20).await?; // mix, ou [] se indisponível
let fallback = yt.video_metadata(&results[0].video_id).await?; // dados secundários do vídeo
```

Crie **uma** instância de `OnlineMetadata` e compartilhe (`Arc`): o limitador de 1 req/s é por
instância, e o MusicBrainz limita por IP.
`find_recording` faz duas requisições espaçadas ao MusicBrainz (busca e detalhes) para preencher
gêneros, ano e ISRC. Com duração informada, uma gravação que difere mais de 10 s não é aceita.
Ao enriquecer a faixa, prefira os campos do MusicBrainz; `video_metadata` fornece a segunda fonte.

## Dependência externa: yt-dlp

O `YtDlpResolver` precisa do executável `yt-dlp` (no `PATH` ou em caminho informado). Sem ele,
toda operação devolve `MediaError::Unavailable` (`code() == "unavailable"`) com instruções
(`pipx install yt-dlp`, `brew install yt-dlp`, `winget install yt-dlp`, ou o binário de
https://github.com/yt-dlp/yt-dlp/releases). O YouTube muda com frequência: mantenha o yt-dlp
atualizado (`yt-dlp -U`). Decisão em aberto: usar o do sistema ou baixar/atualizar pelo app.

Formato pedido: `140/bestaudio[ext=m4a]` (M4A/AAC), sem pós-processamento com ffmpeg. Se o vídeo
não tiver M4A, o erro é `unavailable` (não há fallback para webm/opus: o symphonia não decodifica
Opus).

## Testes

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p miupy-media                      # offline: fixtures, servidor HTTP local, processo falso
# testes de rede (ignorados por padrão), um de cada vez para respeitar o limite do MusicBrainz:
cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1
# só os que não precisam de yt-dlp:
cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1 musicbrainz coverart lrclib
```

O teste do `tagger` usa o `ffmpeg` para gerar 1 s de áudio e é pulado (com mensagem) se ele não
existir. Os testes `ytdlp_*` de `tests/network.rs` exigem o `yt-dlp` instalado.
As fixtures de yt-dlp foram capturadas com a versão 2026.08.19 e aparadas. A URL assinada de áudio
foi substituída por `audio.example.test`; o teste ignorado de streaming obtém uma URL real nova e
faz GET parcial com os cabeçalhos retornados pelo resolver.

## Android e rustypipe

No Android não dá para executar o `yt-dlp`. O plano é uma segunda implementação de
`MediaResolver` com [`rustypipe`](https://crates.io/crates/rustypipe) (Rust puro), atrás de uma
feature opcional (por exemplo `rustypipe`) num módulo novo (`rustypipe_resolver.rs`). Essa
implementação precisa cobrir busca, download, playlist, streaming, relacionadas e metadados do
vídeo. **A dependência não foi adicionada**: o `rustypipe` é GPL-3.0 e a licença do
projeto está em aberto. Também ao adicionar, lembre que o `reqwest` com `rustls` usa `aws-lc-rs`
(código C), o que exige atenção na compilação cruzada para Android.

## Dados de artista

O MusicBrainz não tem biografia nem foto: `ArtistDetails::bio` e `image_url` são sempre `None`.
Ponto de extensão: outra implementação de `MetadataProvider` (TheAudioDB/Last.fm com chave, ou
Wikidata/Wikipedia sem chave) combinada com a `OnlineMetadata`.
