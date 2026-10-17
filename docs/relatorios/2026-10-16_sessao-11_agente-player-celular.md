# Player no celular (RF-46)

- **Data:** 2026-10-16 (implementação) e 2026-10-17 (verificação e relatório)
- **Autor:** Codex `gpt-6-sol` medium na implementação; relatório e ajustes finais pela orquestração (sessão 12), porque o Codex terminou sem escrevê-lo
- **Modelo:** Codex `gpt-6-sol` (medium); Claude Opus 5.5 na orquestração
- **Escopo:** abas embaixo, mini player com deslizar e Tocando em tela cheia com arrastar para fechar, abaixo de 768 px. O desktop não muda.

## O que foi feito

- **RF-46** registrado em `docs/requisitos/requisitos.md` (Should).
- `src/lib/util/drag.ts`: sessão de arrasto pura (`beginDrag`) com distância, velocidade, limiar por fração do tamanho, gesto rápido (≥ 0,65 px/ms nos últimos 80 ms) e cancelamento. Testado em `drag.test.ts`.
- Abas embaixo: Início, Busca, Discoteca, Playlists e Mais. "Buscar" virou "Busca", na segunda posição, com rótulo acessível "Busca no YouTube"; a `nav` passou a se chamar "Abas principais".
- Tocando no celular: a rota `now-playing` abre uma sobreposição `fixed inset-0` (`role="dialog"`, `aria-modal`) que cobre abas e mini player, sobe de baixo com `springTransition` (altura da janela, 420 ms; zero com movimento reduzido) e deixa o resto da interface `inert`. Por baixo continua a tela anterior (`lastRoute`), então fechar não recarrega nada. Escape, o botão "Recolher Tocando" e o arrasto fecham com `router.back()` (ou o Início, sem histórico), e o foco volta ao mini player.
- Arrastar para fechar: a alça do topo e a capa (`data-drag-handle`, `touch-none`) aceitam o arrasto com Pointer Events; fecha a partir de 30 % da altura ou com gesto rápido, senão volta na mola. Botões não iniciam arrasto.
- Coluna no celular: capa, dados, seek e transporte, depois Fila/Letra (`order-*`). Chips técnicos, equalizador e volume ficam só a partir de `md`.
- Mini player: deslizar para a esquerda passa para a próxima faixa e para a direita volta, a partir de 35 % da largura ou com gesto rápido; o toque simples abre o Tocando. Alvo com `min-h-12` e `touch-pan-y` (a rolagem vertical segue livre).
- `springTransition` ganhou `reducedDuration` (padrão 90 ms, como antes).
- Orquestração: corrigido o aviso `state_referenced_locally` do `svelte-check` (valor inicial de `lastRoute`) e o `transform:undefined` literal do estilo da sobreposição.

## Decisões

- Tocando continua sendo uma rota (o histórico do RF-43 e o "voltar" valem); só a forma de mostrar muda no celular. Motivo: menos estado novo.
- Fila em folha de baixo e deslizar nas linhas de faixa ficaram para RF-47 e RF-48, como no briefing.

## Verificação

Rodado pela orquestração no worktree em 2026-10-17:

| Comando | Resultado |
|---|---|
| `npm run check` | 419 arquivos, 0 erros, 0 avisos (antes da correção: 1 aviso) |
| `npm test` | 24 arquivos, 107 testes passando (105 antes das correções da revisão) |
| `npm run build` | ok (`index` JS 237 kB, 75,9 kB gzip) |
| `node scripts/rastreabilidade.mjs --check` | ok: 57 requisitos, nenhum ID desconhecido |

Cargo não rodado: não há Rust no diff. Não rodados: `revisar-playwright` em 390×844 e o celular real (o APK ainda não existe).

## Pendências e problemas conhecidos

- Gestos não provados com dedo de verdade: precisam do APK (`feat/android-base`) e do inspetor remoto.
- Revisão do #86 (corrigido): "← Voltar" no Tocando do desktop, rolagem preservada ao abrir e fechar a sobreposição, mínimo de 24 px no gesto rápido, toque depois de um deslize curto, `touch-none` só no celular e botões de 48 px no mini player.
- Revisão do #86 (fica para RF-47/RF-48): no celular, a ordem de foco põe a fila antes dos controles; favorito e fila em Tocando têm 36 px; o foco não volta ao mini player quando o Tocando fecha pelo voltar do sistema; não há "Anterior" no mini player (o deslizar para a direita não tem botão equivalente; decisão do autor).
- `vite.worktree.config.ts` foi só para o cache do Vite no sandbox e não entra no commit.

## Fatias de commit

1. `docs(docs): registra o rf-46 do player no celular` — `docs/requisitos/requisitos.md`
2. `feat(ui): adiciona a sessão de arrasto para gestos de toque` — `src/lib/util/drag.ts`, `drag.test.ts`
3. `feat(ui): põe a busca nas abas de baixo do celular` — `src/lib/components/Nav.svelte`
4. `feat(ui): abre o tocando em tela cheia no celular com arrastar para fechar` — `src/App.svelte`, `src/routes/NowPlaying.svelte`, `src/lib/motion.ts`
5. `feat(ui): troca de faixa ao deslizar o mini player` — `src/lib/components/MiniPlayer.svelte`
6. `docs(docs): registra o relatório e a arquitetura do player no celular` — `docs/arquitetura/arquitetura.json`, este relatório
