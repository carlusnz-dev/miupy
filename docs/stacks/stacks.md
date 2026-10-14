# Stack do miupy

Versões conferidas no crates.io / npm em 2026-10-13.

## Stack adotada

| Camada | Escolha | Por quê |
|---|---|---|
| App | **Tauri 2** (`tauri` 2.12) | Desktop + Android com o mesmo núcleo Rust. O 3.0 ainda é alpha. |
| Frontend | **Svelte 5 + Vite 8 + TypeScript** | Bundle pequeno e pouca sobrecarga de runtime, o que pesa no WebView do Android. |
| Estilo | **Tailwind 4** + *design tokens* em CSS variables | O tema da lava vira troca de variáveis (`data-lava`), sem recompilar nada. |
| Áudio (desktop) | **rodio 0.23** (cpal + symphonia por baixo) | Entrega play/pause/volume/seek prontos; ver "Erros do rascunho" nº 2. |
| Áudio (Android) | **Plugin Tauri em Kotlin com Media3/ExoPlayer** | Único jeito correto de ter reprodução em segundo plano; ver nº 3. |
| Banco | **rusqlite 0.40** (`bundled`) + **rusqlite_migration** | SQLite embutido, sem depender do sistema; migrações versionadas; FTS5 para busca. |
| Tags | **lofty 0.25** | Leitura/escrita de tags e capa em MP4/M4A, MP3, FLAC, Ogg. |
| YouTube (desktop) | **yt-dlp** como processo externo | A única ferramenta que acompanha as quebras do YouTube; ver nº 4. |
| YouTube (Android) | **rustypipe 0.11** (feature opcional) | Rust puro, roda no Android. É GPL-3.0; ver "Decisões em aberto". |
| Metadados | **reqwest** direto em MusicBrainz, Cover Art Archive e LRCLIB | Três endpoints JSON simples; não justifica uma dependência por API. |
| Assíncrono | runtime do próprio Tauri (`tauri::async_runtime`, tokio) | Evita dois runtimes. |
| Formato de áudio | **M4A/AAC** (itag 140, ~128 kbps) como padrão | Decodifica em Rust puro e toca nativo no ExoPlayer; ver nº 1. |

## Erros e pontos fracos do rascunho (Gemini)

1. **`symphonia` não tem feature `opus`.** O rascunho lista `features: mp3, aac, opus, vorbis, flac`
   e manda salvar `{track_id}.opus`. Na 0.6.1 os codecs são `aac, adpcm, alac, flac, mp1-3, pcm,
   vorbis`: não existe decodificador Opus. O projeto nem compilaria com essa feature, e os arquivos
   baixados não tocariam. Correção: baixar M4A/AAC. Opus fica para depois, via `libopus`, se a
   diferença de qualidade importar.
2. **Motor de áudio escrito à mão (cpal + symphonia + ring buffer).** Funciona, mas é reimplementar
   o `rodio`: semanas de trabalho em resampling, seek e underrun para chegar no mesmo lugar. Além
   disso o rascunho oferece `crossbeam-channel` como alternativa ao `ringbuf` na ponte com o callback
   de áudio, e canal não é seguro para tempo real (pode bloquear e alocar). Com `rodio` o problema
   some; canal fica só para comandos, fora do callback.
3. **Android via "JNI bindings" em `platform/android.rs`.** O Tauri 2 tem sistema de plugins mobile
   em Kotlin; ninguém escreve JNI na mão. E o ponto que o rascunho não trata: tocar pelo cpal/AAudio
   dentro do processo da Activity faz o Android matar o áudio em segundo plano. Reprodução real
   exige `MediaSessionService` em foreground, ou seja, Media3. Por isso o áudio é uma *trait* com
   duas implementações.
4. **`rusty_ytdl` como extrator.** Quebra a cada mudança do YouTube (assinatura, PO token) e
   atualiza devagar. `yt-dlp` é o que se mantém funcionando. Em qualquer opção, esta é a parte mais
   frágil do sistema, então fica isolada atrás de uma *trait* para ser trocada sem tocar no resto.
5. **Busca começando pelo MusicBrainz.** O requisito é pesquisar o que existe no YouTube. A busca
   vai direto ao YouTube; o MusicBrainz entra depois, para enriquecer. Ele também limita a
   1 requisição/s e exige `User-Agent` identificado, o que inviabiliza busca interativa.
6. **`cargo ndk` como "Prioridade 3".** No Tauri 2 o caminho é `tauri android init/dev/build`, que
   já cuida do NDK e dos quatro targets (o rascunho só cita dois).
7. **Esquema incompleto para os requisitos.** Faltam favoritos, histórico de reprodução (sem ele não
   há estatísticas nem retrospectiva), perfil, configurações e fila de downloads. `file_path NOT NULL
   UNIQUE` impede cadastrar uma faixa antes do download; não há ID de origem (vídeo do YouTube) para
   deduplicar; faltam `ON DELETE CASCADE` e índices.
8. **`Heed` (LMDB) no diagrama.** Segundo banco sem função: capas vão para o disco e o resto para o
   SQLite. Removido.
9. **Frontend "HTML/TS/Tailwind" sem framework.** Player, fila, listas e downloads são estado
   reativo; sem framework isso vira código manual de sincronização de DOM.
10. **Tudo dentro de `src-tauri/`.** Acopla domínio ao Tauri e deixa o núcleo impossível de testar
    sem subir o app. Aqui o domínio vive em crates próprias e `src-tauri` é só cola.

O que o rascunho acertou e foi mantido: Tauri 2, SQLite via `rusqlite` bundled, `lofty`,
MusicBrainz + Cover Art Archive + LRCLIB, e a regra de nunca bloquear o callback de áudio.

## Decisões em aberto

- **Licença.** `rustypipe` é GPL-3.0: ligá-lo ao binário obriga o miupy a ser GPL-3.0. Por isso ele
  entra como feature opcional. Se a licença do projeto for GPL, vira o extrator padrão no Android
  sem problema; se for MIT/Apache, é preciso outra saída para o Android.
- **Dados de artista.** O MusicBrainz não tem biografia nem foto. As opções são TheAudioDB ou
  Last.fm (ambas pedem chave de API) ou Wikidata/Wikipedia (sem chave).
- **Distribuição do yt-dlp.** Usar o do sistema ou baixar e atualizar o binário pelo próprio app.

## Ferramentas do ambiente de desenvolvimento

- Rust 1.99 em `~/.cargo/bin` (não está no `PATH` do shell não interativo: use
  `export PATH="$HOME/.cargo/bin:$PATH"`).
- Node 26 + npm. CLI do Tauri via `@tauri-apps/cli` (devDependency), não `cargo-tauri`.
- Android: **SDK, NDK e targets Rust ainda não instalados** nesta máquina.
