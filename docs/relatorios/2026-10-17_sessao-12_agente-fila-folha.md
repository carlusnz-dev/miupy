# Fila em folha de baixo no Tocando

- **Data:** 2026-10-17
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RF-47 na interface móvel, sem alterações em Rust ou IPC.

## O que foi feito

- Registrei RF-47 como Should e atualizei o papel do frontend e a contagem de testes na arquitetura.
- Criei `BottomSheet` reutilizável com alça e título, abertura por toque ou arrasto, recolhimento por arrasto, botão ou Escape, limite de 85dvh, área segura e conteúdo rolável. O gesto usa `beginDrag` e a curva de mola de `motion.ts`; movimento reduzido desliga a transição.
- Coloquei fila e letra na folha somente no celular. O desktop conserva a área de fila. No celular, o DOM segue capa, dados, seek, transporte e folha; controles e abas da folha têm alvos de 48 px.
- O botão de fila abre a folha. O foco volta ao mini player também quando o histórico ou o voltar do sistema sai de Tocando.
- O mini player ganhou Anterior, com os três controles de transporte de 48 px e título truncado.
- Acrescentei um teste dos limiares de abertura e recolhimento da folha.

## Decisões

- O arrasto começa somente no cabeçalho da folha; a lista usa sua própria rolagem.
- Fila e letra compartilham o mesmo trecho de marcação entre os layouts, sem mudar o contrato IPC.
- Mantive a folha montada e o conteúdo fechado `inert`, para evitar foco em controles fora de vista.

## Verificação

- `npm run check`: passou, 0 erros e 0 avisos de diagnóstico. O carregamento inicial de `vite.config.ts` ainda imprimiu um aviso de `EROFS`; o pré-processamento usou configuração temporária gravável.
- `nice -n 10 npm test -- --config vite.worktree.config.ts --configLoader runner`: 24 arquivos, 108 testes passaram.
- `npm run build -- --config vite.worktree.config.ts --configLoader runner`: passou, 217 módulos transformados.
- `node scripts/rastreabilidade.mjs --check`: passou, 58 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.
- Revisão `revisar-design` pelo diff: sem achados pendentes no código; tokens magma, foco visível, ordem de foco, alvos e movimento reduzido conferidos.
- `revisar-playwright`: não rodado; navegador Playwright indisponível neste sandbox. Arrasto real, rolagem e restauração de foco no WebView ainda precisam de verificação no app.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`: não rodados, conforme o briefing; não há Rust no diff nem `target/` neste worktree.

Correções da orquestração (2026-10-17), conferidas com `npm run check` (0 erros), `npm test`
(108), build e rastreabilidade fora do sandbox:

- A sobreposição do Tocando tinha `transform` sempre ativo (resto de uma correção do #86). Com ele,
  a folha `position: fixed` passaria a ser relativa à sobreposição, que rola, e subiria junto com o
  conteúdo. Agora o `transform` só existe durante o arrasto.
- "Mostrar a fila" voltou a ter `aria-pressed` no desktop (o `aria-expanded` vale só no celular).

## Pendências e problemas conhecidos

- A interação por toque e o layout em aparelhos pequenos não foram conferidos visualmente no WebView.

## Fatias de commit

1. `docs(docs): registra a fila em folha de baixo` — `docs/requisitos/requisitos.md`. Rodapé: `Refs: RF-47`.
2. `feat(ui): adiciona folha inferior reutilizável` — `src/lib/components/BottomSheet.svelte`, `src/lib/util/drag.test.ts`. Rodapé: `Refs: RF-47`.
3. `feat(ui): move fila e letra para a folha no celular` — `src/routes/NowPlaying.svelte` (extração do painel e montagem da folha). Rodapé: `Refs: RF-47`.
4. `fix(ui): alinha foco e amplia alvos no tocando` — `src/routes/NowPlaying.svelte` (ordem do DOM e controles). Rodapé: `Refs: RF-47`.
5. `fix(ui): restaura foco ao sair do tocando` — `src/App.svelte`. Rodapé: `Refs: RF-47`.
6. `feat(ui): adiciona anterior ao mini player` — `src/lib/components/MiniPlayer.svelte`. Rodapé: `Refs: RF-47`.
7. `docs(docs): atualiza arquitetura e relatório da folha` — `docs/arquitetura/arquitetura.json`, `docs/relatorios/2026-10-17_sessao-12_agente-fila-folha.md`. Rodapé: `Refs: RF-47`.

## Próximos passos

- Conferir no WebView móvel a área segura, a rolagem da fila, os gestos e o foco após voltar; depois revisar as fatias para integração.
