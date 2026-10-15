# Menu recolhível e player flutuante

- **Data:** 2026-10-15
- **Autor:** agente `feat/menu-e-player` (Sonnet, Orca)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** RF-38 (novo), RF-33, RF-18, RNF-03, RNF-04, RNF-08. Só frontend.

## O que foi feito

- **Estado recolhido:** `settings.sidebarCollapsed` (`settings.svelte.ts`), lido do `localStorage`
  (`miupy.sidebarCollapsed`) na criação da store, sem flash; `parseCollapsed` em `util/sidebar.ts`.
- **Trilho de ícones:** `Nav.svelte` ganha `.nav-shell` (largura `--sidebar-w` → `--rail-w` 4,5 rem, com
  `--ease-spring` e `--dur-base`). Recolhido, os links viram só ícone com `aria-label` e `title`; rótulos,
  "miupy", título "Discoteca" e o nome do perfil entram com `.enter` (fade + Y) ao expandir. O contador de downloads
  vira um ponto. A alça de redimensionar some enquanto recolhido.
- **Botão e atalho:** chevron na borda (`.nav-toggle`, área de clique de 32 px por pseudo-elemento,
  `aria-expanded`, `aria-label`) e **Ctrl+B** (só na variante lateral).
- **Player flutuante:** `PlayerBar` vira bloco com margem, `rounded-2xl`, `--player-bg` (mais escuro que
  `--bg`) e `backdrop-filter: blur(10px)` com recuo sólido (`.player-float`). Fica `absolute` sobre a coluna do
  conteúdo; `main` ganha `padding-bottom` para a última linha não ficar escondida. Em Tocando continua
  escondido; entra/sai com `springTransition`. Mobile (< 768 px) inalterado.
- **Grade:** `App.svelte` usa `grid-cols-[auto_minmax(0,1fr)_var(--aside-w)]`; `--aside-w: 0px` é a
  terceira coluna reservada, sem elemento.
- **Docs:** RF-38 em `requisitos.md`; exceção do player no RNF-04.

## Decisões

- Estado recolhido **só no `localStorage`**, não em `settings_update`: guardar no backend exigiria campo novo em
  Rust/`ipc.md` e esta frente é só frontend. Se o autor quiser no banco, é uma frente pequena de contrato.
- `springCollapse` deixou de ser usado no app (só tem teste); não removi para não ampliar o diff.
- Tokens novos (`--rail-w`, `--aside-w`, `--player-bg`) no fim do `:root`.

## Verificação

- `npm run check`: 0 erros, 0 avisos. `npm test`: 20 arquivos, 89 testes passaram (inclui novos `@test RF-38`).
- `npm run build`: ok. `node scripts/rastreabilidade.mjs --check`: ok, 51 requisitos.
- Cargo (fmt/clippy/test): **não rodado**, sem `target/` e nenhum Rust alterado; o CI roda.
- Revisão do PR: aprovada; ajustados Ctrl+B ignorado ao digitar, nome acessível do contador no trilho e flush da largura ao recolher.
- Playwright (`revisar-playwright`): **não rodado**, o MCP não está disponível nesta sessão (não há Chromium).
  `revisar-design` foi feita só lendo o diff.

## Pendências e problemas conhecidos

- **Medição do blur NÃO feita.** Sem navegador/MCP não consegui rodar `__miupyPerf` nem rAF com o player por
  cima. O blur está ligado como pedido (10 px). **Pergunta ao autor:** medir no app real em software
  (rolagem da Discoteca, `__miupyPerf.run()`); se o p95 passar de 16,7 ms, trocar para sólido (basta apagar o
  bloco `@supports` de `.player-float`) e a exceção do RNF-04 cai. Decisão da exceção é do autor.
- Não provado no app: aparência do trilho, animação da largura (anima `width`, que refaz o layout do conteúdo a
  cada quadro; se engasgar, trocar por salto sem animação), posição do chevron, alinhamento dos ícones no trilho.
- Toasts e aviso fixo calculam a base a partir de `--player-h`; com o player a 1 rem do fundo podem encostar
  nele em ~0,25 rem. Fora do escopo (toasts).
- Sem `will-change` e sem animar `filter`/`backdrop-filter`, como pedido.
