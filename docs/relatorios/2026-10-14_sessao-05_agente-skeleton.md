# Skeleton, cache entre telas e fluidez (RNF-11, RNF-13)

- **Data:** 2026-10-14
- **Autor:** agente `skeleton` (orquestrado)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** frontend (`src/`): medidor de fluidez, correção dos engasgos, esqueleto de carregamento e cache entre telas. Sem backend, sem compilar Rust.

## O que foi feito

Ramo `feat/skeleton-cache` (a partir de `upstream/develop`), três commits locais, sem push:

1. `feat(ui): adiciona o medidor de fluidez só de desenvolvimento` — `src/lib/dev/perf.ts` (`window.__miupyPerf.run()`), `perfStats.ts` (contas puras, testadas), `?perfTracks=N` no mock. Fora do bundle de produção.
2. `perf(ui): corta o trabalho por evento e por linha que engasgava a interface` — reação por `reason`, estado do player campo a campo, pontuação por entrada, mapa de jobs, `Cover` sem `$effect`, animação de entrada só nas 14 primeiras linhas, utilitários `diff`, `libraryEvents`, `chunked`.
3. `feat(ui): mostra esqueleto e guarda o cache das telas entre navegações` — `Resource` (cache por consulta + invalidação por evento), `Skeleton`/`SkeletonList`, todas as telas que tinham spinner, `State` sem spinner, listas longas em blocos.

Mais docs: `docs/contribuicao/desempenho.md` (roteiro, como rodar, metas, antes/depois), tabela de `reason` em `ipc.md`, `src/README.md`, `arquitetura.md` e `.json`.

### Causas: confirmadas e derrubadas

| # | Suspeita | Resultado | Evidência |
|---|---|---|---|
| 1 | `library://changed` refaz tudo em toda tela | **Confirmada (principal)** | Mock com 1500 faixas, Discoteca aberta, 10 eventos em 2 s: 17 a 19 quadros de 117 a 183 ms por rodada. Sondagem isolada: com `artist_photo_updated` (que agora não atualiza listas de faixas) 0 quadros acima de 30 ms. |
| 1b | (nova) `scores.invalidate()` repedia a pontuação de todas as linhas a cada evento e a cada troca de faixa | **Confirmada** | Depois de filtrar por motivo, a rajada de `metadata_refreshed` passou de 10 quadros lentos para 0 na sondagem. |
| 2 | Toda tela abre em `loading = true` com spinner e perde o conteúdo | **Confirmada** | Leitura: 8 telas montavam com `loading = true` e `<State kind="loading">`; não havia cache entre telas. É a causa do "props sumindo e voltando" (também `Downloads` mostrava o `trackId` cru até chegar a lista de faixas). |
| 3 | `player://progress` / `player://state` re-renderizam demais | **Derrubada como causa de engasgo, corrigida por higiene** | `progress` já escreve campo a campo (reatividade fina); nenhuma lista lê a posição. `player://state` trocava `this.state` inteiro, o que reavaliava derivados; agora entra campo a campo. Sem efeito mensurável isolado. |
| 4 | Mola/layout presence e `Cover` | **Parcial** | `Cover` tinha um `$effect` por capa (removido). A mola `spring-in` por linha pesa ao montar centenas de linhas: sondagem isolada no Firefox mostrou que, sem a animação das linhas, somem os quadros de 33 ms. `layoutPresence` não apareceu como custo. |
| 5 | (nova) Montar a lista inteira de uma vez | **Confirmada** | Sem virtualização: Discoteca de 1500 faixas, quadro de 450 a 566 ms; 300 faixas, 117 a 150 ms. |

Descartado por piorar: `content-visibility: auto` nas linhas (de 0 para 40 quadros acima de 50 ms ao montar a lista no Firefox).

### Correções

- **`Resource` (`stores/resource.svelte.ts`)**: guarda a última resposta por chave em memória; `prime` mostra o cache na hora, `load` atualiza em segundo plano; resposta igual mantém a referência (`diff.ts`), diferente troca só o item; erro com dado em cache mantém a tela; resposta de consulta já trocada é descartada; `invalidate(tags)` agenda as telas abertas que dependem do grupo, com no máximo uma consulta a cada 500 ms (rajada de 10 eventos vira 1 a 4 consultas). Por quê: toda tela refazia a consulta inteira e trocava o array a cada evento.
- **`libraryEvents.ts`**: `reason` → grupos (`tracks`, `stats`, `playlists`, `artist`); desconhecido atualiza tudo. Foto de artista não toca em listas de faixas; fim de download não toca em playlists. O player só relê a faixa atual quando o motivo pode mudá-la; a pontuação só recalcula em `favorite`/`favorites`/`track_deleted`.
- **Pontuação**: `SvelteMap` por faixa (antes um objeto trocado inteiro, que repintava todas as linhas) e `invalidate(ids)` só das duas faixas que mudaram na troca de faixa.
- **Downloads**: `forTrack` lê um mapa reativo `trackId → job` em vez de varrer todos os jobs em cada linha a cada `download://progress`.
- **Listas longas** (`chunked.svelte.ts`): Discoteca, Playlist e Artista montam 20 linhas na primeira passada e o resto aos poucos (bloco adaptativo de 4 a 40 linhas, conforme o tempo do quadro); um espaçador mantém o tamanho da barra de rolagem. Só as 14 primeiras linhas animam a entrada.
- **Skeleton** (`Skeleton.svelte`, `SkeletonList.svelte`, estilos no próprio componente, chapado, pulso de opacidade de 1,6 s, parado com `prefers-reduced-motion`): variantes `tracks`, `cards`, `stats`, `jobs`, `recs`, `lines`, `hero`, `form`, `card`. Telas: Início, Discoteca, Playlists, Playlist, Artista, Estatísticas, Downloads, DNA, Retrospectiva, Perfil, busca do YouTube, sugestões e letra. `State` perdeu o tipo `loading` (não há mais spinner no app).
- **Cache**: Início, Estatísticas, Artista, Discoteca, Playlist, Downloads (títulos), Retrospectiva (por ano), sugestões (por playlist) e playlists (na store) mostram o último dado na hora. DNA e perfil já guardavam o dado em store; passaram a usar `mergeDeep`/esqueleto.
- **Mock**: `?perfTracks=N` (acervo grande) e `?latency=ms` (ver os esqueletos). Só em dev.

### Antes e depois

Motor: **Firefox 157 headless (Gecko, renderização por software) no mock**, máquina em uso por outros processos (ruído de ~50 ms em quadros isolados). 3 rodadas por célula depois de uma de aquecimento. Detalhes e tabela por passo em `docs/contribuicao/desempenho.md`.

| Acervo | Versão | Quadros > 50 ms (3 rodadas) | Maior quadro | p95 | Quadros rápidos |
|---|---|---|---|---|---|
| 28 | antes / depois | 0/0/0 / 0/0/0 | 33 / 49 ms | 17,3 / 17,3 | 100% / 99% |
| 300 | antes / depois | 4/4/6 / 2/0/0 | 184 / 50 ms | 17,3 / 17,2 | 97% / 98% |
| 1500 | antes / depois | 27/25/27 / 3/3/4 | 567 / 134 ms | 34,2 / 17,3 | 94% / 95% |

A meta (nenhum quadro > 50 ms, 95% abaixo de 16,7 ms) fecha com 28 faixas, quase fecha com 300 (restam quadros em 50 ms) e **não fecha com 1500 faixas**: sobram 3 a 4 quadros de 50 a 134 ms nos passos Artista e Discoteca/rolagem. Sem virtualização de lista, isso é o piso por enquanto.

## Decisões

- "Quadro rápido" = < 16,7 ms + 1 ms de folga. O `requestAnimationFrame` a 60 Hz oscila entre 16,6 e 17,4 ms; sem folga metade dos quadros "falharia". Está no código e na doc.
- `Resource` guarda as respostas só em memória (sessão), como o contrato manda; não persiste em disco.
- Lista em blocos em vez de virtualização: bem menos invasivo (foco, arrastar para reordenar, transições compartilhadas continuam iguais), mas ainda monta todas as linhas no fim. Virtualização fica como próximo passo se o app real mostrar necessidade.
- Conflitos evitados: não mexi em `Nav.svelte`, `App.svelte`, `Toasts.svelte`, `app.css` nem em `AudioSection.svelte`. Em `NowPlaying.svelte` só troquei o spinner da letra (`State loading` → `SkeletonList variant="lines"`) e acrescentei o import, 2 linhas.
- `library.version` foi removido (ninguém mais o usa); `library.playlists` agora é derivado de um `Resource`, e `playlistsReady`/`playlistsError` alimentam o esqueleto.

## Verificação (comandos rodados e resultado real)

- `npm run check`: 0 erros, 0 avisos (375 arquivos).
- `npm test`: 11 arquivos, 49 testes passando (novos: `resource.test.ts`, `diff.test.ts`, `libraryEvents.test.ts`, `chunked.test.ts`, `perfStats.test.ts`, e dois testes do player em `stores.test.ts`; ajustado o teste de `library`, que esperava `version` e 120 ms).
- `npm run build`: ok; `grep` no `dist/` por `__miupyPerf`, `__miupyMockEmit`, `perfTracks`, `createMockApi`, `latency`: nenhuma ocorrência (o medidor e o mock não entram no bundle).
- `node scripts/rastreabilidade.mjs --check`: ok, 50 requisitos, nenhum ID desconhecido. Não rodei `docs:matriz` (o CI gera).
- Não rodei `cargo` (frente só de frontend).
- Captura de tela dos esqueletos com `?latency=60000` no Firefox headless (Início, Discoteca, Playlists, Artista, Downloads, Estatísticas): conferi Início, Discoteca, Artista e Playlists. Não conferi Perfil, Retrospectiva, DNA, YouTube e sugestões visualmente.
- Não há teste automatizado de componente (o projeto não tem jsdom); o esqueleto e o `prefers-reduced-motion` foram verificados por leitura e captura, não por teste.

## Pendências e problemas conhecidos

- **Medir no app real** (WebKitGTK + GPU): passo a passo em `docs/contribuicao/desempenho.md`. O número do Firefox por software não vale como absoluto.
- Com milhares de faixas ainda sobram quadros de 50 a 134 ms (ver acima).
- Favoritar ainda repede a pontuação de todas as linhas já carregadas (o evento não traz o id da faixa).
- Uma tela com erro e cache mostra o cache e esconde o erro (o erro só aparece sem dado); não há aviso de "dados desatualizados".
- Se uma tela abre durante uma consulta em andamento de outra instância da mesma chave, as duas consultam (sem deduplicar entre instâncias).

### Propostas para o backend (não implementadas)

1. `src-tauri/src/events.rs:69` (`library_changed`): incluir o id afetado no payload, por exemplo `{ reason, trackIds?: string[], artistId?, playlistId? }`. Com isso o frontend atualiza só a linha (e só a pontuação da faixa) em vez de reconsultar a lista. Hoje o `reason` já permite o filtro por grupo, mas não por item.
2. `src-tauri/src/metadata.rs:162` e `:291` (`metadata_refreshed`): o reprocessamento emite um evento por faixa, e o lote já emite `metadata_batch_finished` ao fim (`metadata.rs:82`). Emitir só o evento final durante um lote (e um por faixa só nas chamadas avulsas) tiraria dezenas de eventos por reprocessamento.
3. `src-tauri/src/metadata.rs:387` e `:431` (`artist_photo_updated`, `artist_updated`): já existe o evento `artist://changed { artistId }` no contrato; emitir só ele (não `library://changed`) para fotos e dados de artista, e a tela do artista assinar esse evento. Assim `library://changed` volta a significar "faixas ou playlists mudaram".
4. Sem urgência: um comando `library_list_tracks` com `ids`/paginação ajudaria a virtualizar a Discoteca.

## Próximos passos

- Autor mede no app real e preenche a tabela vazia de `desempenho.md`.
- Se o app real mostrar quadros > 50 ms em Discoteca/Artista com a biblioteca dele, virtualizar a lista (linha de 56 px fixa) no lugar do `Chunked`.
- Aplicar as propostas 1 e 3 no backend e trocar a lista de `libraryEvents.ts` por `trackIds`/`artistId`.
