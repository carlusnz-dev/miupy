# Painel da direita

- **Data:** 2026-10-15
- **Autor:** agente `feat/painel-direito` (Sonnet, Orca)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** RF-39 (novo), RF-03, RNF-08. Só frontend.

## O que foi feito

- **Estado:** `settings.rightPanel` (`boolean | null`) no `localStorage` (`miupy.rightPanel`), no mesmo lugar do
  `sidebarCollapsed`. `null` = nunca escolhido, e vale o padrão pela largura (aberto a partir de 1280 px).
  Funções puras em `util/rightPanel.ts` (`parseRightPanel`, `rightPanelOpen`, `upcoming`).
- **Faixa atual:** capa, título, artista com link e botão de favoritar, com `.enter`.
- **Fila:** a faixa atual e as próximas (`player.queueTracks` a partir de `queueIndex`), atual destacada, clique
  toca com `player.play(id, queue)`. Usa `Chunked` e `enterStyle(i)`, como as outras listas.
- **Botão e atalho:** chevron na borda (`.right-toggle`, `aria-expanded`, `aria-label`) e **Ctrl+J**
  (o Ctrl+B é do menu). Ignorado ao digitar.
- **Layout:** `App.svelte` ganha a terceira coluna `auto`. A partir de `xl` (1280 px) o painel ocupa a coluna;
  abaixo disso fica `absolute` à direita, por cima do conteúdo. Largura anima com `--ease-spring`; o miolo tem
  largura fixa, só o casco anima. Fundo `bg-s1` sólido, sem `backdrop-filter`, sem animação contínua.
- **Esconder:** não é renderizado em Tocando; `hidden md:block` o esconde abaixo de 768 px.
- **Docs:** RF-39 em `requisitos.md`. Token `--aside-w` virou `--right-w` (20 rem).

## Decisões

- Nome do contexto ("Tocando de: …") **não feito**: `feat/fila-contexto` ainda não está na develop e
  `PlayerState.context` não existe. É um bloco pequeno no cabeçalho da fila quando entrar.
- Estado só no `localStorage`, como o do menu (sem campo novo no contrato).
- A fila mostra da faixa atual em diante, não as já tocadas; o histórico fica na tela Tocando.
- Ctrl+J escolhido por não colidir com o Ctrl+B.

## Verificação

- `npm run check`: 0 erros, 0 avisos. `npm test`: 22 arquivos, 96 testes passaram (novos `@test RF-39`).
- `npm run build`: ok. `node scripts/rastreabilidade.mjs --check`: ok, 53 requisitos.
- Cargo (fmt/clippy/test): **não rodado**, sem `target/` e nenhum Rust alterado; o CI roda.
- Playwright (`revisar-playwright`): **não rodado**, sem MCP nem Chromium nesta sessão. `revisar-design` feita só
  lendo o diff.

## Pendências e problemas conhecidos

- **Não provado no navegador/app:** aparência, animação da largura (anima `width`, que refaz o layout do conteúdo;
  se engasgar, trocar por salto), posição do botão com o painel recolhido (fica a 2 rem da borda, sobre o canto
  do conteúdo) e a sobreposição abaixo de 1280 px (o painel cobre parte do player flutuante).
- Lista longa: o espaçador do `Chunked` usa a altura de 56 px por linha; a linha daqui tem ~52 px, então a barra de
  rolagem pode variar um pouco.
