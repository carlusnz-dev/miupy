# Desempenho e fluidez (RNF-13)

Meta do RNF-13: no roteiro abaixo, **nenhum quadro acima de 50 ms** e **pelo menos 95% dos quadros
abaixo de 16,7 ms**. Como o `requestAnimationFrame` oscila entre 16,6 e 17,4 ms mesmo sem perder
quadro em 60 Hz, o medidor conta como "rápido" o quadro abaixo de 16,7 ms + 1 ms de folga
(`TARGET.jitterMs` em `src/lib/dev/perfStats.ts`).

## O medidor

`src/lib/dev/perf.ts` existe só em desenvolvimento: `main.ts` o importa sob `import.meta.env.DEV`,
então ele (e o mock) ficam fora do `dist/`. Ele mede o intervalo entre quadros por
`requestAnimationFrame` durante todo o roteiro e, quando o motor suporta (Chromium; não o Firefox nem o
WebKitGTK), conta *long tasks* com `PerformanceObserver`. As contas puras (percentil, quadros acima de
50 ms, meta) estão em `perfStats.ts`, com testes.

### Roteiro fixo

`window.__miupyPerf.run()` faz, nesta ordem, com a tela em 60 Hz:

0. toca a primeira faixa da discoteca (com fila de até 50) e espera 0,6 s;
1. Início, 2. Discoteca, 3. Artista (o da primeira faixa), 4. Playlists, 5. Tocando (0,9 s em cada);
6. abre a aba da fila (a primeira aba da tela Tocando);
7. volta ao Início;
8. volta à Discoteca e rola a lista de cima a baixo e de volta por 2 s;
9. na Discoteca, dispara 10 eventos `library://changed` em 2 s (motivos variados:
   `metadata_refreshed`, `artist_photo_updated`, `artist_updated`, `download_done`) e espera 1 s.

Devolve `{ frames, meanMs, p50Ms, p95Ms, p99Ms, maxMs, over50, fastRatio, longTasks, ok, engine,
tracks, steps[] }`; `steps` repete o resumo por passo, para achar qual tela engasga.

### No app real (o número que vale)

```bash
npx tauri dev
```

Abra o DevTools da janela (botão direito, "Inspecionar elemento") e, no console:

```js
const r = await __miupyPerf.run();
console.table(r.steps);
r.ok; r.over50; r.fastRatio; r.p95Ms; r.maxMs
```

Cuidados: o passo 0 **toca a primeira faixa de verdade** (baixe o volume antes); o passo 9 usa o
`emit` do Tauri, o que dispara o mesmo `library://changed` que o backend emite. Rode 3 vezes e ignore
a primeira (caches frios). Feche outras janelas pesadas e, se der, use uma discoteca com algumas
centenas de faixas.

### No navegador, com o mock

```bash
npm run dev                       # http://localhost:1420 (use outra porta se o app já ocupar)
# no console da página:  await __miupyPerf.run()
```

Parâmetros do mock (só em dev):

- `?perfTracks=1500` replica o acervo de exemplo até N faixas (máx. 5000). Com as 28 faixas padrão quase
  nada engasga; as telas só mostram os problemas com centenas de faixas.
- `?latency=800` soma atraso a toda chamada do mock, para ver os esqueletos na tela.

Em Firefox sem janela (como foi medido abaixo) use o Marionette: `firefox --headless --marionette
--profile <dir temporário>`, navegue até a URL do mock e chame `__miupyPerf.run()` com
`WebDriver:ExecuteAsyncScript`. O script de medição usado fica fora do repositório.

## Resultados

Motor: **Firefox 157 headless (Gecko, renderização por software) no mock**, em Linux, com a máquina
em uso por outros processos. Não é o WebKitGTK do app: serve para comparar antes e depois, não como
número absoluto. Três rodadas por célula, depois de uma rodada de aquecimento descartada.
"Quadros > 50 ms" é a lista por rodada. Antes = commit `baab9e4` (só o medidor); depois = `HEAD` do
ramo `feat/skeleton-cache`.

| Acervo (mock) | Versão | Quadros > 50 ms (3 rodadas) | Maior quadro (pior rodada) | p95 (mediana) | Quadros rápidos (mediana) |
|---|---|---|---|---|---|
| 28 faixas | antes | 0 / 0 / 0 | 33 ms | 17,3 ms | 100% |
| 28 faixas | depois | 0 / 0 / 0 | 49 ms | 17,3 ms | 99% |
| 300 faixas | antes | 4 / 4 / 6 | 184 ms | 17,3 ms | 97% |
| 300 faixas | depois | 2 / 0 / 0 | 50 ms | 17,2 ms | 98% |
| 1500 faixas | antes | 27 / 25 / 27 | 567 ms | 34,2 ms | 94% |
| 1500 faixas | depois | 3 / 3 / 4 | 134 ms | 17,3 ms | 95% |

Por passo, 1500 faixas (maior quadro na pior das 3 rodadas):

| Passo | Antes | Depois |
|---|---|---|
| 2 Discoteca (montar a lista) | 566 ms | 49 ms |
| 3 Artista (~190 faixas) | 250 ms | 84 ms |
| 8 voltar e rolar a Discoteca | 567 ms | 134 ms |
| 9 rajada de `library://changed` | 183 ms (17 a 19 quadros acima de 50 ms por rodada) | 33 ms (nenhum acima de 50 ms) |

Leitura honesta: com 28 e 300 faixas a meta quase fecha (restam quadros de 50 ms no limite, dentro do
ruído de uma máquina em uso); com 1500 faixas e sem virtualização ainda sobram 3 a 4 quadros entre 50 e
134 ms, nos passos Artista e Discoteca/rolagem. **Falta medir no app real** (WebKitGTK, GPU).

## O que engasgava (e o que foi feito)

| Causa | Evidência | Correção |
|---|---|---|
| `library://changed` refazia toda consulta de toda tela aberta, e o player relia a faixa atual e a pontuação de todas as linhas | rajada de 10 eventos: 17 a 19 quadros de 117 a 183 ms (1500 faixas) | o `reason` decide quais grupos (`tracks`, `stats`, `playlists`, `artist`) se atualizam; no máximo uma consulta a cada 500 ms por tela; a resposta só troca o item que mudou; pontuação só recalcula em `favorite`/`track_deleted` |
| Montar centenas de linhas de uma vez | Discoteca com 1500 faixas: quadro de 450 a 566 ms; 300 faixas: 117 a 150 ms | `Chunked`: 20 linhas na primeira passada, o resto de 4 a 40 por quadro conforme o tempo do quadro; um espaçador mantém a barra de rolagem |
| Centenas de animações `spring-in` ao mesmo tempo | sondagem isolada no Firefox: com as animações das linhas desligadas, somem os quadros de 33 ms ao montar | só as 14 primeiras linhas animam a entrada |
| Cada linha varria todos os jobs de download | O(linhas × jobs) por evento `download://progress` (leitura do código) | mapa `trackId → job` por entrada reativa |
| `scores.invalidate()` repedia a pontuação de todas as linhas a cada troca de faixa | leitura do código | só as duas faixas que mudaram |
| Spinner e conteúdo sumindo a cada navegação | leitura do código (`loading = true` em toda montagem) | cache por consulta e esqueleto no formato do conteúdo (RNF-11) |

Tentativas que pioraram e foram descartadas: `content-visibility: auto` nas linhas (de 0 para 40
quadros acima de 50 ms ao montar a Discoteca no Firefox).

## Tabela para o app real

Preencha depois de rodar no `tauri dev` (3 rodadas, ignorando a primeira):

| Acervo real | Quadros > 50 ms | Maior quadro | p95 | Quadros rápidos | Long tasks |
|---|---|---|---|---|---|
| (n faixas) | | | | | |
