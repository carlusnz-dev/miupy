# Tela Tocando: entrada animada, letra em destaque e mesh com as cores da capa

- **Data:** 2026-10-15
- **Autor:** agente `feat/tocando-visual` (Sonnet, Orca)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** visual da tela Tocando (RF-05, RF-17, RNF-05, RNF-08, RNF-13) e novo RF-40.

## O que foi feito

- Entrada: capa, título, artista, chips, botão do EQ, abas, lista/letra e controles usam `.enter`
  (fade + Y) com `--enter-delay` escalonado (0 a 320 ms). A capa entra por um `div` próprio para não
  brigar com o `transform` de hover do `.motion-cover`.
- Título com `AnimatedText` (reanima a cada troca de faixa).
- Letra sincronizada: linha atual com `text-shadow` estático e gradiente de texto (cópia em `::after`
  com `background-clip:text`, porque texto transparente esconderia o `text-shadow`); as outras a 70% de
  opacidade; `aria-current` na linha atual. A rolagem suave centrada já existia. Sem letra sincronizada
  nada muda.
- `src/lib/util/palette.ts`: `dominantColors` (pura; quantização em 16 níveis por canal, descarta
  preto, branco, cinza e transparente, cores distintas por distância) e `coverPalette` (canvas 32×32,
  `null` se tainted). 4 testes.
- `MeshGradient` ganhou `colors?: string[]` (escreve `--mesh-1..3` e `--mesh-dur: 1000ms`); a transição
  usa as `@property` já existentes, sem camada nova e sem animação contínua. Sem capa ou canvas
  tainted: cores do tema.
- RF-40 acrescentado em `docs/requisitos/requisitos.md`.

## Decisões

- Prioridade C no RF-40 (visual, não afeta o uso).
- Duração própria (1 s) só quando há `colors`, via `--mesh-dur`; o resto do app segue `--dur-slow`.
- Todas as faixas da paleta vêm da capa; se houver menos de 3 cores, as faltantes ficam no tema.

## Verificação

- `npm run check`: 0 erros, 0 avisos. `npm test`: 21 arquivos, 91 testes passaram. `npm run build`: ok.
- `node scripts/rastreabilidade.mjs --check`: ok, 51 requisitos.
- Cargo (fmt/clippy/test): **não rodado**, só frontend e sem `target/` no worktree; o CI roda.
- `revisar-playwright`: **não rodado**, o MCP do Playwright não está disponível nesta sessão (subi o dev
  na porta 1430 só para conferir que responde e o encerrei). Não provado no navegador: a aparência do
  gradiente na linha atual, a leitura do canvas com capas `asset://` (CORS) e a transição de 1 s.

## Pendências e problemas conhecidos

- Conferir no app real o custo da transição de cor e se o gradiente da linha atual tem contraste bom.
- Capas muito escuras ou cinzas dão 0 cores e caem para o tema.

## Próximos passos

- Medir CPU/GPU da tela Tocando com o inspetor remoto (o autor abre o app).
