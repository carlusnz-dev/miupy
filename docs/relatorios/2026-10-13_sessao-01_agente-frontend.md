# Frontend inicial (Svelte 5 + Vite + Tailwind 4) com cliente IPC e mock

- **Data:** 2026-10-13
- **Autor:** agente frontend (subagente do Claude Code, orquestrado por Claude Opus 5.5 a pedido de Carlos Antunes)
- **Modelo:** Claude Sonnet, esforço medium
- **Escopo:** projeto Node na raiz e `src/`: cliente IPC tipado com mock, stores, todas as telas, tema magma.

## O que foi feito

- Projeto Node: `package.json` (scripts `dev`, `build`, `preview`, `check`, `tauri`, `docs:matriz`), `vite.config.ts`
  (porta 1420, `strictPort`, saída `dist/`), `svelte.config.js`, `tsconfig.json`, `index.html`, `public/favicon.svg`.
- `src/lib/ipc/`: tipos de `ipc.md`, interface `MiupyApi`, cliente Tauri (um método por comando, erros
  normalizados, eventos) e mock em memória (28 faixas, playlists, estatísticas, retrospectiva, letras, player
  simulado, fila de downloads simulada). Seleção automática por `__TAURI_INTERNALS__`.
- Stores (runes): router por hash, player, library/playlists, downloads, settings (tema), profile, toast, ui, selection.
- Telas: Início, Biblioteca (busca, ordenação, favoritas, seleção múltipla com baixar/adicionar a playlist),
  Playlists (criar, renomear, apagar), detalhe (busca, ordenação, reordenar por arrastar e por botões, remover,
  baixar playlist), Buscar no YouTube, Downloads (progresso e cancelamento), Tocando agora (seek, volume, favorito,
  fila, letra sincronizada), Artista, Estatísticas, Retrospectiva (6 cartões), Perfil (validação), Configurações
  (seletor da lava com pré-visualização).
- Barra do player (desktop), mini player e navegação inferior (mobile), atalhos de teclado (espaço, setas, M).
- Tema magma: tokens em CSS variables, `data-lava` aplicado antes da primeira pintura (script inline + cache em
  localStorage), `MeshGradient` reutilizável (transform/opacity, pausa fora da tela, respeita reduced-motion).
- `src/README.md` e anotações `@req` nos componentes e stores.

## Decisões

- TypeScript 6 em vez do 7: `svelte-check` 4.7 declara peer `typescript ^5 || ^6`, e o 7.0.2 quebrava o `npm install`.
- Roteador próprio por hash (sem dependência); funciona igual no WebView e no navegador.
- Sem biblioteca de ícones: SVGs inline em `Icon.svelte`.
- Mock fora do bundle real? Não: o mock entra no bundle de produção (a escolha é em runtime). Custo pequeno; pode virar
  import dinâmico depois.
- Erros do backend viram toast (ações) ou `State kind="error"` com "Tentar de novo" (carregamentos).

## Verificação (comandos rodados e resultado real)

- `npm run check` -> `COMPLETED 314 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS`.
- `npm run build` -> `158 modules transformed`, `dist/index.html 0.87 kB`, `dist/assets/index-*.css 32.97 kB (7.36 kB gzip)`,
  `dist/assets/index-*.js 159.05 kB (52.06 kB gzip)`, `built in 1.62s`.
- `npm run dev` + `curl`: `http://localhost:1420/` -> 200 (HTML com o cliente do Vite), `/src/main.ts`, `/src/App.svelte` e
  `/src/lib/ipc/mock.ts` -> 200. Servidor encerrado em seguida (nada ouvindo na 1420).
- Smoke test do mock em Node (script temporário, fora do repositório): listagem/busca/ordenação, play + eventos
  (`state`, `p`), seek, retrospectiva, busca YouTube, enqueue e erro `invalid_input` ao criar playlist sem nome -> comportaram-se
  como esperado.
- `node scripts/rastreabilidade.mjs --check` -> `ok: 24 requisitos, nenhum ID desconhecido.` (não rodei `docs:matriz`
  sem `--check`, pois escreve em `docs/rastreabilidade/`, fora da minha área.)

## Pendências e problemas conhecidos

- **A aparência não foi conferida visualmente** (não há navegador aqui): layout responsivo, contraste, mesh animado,
  gradientes e a navegação por teclado/foco foram escritos com cuidado, mas nunca vistos. Esperar ajustes finos.
- Fluxos interativos (clicar, arrastar, formulários) não foram executados em navegador; só compilação, tipos, HTTP e o
  mock em Node.
- Cliente Tauri (`tauri.ts`) não foi exercitado contra o backend real: `src-tauri/` ainda não existe.
- Capas locais dependem do protocolo `asset` do Tauri estar habilitado e com escopo no diretório de dados.
- Sem testes automatizados (nenhum `@test`).
- Perfil: o contrato tem `avatarPath`, mas não há comando para escolher/enviar imagem; o avatar mostra iniciais.
- Acessibilidade: rótulos e `aria-*` nos controles principais, mas sem auditoria com leitor de tela.
- Retrospectiva: o seletor de ano oferece só os 3 últimos anos (o contrato não lista anos com dados).
- Tauri: a lógica do botão "Mais" no mobile fecha o menu ao trocar de rota; o seek otimista do player pode piscar se o
  backend demorar a confirmar.

## Lacunas do contrato ipc.md (sem desvio, mas notadas)

- Não há comando para listar faixas de um artista; a tela de artista filtra `library_list_tracks` no cliente por `artistId`.
- A fila só traz ids; o cliente resolve cada faixa com `library_get_track` (N chamadas por mudança de fila). Um comando
  `player_get_queue` ou `Track[]` no estado resolveria.
- `player://state` não diz se a faixa não tem arquivo (não baixada); `player_play` deve devolver erro (`unavailable`) e o
  cliente mostra o toast.
- Não há comando para remover/limpar downloads finalizados nem para retentar um job; "tentar de novo" reenfileira com
  `download_enqueue`.
- `stats_wrapped` não informa os anos disponíveis; `playlist_move_track` e `library_delete_track` retornam `void`, então o
  cliente recarrega a lista após mover.
- Ordenação `position` só faz sentido em playlist; o contrato não diz o que `library_list_tracks` faz com ela (o cliente nunca a envia).

## Próximos passos

1. Criar `src-tauri/` apontando para `dist/` e `http://localhost:1420`, habilitar o protocolo `asset` e testar o cliente real.
2. Ver o app no navegador/WebView (desktop e Android) e ajustar densidade, contraste e desempenho do mesh.
3. Carregar o mock por `import()` dinâmico para tirá-lo do bundle dentro do Tauri.
4. Testes (Vitest) para `parseLrc`, validação do perfil e o mock; marcar com `@test`.
