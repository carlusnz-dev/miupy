# src-tauri — casca do app

Única parte do miupy que conhece o Tauri. Implementa o contrato de
[`docs/arquitetura/ipc.md`](../docs/arquitetura/ipc.md) sobre as crates `miupy-core`, `miupy-audio`
e `miupy-media`, e faz a orquestração que cruza as três.

## Estrutura

| Arquivo | Papel |
|---|---|
| `src/lib.rs` | Montagem do app: estado, plugins, registro dos 37 comandos, encerramento. |
| `src/state.rs` | `AppState`: banco, backend de áudio, resolver, provedor de metadados, serviços. |
| `src/db.rs` | `Mutex<Database>` com acesso por closure (o banco do core é síncrono). |
| `src/paths.rs` | Diretórios de dados: `miupy.db`, `audio/`, `covers/`. |
| `src/player.rs`, `queue.rs` | Player e fila: próxima/anterior, avanço automático no fim da faixa. |
| `src/listening.rs` | Contabiliza o tempo realmente ouvido para o registro de atividade. |
| `src/downloads.rs` | Fila de downloads em segundo plano, com progresso e cancelamento. |
| `src/youtube.rs`, `metadata.rs` | Busca/adição do YouTube e enriquecimento de metadados. |
| `src/events.rs` | Emissão de `player://*`, `download://progress` e `library://changed`. |
| `src/dto.rs`, `error.rs`, `tz.rs` | Tipos do IPC, erro `{ code, message }` e fuso local. |
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

- **Wayland:** no Linux o app desliga o renderizador DMABUF do WebKitGTK na inicialização, porque
  ele derruba a janela em alguns compositores (`Error 71 dispatching to Wayland display`). Para
  religar: `WEBKIT_DISABLE_DMABUF_RENDERER=0`.
- **yt-dlp:** precisa estar no `PATH`. Sem ele, busca e download devolvem erro `unavailable`.
- **Sem dispositivo de áudio** o app abre normalmente; o erro só aparece ao tentar tocar.

## Android (pendente)

1. Instalar Android SDK, NDK e os targets Rust (`rustup target add aarch64-linux-android ...`).
2. `npx tauri android init` e `npx tauri android dev`.
3. Trocar o `NullBackend` por um plugin Kotlin com Media3 (`MediaSessionService`).
4. Definir o extrator do YouTube: o `yt-dlp` não roda como processo no Android.
