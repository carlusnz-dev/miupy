# Vidro no desktop: tokens, cor da capa e superfícies flutuantes

- **Data:** 2026-10-17
- **Autor:** Claude (Sonnet 5.5), agente `vidro-base` no Orca, sessão 13
- **Modelo:** Sonnet 5.5
- **Escopo:** RNF-04 reescrito, classe `.glass`, cor da capa global (`--cover-tint`), player, menu, modais, menus, toasts e brilho ambiente do desktop. Só frontend e docs (sem cargo).

## O que foi feito

1. **RNF-04** agora é "Interface magma com vidro": junta as três exceções (toast, player, menu) numa regra só, com a lava como único destaque dos controles, a cor da capa tingindo fundo, player e Tocando, e o recuo sólido onde o `backdrop-filter` não é desenhado. Tipografia mantida.
2. **Tokens e classe de vidro** em `src/app.css`: `--glass-alpha`, `--glass-alpha-strong`, `--glass-tint-mix`, `--glass-blur` (12 px), `--glass-blur-strong` (24 px), `--glass-line`, `--glass-sheen`. `.glass` e `.glass-strong` trocam os três blocos `@supports` (`nav-glass`, `player-float`, `toast-glass`). O fundo já nasce quase sólido e o `@supports` só o torna translúcido (o recuo).
3. **Cor da capa global:** `src/lib/stores/cover.svelte.ts` calcula a paleta da faixa atual com `coverPalette` e publica `--cover-tint` no `:root`; sem capa a variável sai e vale a lava. O Tocando passou a ler `cover.colors` (a conta saiu de `NowPlaying.svelte`). `meshTriple` (em `palette.ts`, com teste) completa as três cores do mesh.
4. **Player e menu lateral:** `.glass`; o player ganhou a capa desfocada por baixo do vidro (`CoverGlow.svelte`).
5. **Modais, menus, folha e barras fixas:** `Modal`, `BottomSheet`, `Dropdown`, `Menu` (ações da playlist, vindo da `develop` no merge), menu "Mais" do `Nav`, sugestões do YouTube, menu do equalizador e `PinnedToast` em `.glass-strong`/`.glass`; as duas barras de seleção `sticky` em `.glass`. Toasts já no commit 2.
6. **Brilho ambiente:** `.ambient` atrás da página (só desktop e fora do Tocando): gradiente radial da cor da capa/lava mais a capa a 18% com desfoque de 64 px.
7. **Skills:** `desenhar-interface` ("Identidade") e `revisar-design` (item 1) falam do vidro e proíbem `backdrop-filter` solto e vidro em linha de lista.

## Decisões

- **`--tint` registrada sem herdar** (`@property`): a cor da capa transiciona só nos elementos de vidro e no ambiente, não na árvore inteira (transicionar `--cover-tint` no `:root` recalcularia o estilo de todas as linhas). `prefers-reduced-motion` já zera a transição pela regra global `*`.
- **Dois níveis de desfoque:** `.glass` (12 px, 62% de opacidade) para o que flutua sobre a lista; `.glass-strong` (24 px, 84%) para modal, folha e menu, que podem cair sobre texto ou sobre outro vidro.
- **Borda:** `.glass` define `border: 1px solid`; onde um lado não cabe (menu lateral, folha) a classe do componente usa `!border-y-0` etc., como o projeto já faz com `!border-0`.
- **Tintura baixa** (5% no fundo, 9% no brilho do topo): com 10% e 14% o menu ficou marrom-avermelhado demais no screenshot.
- **Recuo:** continua só pelo `@supports`, como antes. Com `MIUPY_GPU=off` o WebKitGTK aceita a propriedade e não a desenha: o vidro vira o fundo translúcido sem desfoque, ainda legível porque a opacidade mínima é 62%.
- Mini player e Tocando do celular, abas em pílula e o `RightPanel` ficam para a frente seguinte (o painel da direita não estava na lista de superfícies).

## Verificação

- `npm run check`: 423 arquivos, 0 erros, 0 avisos.
- `npm test`: 24 arquivos, 113 testes passaram (inclui `meshTriple`), depois do merge da `develop`.
- `npm run build`: ok.
- `node scripts/rastreabilidade.mjs --check`: ok, 60 requisitos, nenhum ID desconhecido.
- Cargo não rodado (frente só de frontend, sem `target/` no worktree).
- Visual: screenshot do Firefox headless (mock, 1400 px) do menu e da lista com o novo vidro; **não vi** o player com capa nem o Tocando (o mock não tem capa com imagem, então `CoverGlow` e o ambiente com a imagem não foram vistos), nem modais e menus abertos.

### Rolagem (RNF-13): inconclusiva

Medi no navegador (Firefox 157 headless, renderização por software, mock com 300 faixas, rolagem da Discoteca por 2 s com uma faixa tocando e o player por cima; é o passo 8 do roteiro, sem o `__miupyPerf`). Para a Discoteca rolar de verdade foi preciso injetar `grid-template-rows: minmax(0,1fr)` na medição: no Gecko, depois de tocar, a coluna central cresce com o conteúdo (já ocorre na `develop`, não é desta frente).

| Variante | p95 do quadro (cada valor é uma execução de 2 s) | Observação |
|---|---|---|
| sem vidro (`backdrop-filter: none`) | 18, 18 | 0 quadros acima de 50 ms |
| `develop` (blur de 10 px no menu e no player) | 18, 18, 18, 18 (máquina folgada); depois 41, 44, 21, 18 (carregada) | |
| vidro desta frente | 42, 40, 40, 40 (folgada); depois 35, 43, 71, 85 (carregada) | 1 a 2 quadros acima de 50 ms por execução |
| vidro com o blur do menu desligado | 18, 18 | o player em vidro sozinho não custou |

Não dá para concluir: a carga da máquina (média 25 em 12 threads, outros agentes compilando) fez a mesma `develop` variar de 18 a 44 ms. O sinal das rodadas folgadas aponta o `backdrop-filter` do menu lateral (coluna inteira) como o custo, e a `develop` também tinha esse blur (10 px, sem ambiente atrás). Não é o WebKitGTK do app. **O número que vale é o do app real** (`__miupyPerf.run()` na Discoteca com GPU e com `MIUPY_GPU=off`).

## Pendências e problemas conhecidos

- Medir RNF-13 no app real com GPU; se o menu lateral for o custo, o corte é tirar o `backdrop-filter` dele (o fundo translúcido sobre o ambiente já lê como vidro) ou usar `.glass` só no player.
- Conferir no app: player com capa de verdade, ambiente com capa clara (o texto não pode perder contraste), modal e menus abertos, e a transição de cor ao trocar de faixa.
- `.ambient` só aparece com o menu lateral e a página em basalto; no Tocando do desktop vale o mesh.
- A matriz de rastreabilidade não foi regenerada (só no branch de versão).

## Próximos passos

Frente seguinte: abas em pílula, mini player e Tocando do celular, usando `.glass`/`.glass-strong`, `--cover-tint` e `CoverGlow`.
