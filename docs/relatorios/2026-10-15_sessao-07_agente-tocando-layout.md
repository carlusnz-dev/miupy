# Layout da tela Tocando (RF-17)

- **Data:** 2026-10-15
- **Autor:** Claude
- **Modelo:** Sonnet 5.5
- **Escopo:** esconde o player de baixo em Tocando e reorganiza os controles da tela em duas linhas

## O que foi feito

- `src/App.svelte`: o `PlayerBar` (desktop) só existe fora de `/tocando`, dentro de um contêiner com a transição nova `springCollapse` (`src/lib/motion.ts`). Ela anima altura e opacidade com a mesma mola de `springTransition`, então o espaço é reservado e liberado sem salto. Respeita `prefers-reduced-motion`.
- `src/routes/NowPlaying.svelte`: os controles saíram da coluna estreita da capa e ganharam uma seção própria, centralizada e com a largura da tela. Linha 1: progresso e tempos. Linha 2: aleatório/Surfle, anterior, tocar/pausar e próximo ao centro; favorito, fila e volume no mesmo eixo, à direita. O favorito saiu do título e foi para essa linha; o botão de fila ativa a aba Fila.
- Abaixo de `md` a linha 2 quebra com `flex-wrap`, para caber no Android.
- Atalhos (`App.svelte`) e seletor de perfil do EQ (`dismissable`) não foram tocados.

## Decisões

- **Repetir não existe:** o player não tem modo de repetição (nenhum estado ou comando no código nem no contrato IPC). Não inventei o botão; fica para uma frente própria que defina o contrato.
- Tentei deixar a seção de controles fixa no rodapé (`sticky`), mas o `MeshGradient` tem `overflow-hidden`, que quebra o `sticky`. Desfiz em vez de mexer no componente compartilhado; em janelas baixas os controles rolam com a página.
- Não mexi em `TrackRow`, `TrackToolbar`, `Library` nem nos `<select>`.

## Verificação (comandos rodados e resultado real)

- Cargo (`fmt`, `clippy`, `test`): **não rodei**. O worktree não tem `target/` e a frente é só frontend; o CI roda.
- `npm run check`: passou, 0 erros e 0 avisos.
- `npm test`: passou, 74 testes (um novo, de `springCollapse`).
- `npm run build`: passou.
- `node scripts/rastreabilidade.mjs --check`: passou, 50 requisitos, nenhum ID desconhecido.
- Navegador: o MCP do Playwright não subiu (Chrome ausente em `/opt/google/chrome`). Usei o Chromium do cache do Playwright por script próprio, contra `vite` na porta 1430 (encerrado depois), em modo demonstração: `footer[aria-label=Player]` existe em `/` e na biblioteca, some em `/tocando` e volta ao sair. Em 960, 1280 e 1920 px a seção de controles tem 100 px de altura (progresso + uma linha), com volume alinhado ao eixo dos demais e dentro da largura da seção. Sem prints.
- Não rodei `npm run docs:matriz`.

## Commits

1. `feat(ui): esconde o player de baixo na tela tocando`
2. `feat(ui): centraliza os controles de tocando em duas linhas` (inclui o volume no eixo)
3. `test(ui): cobre a mola que recolhe o player`
4. `docs: registra o relatório do layout de tocando`
