# Menu translúcido e mesh animado em Tocando

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** menu lateral, fundo de Tocando e desempenho do mesh; sem alterar `App.svelte`.

## O que foi feito

- O menu lateral ganhou fundo translúcido e blur de 10 px, com cor sólida de recuo. A exceção foi registrada em RNF-04.
- `MeshGradient` ganhou a prop opt-in `animated`; só Tocando a ativa. As três camadas usam `transform` em ciclos de 12, 16 e 20 s; as cores da capa continuam com transição de 1 s.
- A animação pausa com janela desfocada ou oculta e é removida ao mudar de rota, mesmo durante a saída visual de Tocando. Sem movimento reduzido, segue animada apenas quando visível e em foco; com `prefers-reduced-motion`, fica estática. `will-change` só vale durante a animação ativa.

## Decisões

- O menu ocupa uma coluna própria. O `body` já pinta `--bg` atrás dessa coluna; a translucidez deixa esse fundo passar pelo menu sem alterar o layout de `App.svelte`. Como não há conteúdo rolando sob a coluna, o blur pode ser sutil no app atual.
- A rota é consultada em Tocando para remover a classe de animação imediatamente na navegação; depois a transição desmonta o componente. A pausa usa `animation-play-state` no CSS, sem `animation` inline que vença a regra.
- Não houve mudança de contrato IPC nem de estrutura de módulos; `arquitetura.json` permanece válido.

## Verificação (comandos rodados e resultado real)

- `npm test`: 21 arquivos e 94 testes passaram.
- `node scripts/rastreabilidade.mjs --check`: 52 requisitos, nenhum ID desconhecido.
- `npm run build`: bloqueado antes da compilação por EROFS em `node_modules/.vite-temp` (link somente leitura neste worktree). `npm run build -- --configLoader runner`: passou, 211 módulos transformados.
- `npm run check`: mesmo EROFS no pré-processamento de estilos. `npm run check -- --config /tmp/miupy-svelte-check.config.cjs`, com configuração temporária vazia para evitar esse pré-processamento: 0 erros e 0 avisos. A verificação padrão permanece impedida pelo ambiente.
- `git diff --check`: passou.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`: não rodados por instrução da frente frontend sem `target/`.
- Revisão de design pelo diff: tokens existentes, links e botão de menu preservados, movimento reduzido respeitado. Revisão Playwright não rodada: o MCP não está disponível nesta sessão. Sem medição visual ou de GPU no app real.

## Pendências e problemas conhecidos

- A orquestração deve medir no app real com GPU seguindo `docs/contribuicao/desempenho.md` (seção “GPU no Linux”): `MIUPY_GPU=on WEBKIT_SHOW_FPS=1 WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:9333 npx tauri dev`, inspetor remoto e `requestAnimationFrame`/`__miupyPerf.run()`; comparar Tocando, saída da rota e menu recolhido/expandido. Repetir três vezes e descartar a primeira, conforme o roteiro.
- Confirmar visualmente no WebKitGTK se a translucidez do menu sobre o fundo do app é perceptível; a coluna não sobrepõe conteúdo rolável.

## Fatias de commit

Sem commits, push ou PR neste worktree. Fatias sugeridas para a orquestração:

1. `feat(ui): adiciona fundo translúcido ao menu lateral` — `src/lib/components/Nav.svelte`, bloco `.nav-glass` de `src/app.css`, `docs/requisitos/requisitos.md`.
2. `feat(ui): anima o mesh somente na tela tocando` — prop e classes em `src/lib/components/MeshGradient.svelte`, uso em `src/routes/NowPlaying.svelte`, keyframes e regras `.mesh-animated` em `src/app.css`.
3. `fix(ui): pausa e remove o mesh fora de tocando` — efeito de foco e visibilidade em `src/lib/components/MeshGradient.svelte`, condição da rota em `src/routes/NowPlaying.svelte`, regras `.paused`, `will-change` e movimento reduzido em `src/app.css`.
4. `docs(docs): registra a frente visual da sessão 09` — este relatório.

## Próximos passos

- Medir GPU e rAF no app real e incorporar o resultado ao relatório de sessão da orquestração.
