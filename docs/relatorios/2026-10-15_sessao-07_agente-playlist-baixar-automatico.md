# Download automático de playlists (RF-31)

- **Data:** 2026-10-15
- **Autor:** agente playlist-baixar-automatico
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** migração 0009, núcleo, fila da casca, IPC, mock e interface.

## O que foi feito

- Adicionada `playlists.auto_download` com padrão 0 e restrição 0/1. O modelo, patch, consultas de faixas elegíveis e busca de playlists automáticas foram implementados no núcleo.
- A casca enfileira faixas elegíveis ao ligar a opção e ao adicionar faixas por playlist, YouTube ou favoritas. Cada operação emite um resumo `download://auto`. Desligar preserva jobs já enfileirados.
- Falhas de rede em jobs de playlists automáticas recebem um marcador persistente `auto_offline:`. A fila existente retenta esses jobs na abertura e a cada cinco minutos. Um evento `offline` resume os jobs pendentes; falhas de outros tipos continuam como `error`. A limpeza de concluídos preserva os jobs aguardando rede.
- A tela de playlist oferece a opção, a tela de downloads mostra os jobs como “Aguardando rede”, e a store exibe toasts curtos. Tipos IPC e mock foram atualizados.

## Decisões

- A migração 0008 ainda não existe neste worktree. `db.rs` registra o arquivo **0009** como oitavo item temporário da lista contígua; por isso, neste checkout isolado, o `user_version` final é **8**. Na integração, inserir a migração 0008 antes da 0009 na lista e atualizar as asserções de versão para **9**, antes de publicar ou usar esse banco com dados reais. O teste `sql_0009_funciona_sobre_banco_na_versao_8` executa o SQL da 0009 sobre um banco marcado como versão 8; a migração completa de um banco real v8 pelo registrador só poderá ser testada após integrar a 0008. Não foi criado um arquivo 0008 falso.
- O marcador persistente no campo `download_jobs.error` permite retentar após fechar o app sem outra fila ou migração. A interface esconde o marcador e exibe “Aguardando rede”.
- O mock simula enfileiramento e eventos; a falha de rede foi testada na casca com um resolvedor injetado, não contra a rede real.

## Verificação (comandos rodados e resultado real)

- `nice -n 10 env CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 cargo fmt --all`: passou.
- `nice -n 10 env CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 cargo clippy --workspace --all-targets -- -D warnings`: passou após corrigir duas sugestões de `cloned_ref_to_slice_refs` nos testes.
- `nice -n 10 env CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 cargo test --workspace --quiet`: passou. Casca: 108 passaram, 2 ignorados; áudio: 35 passaram, 1 ignorado; núcleo: 94 passaram; mídia: 72 passaram; testes de rede real: 11 ignorados; doc test: 1 passou.
- `npm run check`: 0 erros e 0 avisos. A primeira tentativa falhou por `EROFS` porque Vite tentou gravar no `node_modules` compartilhado e somente leitura. Para verificar, usei temporariamente links locais dos pacotes com `.vite-temp` gravável e restaurei o link original ao fim.
- `npm test`: 14 arquivos, 55 testes passaram.
- `npm run build`: passou, 197 módulos transformados.
- `node scripts/rastreabilidade.mjs --check`: passou, 50 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- Integrar a migração 0008 antes da 0009 e ajustar os testes de `schema_version` para 9. Até lá, este branch não deve ser usado para abrir um banco real já na versão 8.
- Não foi feita prova contra a rede real; os testes injetam `MediaError::Network` no resolvedor.
- Não houve commit nem alteração em outro checkout, conforme o sandbox desta frente.

## Próximos passos

- Integrar esta árvore à `develop` após a frente da 0008, revisar o merge nos arquivos compartilhados e executar novamente a bateria de CI.

## Fatias de commit

Cada mensagem abaixo usa o rodapé `Refs: RF-31`.

1. `feat(db): adicione opção de download automático às playlists` — `crates/miupy-core/migrations/0009_auto_download.sql`; `crates/miupy-core/src/db.rs` (registro da 0009 e asserções de versão). Rodapé: `Refs: RF-31`.
2. `feat(core): persista opção e consulte faixas pendentes` — `crates/miupy-core/src/model.rs`; `crates/miupy-core/src/playlists.rs` (modelo SQL, patch e três consultas, sem o bloco de testes). Rodapé: `Refs: RF-31`.
3. `test(core): cubra migração e consultas automáticas` — `crates/miupy-core/src/db.rs` (dois testes RF-31); `crates/miupy-core/src/playlists.rs` (teste RF-31). Rodapé: `Refs: RF-31`.
4. `feat(shell): enfileire faixas novas de playlists automáticas` — `src-tauri/src/auto_download.rs` (função de produção); `src-tauri/src/commands/playlist.rs`; `src-tauri/src/commands/library.rs`; `src-tauri/src/commands/youtube.rs`; `src-tauri/src/events.rs`; `src-tauri/src/lib.rs`. Rodapé: `Refs: RF-31`.
5. `feat(shell): retente downloads automáticos após falha de rede` — `crates/miupy-core/src/downloads.rs` (consultas offline, proteção da limpeza); `src-tauri/src/downloads.rs` (campos, classificação, evento e nova tentativa, sem os blocos de testes). Rodapé: `Refs: RF-31`.
6. `test(shell): cubra fila automática e recuperação offline` — `src-tauri/src/auto_download.rs` (módulo de testes); `src-tauri/src/downloads.rs` (ajuste do resolvedor falso e teste RF-31); `crates/miupy-core/src/downloads.rs` (teste RF-31). Rodapé: `Refs: RF-31`.
7. `feat(ipc): exponha opção e eventos no contrato de interface` — `src/lib/ipc/types.ts`; `src/lib/ipc/api.ts`; `src/lib/ipc/mock.ts`. Rodapé: `Refs: RF-31`.
8. `feat(ui): mostre opção e avisos de download automático` — `src/routes/PlaylistDetail.svelte`; `src/routes/Downloads.svelte`; `src/lib/stores/downloads.svelte.ts`. Rodapé: `Refs: RF-31`.
9. `test(ui): cubra opção automática no mock` — `src/lib/ipc/mock.test.ts`; `src/lib/stores/stores.test.ts` (campo obrigatório do fixture). Rodapé: `Refs: RF-31`.
10. `docs(docs): registre migração e resultado da sessão` — `docs/arquitetura/banco-de-dados.md`; `docs/arquitetura/arquitetura.json`; `docs/relatorios/2026-10-15_sessao-07_agente-playlist-baixar-automatico.md`. Rodapé: `Refs: RF-31`.

## Correções da revisão

- **M1/B1:** o timer continua tentando a cada cinco minutos, mas não limpa `offline_notified` ao reenfileirar e não emite `retry`. Esse evento sai apenas na tentativa da abertura. O aviso `offline` só volta a ficar disponível depois de um download bem-sucedido ou quando não restam pendências nem jobs em andamento naquela playlist. Assim, o segundo ciclo sem rede não gera outro toast e uma queda futura pode ser avisada.
- **M2:** jobs `auto_offline:` sem vínculo com qualquer playlist automática viram erros comuns no retry e antes de “Limpar concluídos”, inclusive após desligar a opção, remover a faixa ou apagar a playlist. A tela oferece cancelamento em “Aguardando rede”; o mock também remove o job cancelado.
- **B2:** o contrato IPC e a descrição de `download_jobs` documentam `status: 'error'`, o prefixo `auto_offline:` e o texto “Aguardando rede”. O frontend usa `isWaitingNetwork(job)` na tela, na store e no mock.
- **B7:** testes cobrem repetição da ativação e de faixa já presente sem novo job/evento, segundo ciclo sem rede sem aviso, conversão e limpeza de job órfão e novo aviso após a playlist perder as pendências anteriores.
- **B8:** o checkbox muda imediatamente, sincroniza com a playlist atualizada e restaura o valor anterior com toast de erro se `playlistUpdate` falhar.
- A migração 0008 já está integrada antes da 0009 neste worktree e o esquema está na versão 9; as ressalvas históricas acima descrevem o checkout anterior à integração.

### Verificação das correções (resultado real)

- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou, sem avisos.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace`: passou; casca 113, núcleo 99, áudio 35 e mídia 72 testes aprovados; 14 ignorados conforme os testes de rede/dispositivo; 1 doc test aprovado. Uma execução anterior falhou porque o fixture de limpeza não tinha playlist automática; o fixture foi corrigido e a bateria completa passou.
- `npm run check`: passou, 0 erros e 0 avisos.
- `npm test`: passou, 14 arquivos e 61 testes.
- `npm run build`: passou, 200 módulos transformados.
- `node scripts/rastreabilidade.mjs --check`: passou, 50 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.
- O primeiro `npm run check` e o primeiro `npm test` encontraram `EROFS` no cache Vite do `node_modules` compartilhado. As execuções válidas usaram links locais temporários com `.vite` e `.vite-temp` graváveis; o link original de `node_modules` foi restaurado.

## Fatias de commit (correções)

Cada mensagem abaixo usa o rodapé `Refs: RF-31`. São fatias sugeridas; nenhuma foi commitada.

1. `fix(shell): evita avisos repetidos sem rede` — `src-tauri/src/downloads.rs` (fluxo de produção).
2. `fix(shell): libera downloads automáticos órfãos` — `crates/miupy-core/src/downloads.rs` (consultas e limpeza).
3. `fix(ui): permite cancelar espera de rede e restaura opção após falha` — `src/routes/Downloads.svelte`, `src/routes/PlaylistDetail.svelte`, `src/lib/stores/downloads.svelte.ts`, `src/lib/ipc/mock.ts`, `src/lib/ipc/download.ts`.
4. `test(shell): cobre retry silencioso e enfileiramento repetido` — `src-tauri/src/downloads.rs`, `src-tauri/src/auto_download.rs`, `src-tauri/src/commands/playlist.rs`, `crates/miupy-core/src/downloads.rs` (blocos de testes e função de seleção de faixas novas).
5. `docs(ipc): documenta espera pela rede nos downloads` — `docs/arquitetura/ipc.md`, `docs/arquitetura/banco-de-dados.md`, `docs/relatorios/2026-10-15_sessao-07_agente-playlist-baixar-automatico.md`.
