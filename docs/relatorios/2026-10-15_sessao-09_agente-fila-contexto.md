# Contexto da fila de reprodução

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RF-03 e base de RF-42 no Rust, IPC e cliente frontend.

## O que foi feito

- Adicionado `QueueContext` ao `PlayerState` e o argumento opcional `context` a `player_play`.
- O player associa o contexto à fila nova, conserva-o ao navegar, buscar posição e trocar o modo aleatório, e limpa-o ao parar, esvaziar a fila ou iniciar fila unitária fora da atual.
- Renomear a playlist do contexto atual atualiza o `label`; apagá-la limpa o contexto. Ambos emitem `player://state`.
- O cliente Tauri, mock e store aceitam o contexto; `isPlayingContext(kind, id)` compara a origem com a reprodução ativa.
- As filas iniciadas nas telas de playlist, artista e discoteca enviam a origem. O requisito RF-42 foi registrado; nenhuma interface visual foi alterada.

## Decisões

- O contexto fica só em memória, como a fila. Sem migração; `restore_prefs` continua restaurando apenas preferências.
- A casca atualiza o nome no comando `playlist_update` e limpa no `playlist_delete`: evita uma consulta ao banco em cada `snapshot` e mantém o evento do player imediato.
- `player_play` sem `queue` mantém o contexto apenas se a faixa já pertence à fila; outra faixa cria fila unitária sem origem.
- `docs/arquitetura/arquitetura.json` permanece igual porque nenhum módulo ou relação estrutural mudou.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace --quiet`: passou; 125 + 35 + 103 + 84 testes unitários e 1 doctest, 15 ignorados (rede ou dispositivo).
- `npm test`: 21 arquivos, 94 testes passaram.
- `npm run check`: falhou por `EROFS` ao gravar `node_modules/.vite-temp` no link somente leitura do worktree. `npm run check -- --config /tmp/miupy-svelte-check.config.mjs` passou com 0 erros e 0 avisos; a configuração temporária omite o pré-processamento de CSS.
- `npm run build -- --configLoader runner`: passou, 210 módulos transformados; o carregador evita a mesma escrita em `node_modules`.
- `node scripts/rastreabilidade.mjs --check`: passou, 52 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- O visual do cartão no Início e do botão Pausar na playlist pertence à próxima frente.
- A verificação de estilo do `svelte-check` padrão não foi comprovada neste worktree; o build com os plugins normais passou.
- O app real e o DOM não foram exercitados nesta frente, que não alterou marcação nem estilos.

## Fatias de commit

1. `feat(ipc): adiciona contexto ao estado da fila` — `src-tauri/src/dto.rs`, `docs/arquitetura/ipc.md`. Rodapé: `Refs: RF-03 RF-42`.
2. `feat(shell): mantém contexto durante a reprodução` — `src-tauri/src/player.rs`. Rodapé: `Refs: RF-03 RF-42`.
3. `feat(ipc): propaga contexto no comando e nas playlists` — `src-tauri/src/commands/player.rs`, `src-tauri/src/commands/playlist.rs`. Rodapé: `Refs: RF-03 RF-42`.
4. `feat(ui): acompanha contexto no cliente do player` — `src/lib/ipc/types.ts`, `src/lib/ipc/api.ts`, `src/lib/ipc/tauri.ts`, `src/lib/ipc/mock.ts`, `src/lib/ipc/mock.test.ts`, `src/lib/stores/player.svelte.ts`, `src/lib/stores/stores.test.ts`. Rodapé: `Refs: RF-03 RF-42`.
5. `feat(ui): informa origem nas filas das telas` — `src/routes/PlaylistDetail.svelte`, `src/routes/Artist.svelte`, `src/routes/Library.svelte`. Rodapé: `Refs: RF-42`.
6. `docs(docs): registra requisito de playlist tocando` — `docs/requisitos/requisitos.md`. Rodapé: `Refs: RF-42`.
7. `docs(docs): relata frente de contexto da fila` — `docs/relatorios/2026-10-15_sessao-09_agente-fila-contexto.md`. Rodapé: `Refs: RF-03 RF-42`.

## Próximos passos

- Implementar o cartão da playlist tocando no Início e a troca Tocar/Pausar na tela da playlist usando `PlayerState.context` e `isPlayingContext`.
