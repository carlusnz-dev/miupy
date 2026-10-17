# Controle da fila por IPC

- **Data:** 2026-10-17
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RF-49 no Rust, contrato e clientes IPC, sem interface.

## O que foi feito

- Registrei o RF-49 nos requisitos.
- Acrescentei à `PlayQueue` inserção, remoção e movimento por posição, preservando a ocorrência atual e a ordem original usada pelo aleatório.
- Liguei as operações ao `Player` e a três comandos Tauri. `enqueue` valida todos os IDs antes de alterar a fila; cada operação publica `player://state` e devolve `PlayerState`.
- Expus os comandos nos clientes IPC real e mock e documentei suas regras no contrato.
- Testei fila pura, Player, cliente Tauri e mock, incluindo duplicatas, índices inválidos, fila vazia e retorno do aleatório para `off`.

## Decisões

- Enfileirar numa fila vazia deixa `queueIndex = -1` e não toca. `player_next` e `player_resume` iniciam a primeira faixa depois disso.
- A posição de uma ocorrência repetida em `original_ids` é associada pelo ordinal do mesmo ID na fila visível. Mover com aleatório ligado altera só a fila visível; com `off`, altera as duas ordens.
- Os comandos preservam o `context`. Remover a posição atual ou usar índice inválido devolve `invalid_input`; ID ausente no `enqueue` devolve `not_found` sem alteração parcial.
- `docs/arquitetura/arquitetura.json` não mudou: nenhum módulo, dependência ou relação estrutural foi acrescentado.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou sem avisos.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test --workspace`: 374 passaram, 16 ignorados, 0 falhas. A primeira execução falhou por uma expectativa errada em um teste novo; corrigi a expectativa e repeti a suíte completa.
- `npm run check -- --config /tmp/fila-ipc-svelte.config.mjs`: 0 erros e 0 avisos. A primeira tentativa encontrou EROFS no cache do Vite; a cópia temporária do config desativou só o pré-processamento de estilos nessa checagem.
- `npm test -- --configLoader runner --config vite.worktree.config.ts`: 24 arquivos e 110 testes passaram.
- `npm run build -- --configLoader runner --config vite.worktree.config.ts`: passou, 217 módulos transformados. `vite.worktree.config.ts` foi removido após as verificações.
- `node scripts/rastreabilidade.mjs --check`: 60 requisitos, nenhum ID desconhecido.

## Pendências e problemas conhecidos

- Sem teste manual no desktop ou Android nesta frente; os testes Rust e IPC cobrem o contrato, mas não a interface da outra frente.
- Nenhum commit foi criado por restrição do sandbox. `npm run docs:matriz` não foi executado, conforme o fluxo do projeto.

## Fatias de commit

1. `docs(docs): registra o rf-49 de controle da fila` — `docs/requisitos/requisitos.md`.
2. `feat(core): adiciona enqueue, remove e move à fila de reprodução` — `src-tauri/src/queue.rs`.
3. `feat(shell): liga os comandos de controle da fila ao player` — `src-tauri/src/player.rs`, `src-tauri/src/commands/player.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/dto.rs`.
4. `feat(ipc): expõe player_enqueue, player_queue_remove e player_queue_move` — `src/lib/ipc/api.ts`, `src/lib/ipc/tauri.ts`, `src/lib/ipc/mock.ts`, `src/lib/ipc/mock.test.ts`, `src/lib/ipc/tauri.test.ts`.
5. `docs(docs): documenta o controle da fila no contrato` — `docs/arquitetura/ipc.md`.
6. `docs(docs): registra a frente de controle da fila` — este relatório.

## Próximos passos

- A frente de interface pode usar os três métodos de `MiupyApi` para botões e gestos.
