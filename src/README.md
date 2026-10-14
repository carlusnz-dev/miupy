# Frontend (`src/`)

SPA em Svelte 5 (runes) + Vite + TypeScript + Tailwind 4. Sem SvelteKit. Textos em pt-BR, identificadores em inglês.

## Como rodar

```bash
npm install
npm run dev       # http://localhost:1420 (porta fixa, strictPort); no navegador usa o mock
npm run build     # gera dist/ (a casca Tauri aponta para ele)
npm run check     # svelte-check
npm run tauri dev # quando src-tauri/ existir
npm run docs:matriz
```

## Estrutura

```
index.html            script inline aplica data-lava antes da primeira pintura
src/main.ts, App.svelte   casca: layout responsivo, init das stores, atalhos de teclado
src/app.css           design tokens "magma" (variáveis CSS) + utilitários (.btn, .card, .field...)
src/lib/ipc/          types.ts (contrato), api.ts (interface MiupyApi), tauri.ts (real), mock.ts, index.ts
src/lib/stores/       router, player, library, downloads, recommendations, settings (tema), profile, toast, ui, selection
src/lib/components/   Icon, MeshGradient, Cover, TrackRow, Nav, PlayerBar, MiniPlayer, Modal, ...
src/lib/actions.ts    ações compartilhadas (favoritar, baixar) com aviso por toast
src/routes/           uma tela por rota (Home, Library, Playlists, PlaylistDetail, YouTube, Downloads,
                      NowPlaying, Artist, Stats, Dna, Wrapped, Profile, Settings)
```

Rotas (hash): `#/`, Discoteca (`#/biblioteca`), `#/playlists`, `#/playlists/:id`, `#/youtube`, `#/downloads`,
`#/tocando`, `#/artista/:id`, `#/estatisticas`, `#/dna`, `#/retrospectiva`, `#/perfil`, `#/configuracoes`.

## Cliente IPC e mock

`lib/ipc/index.ts` exporta `api`, uma implementação de `MiupyApi`:

- Dentro do Tauri (`window.__TAURI_INTERNALS__`): `tauri.ts`, um método por comando de
  `docs/arquitetura/ipc.md` (`invoke` com os argumentos em camelCase), erros normalizados para
  `{ code, message }`, eventos via `listen`.
- No navegador puro: `mock.ts`, em memória, com 28 faixas, 8 artistas, 3 playlists, estatísticas e
  retrospectiva geradas de forma determinística. O player simulado avança o tempo a cada 500 ms e emite
  `player://state` e `player://progress`; downloads progridem e emitem `download://progress` e
  `library://changed`. Os dados somem ao recarregar a página.

Capas: `api.resolveAsset(path)` usa `convertFileSrc` no Tauri (exige o protocolo `asset` habilitado em
`src-tauri/tauri.conf.json`, com escopo para o diretório de dados). Sem capa, `Cover` desenha um gradiente
derivado do id e das cores do tema.

## Tema da lava

`:root` define a paleta (`--lava-core`, `--lava`, `--lava-deep`, `--lava-ember`, `--lava-text`, `--lava-ink`),
o basalto (`--bg`, `--surface-1..3`, `--line`) e os gradientes derivados (`--grad-lava`, `--grad-ember`).
`data-lava="orange|purple|blue"` no `<html>` redefine só a rampa; basalto e gradientes se recalculam
(`color-mix`). `settings.setLava()` aplica na hora, grava em `localStorage` (cache para o script do
`index.html` evitar flash) e persiste com `settings_update`. A fonte da verdade é `settings_get`.

## MeshGradient

4 camadas com `radial-gradient` animadas só por `transform` (compositor), sem `filter: blur`. Pausa
fora da tela (`IntersectionObserver`) e com a aba oculta; `prefers-reduced-motion` desliga a animação.
Usado em: tocando agora, topo da Home, cabeçalhos de playlist e artista, cartões da retrospectiva e
pré-visualização das configurações.

## Movimento entre telas

`lib/layoutPresence.ts` registra apenas nós marcados com `use:useShared` ou a propriedade `shared` de
`Cover`. Chaves iguais produzem FLIP por `transform` entre telas; ao trocar de rota no meio do
movimento, a captura mais nova substitui a anterior. A animação reduzida usa apenas fade. O modal
nativo centraliza com `margin: auto` e mantém o diálogo aberto durante a saída animada.

## Layout

`md` (>= 768 px): barra lateral + conteúdo + barra do player. Abaixo: conteúdo + mini player + navegação
inferior (o menu "Mais" abriga Downloads, Estatísticas, Retrospectiva, Perfil e Configurações) e a tela
cheia `#/tocando`. Atalhos: espaço, setas (seek/volume), `M`; ignorados em campos de texto.

## Rastreabilidade

Componentes e stores levam `@req RF-NN` / `RNF-NN`. Testes de stores e utilitários: `npm test` (Vitest).
