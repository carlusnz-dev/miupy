# Movimento: mola suave, entrada padrão, texto animado, serifa e mesh leve

- **Data:** 2026-10-15
- **Autor:** agente `feat/movimento` (Sonnet, Orca)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** base de movimento e tipografia para as próximas frentes (RNF-04, RNF-05, RNF-08, RNF-09, RNF-13).

## O que foi feito

- Tokens em `:root` (`src/app.css`): `--ease-spring` (`linear()` de 25 pontos, um overshoot de ~0,9%),
  `--dur-fast` 180 ms, `--dur-base` 360 ms, `--dur-slow` 420 ms, `--enter-y` 8 px.
- Mola suave: `spring-in` agora é Y 6 px + escala .98 → 1 sem overshoot de escala; `spring-bounce` mais
  discreto (.94); hover, modal e `motion-bounce` usam os tokens. `springTransition`/`springCollapse` usam a
  função `spring(t)` (`src/lib/motion.ts`), a mesma curva do token.
- Entrada padrão `.enter` (fade + Y, `--enter-delay`). `.motion-item`, `.motion-player`, `.motion-page` e
  `.motion-toast` foram removidos (page e toast não tinham uso); `TrackRow`, `Recommendations` e `PlayerBar`
  usam `.enter`.
- `AnimatedText.svelte` + `util/animatedText.ts` (divisão em palavras/letras e atrasos, testados); usado no
  título da `PageHeader`.
- Source Serif 4 (`opsz.css` importado em `main.ts`): `h1`/`h2` e classe `font-display` nos números grandes
  (Home, Dna, Stats, Wrapped). RNF-04 atualizado no requisitos.
- `MeshGradient` **estático** (corrige o item 5 do briefing, por recado da orquestração): 3 camadas fixas, sem
  animação infinita, sem `will-change`, sem pausa (o `animation:` inline vencia `.mesh.paused`, então a pausa
  nunca funcionou). Medido no app real pelo inspetor do WebKit, com as 4 animações: janela ~37% e
  WebKitWebProcess ~40% de CPU; pausadas: 4% e 2%. Só as cores transicionam (`--mesh-1..3` com `@property`,
  `--dur-slow`) quando a lava muda; é a base para a tela Tocando.

## API para as próximas frentes

- **Entrada:** `class="enter"`; em listas `style={enterStyle(i)}` (`enterStyle(i, step=20, limit=14)`, só as 14
  primeiras atrasam). Atraso solto: `style="--enter-delay:120ms"`.
- **Texto:** `<AnimatedText text={title} class="..." />`; reanima ao mudar `text`. Envolver em `h1`/`h2`.
- **Transição Svelte:** `transition:springTransition` (padrão y 8 px, 360 ms) e `springCollapse`.
- **Tokens:** `var(--ease-spring)`, `var(--dur-*)`, `var(--enter-y)`; serifa: `font-display` ou `var(--font-display)`.

## Decisões

- `AnimatedText` usa um `<span class="sr-only">` com o texto e esconde as letras com `aria-hidden`, em vez de
  `aria-label`: `aria-label` em `<span>` sem papel não é anunciado de forma confiável.
- O Modal e o `Toasts.svelte` não foram alterados além dos tokens no CSS; o toast mantém a duração explícita.
- Estatísticas Wrapped também ganharam serifa (números grandes).

## Verificação

- `npm run check`: 0 erros, 0 avisos. `npm test`: 20 arquivos, 89 testes passaram. `npm run build`: ok.
- `node scripts/rastreabilidade.mjs --check`: ok, 50 requisitos.
- Cargo (fmt/clippy/test): **não rodado** — o worktree não tem `target/` e só mexi no frontend; o CI roda.
- Playwright (`revisar-playwright`): **não rodado** — o MCP não estava disponível nesta sessão; revisão de
  design feita só lendo o diff. Não provado: aparência da serifa, sensação da mola e a transição de cor do mesh
  (precisam do app real).

## Pendências e problemas conhecidos

- Conferir no app se o stutter sumiu com o mesh estático (a transição de cor via `@property` não foi vista).
- `truncate` no `h1` com `AnimatedText`: palavras são `inline-block`; título longo corta por palavra.
- Toast ainda usa `springTransition` com duração própria (220 ms).
