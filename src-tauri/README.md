# src-tauri — casca do app

Única parte do miupy que conhece o Tauri. Implementa o contrato de
[`docs/arquitetura/ipc.md`](../docs/arquitetura/ipc.md) sobre as crates `miupy-core`, `miupy-audio`
e `miupy-media`, e faz a orquestração que cruza as três.

## Estrutura

| Arquivo | Papel |
|---|---|
| `src/lib.rs` | Montagem do app: estado, plugins, registro dos comandos, encerramento. |
| `src/state.rs` | `AppState`: banco, backend de áudio, resolver, provedor de metadados, serviços. |
| `src/db.rs` | `Mutex<Database>` com acesso por closure (o banco do core é síncrono). |
| `src/paths.rs` | Diretórios de dados: `miupy.db`, `audio/`, `covers/`. |
| `src/player.rs`, `queue.rs` | Player e fila: reprodução local ou por URL, próxima/anterior, avanço automático e registro de atividade. |
| `src/listening.rs` | Contabiliza o tempo realmente ouvido para o registro de atividade. |
| `src/downloads.rs` | Fila de downloads em segundo plano, com progresso e cancelamento. |
| `src/youtube.rs`, `metadata.rs` | Busca/adição do YouTube e enriquecimento de metadados. |
| `src/recommendations.rs` | Busca limitada de candidatas e ranking local pelo núcleo. |
| `src/events.rs` | Emissão de eventos do player, downloads, discoteca e recomendações. |
| `src/dto.rs`, `error.rs`, `tz.rs`, `gpu.rs` | Tipos do IPC, erros, fuso local e escolha do renderizador. |
| `src/commands/` | Os comandos Tauri, um arquivo por domínio; só repassam para os serviços. |

## Rodar

```sh
npm run tauri dev                      # desenvolvimento, com recarga
npm run tauri build                    # pacote de distribuição
cargo test -p miupy                    # testes da orquestração (sem janela)
```

Os dados ficam em `~/.local/share/dev.miupy.app/` no Linux (banco, áudio, capas e `logs/`).

`MIUPY_CONTACT` define o contato enviado no `User-Agent` ao MusicBrainz; sem ela, usa o e-mail do
perfil.

## Notas

- **GPU no Linux:** `MIUPY_GPU=auto` (padrão) mantém DMABUF ligado e o desliga ao detectar Wayland
  com driver NVIDIA proprietário. `MIUPY_GPU=on` força GPU; `MIUPY_GPU=off` força software.
  `WEBKIT_DISABLE_DMABUF_RENDERER`, se já estiver definida, tem prioridade. O modo e o motivo
  são registrados no log. Para conferir visualmente: `MIUPY_GPU=on npx tauri dev` e
  `MIUPY_GPU=off npx tauri dev`.
- **yt-dlp:** precisa estar no `PATH`. Sem ele, busca e download devolvem erro `unavailable`.
- **Sem dispositivo de áudio** o app abre normalmente; o erro só aparece ao tentar tocar.
- **Streaming:** faixas do YouTube sem arquivo resolvem uma URL temporária; o backend recebe
  cabeçalhos e duração. URL expirada ao retomar ou erro do backend provoca uma nova resolução.
- **Recomendações:** `recommendations_list` consulta apenas SQLite; `refresh` consulta até nove
  sementes, com três chamadas simultâneas, e grava candidatas para o ranking local. Lives sem
  duração e vídeos fora de 30 segundos a 15 minutos são ignorados.
- **Metadados:** MusicBrainz fornece gêneros, ano e identificadores. O vídeo complementa título,
  artista, álbum e ano; categorias do YouTube não são gravadas como gênero.

## Android (pendente)

1. Instalar Android SDK, NDK e os targets Rust (`rustup target add aarch64-linux-android ...`).
2. `npx tauri android init` e `npx tauri android dev`.
3. Trocar o `NullBackend` por um plugin Kotlin com Media3 (`MediaSessionService`).
4. Definir o extrator do YouTube: o `yt-dlp` não roda como processo no Android.
