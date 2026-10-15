---
name: revisar-playwright
description: Confere uma mudança de interface do miupy no navegador com o MCP do Playwright gastando poucos tokens (snapshot de acessibilidade e evaluate em vez de prints). Use quando revisar-design não basta para saber se a tela funciona.
---

# Revisar no navegador (Playwright MCP)

O servidor `playwright` está em `.mcp.json` (headless, perfil isolado, **sem imagem nas
respostas**). Ele roda o frontend com o **mock** (`src/lib/ipc/mock.ts`), não o app Tauri: serve
para fluxo, estado e acessibilidade; áudio, GPU e arquivos reais ficam para o autor no app.

## Subir

```sh
npm run dev -- --port 1430 --strictPort    # porta própria; a 1420 é do tauri dev
```

Se precisar de fundo para o servidor, rode-o num terminal do Orca, não em segundo plano na sessão.
Parâmetros do mock: `?perfTracks=300` (acervo maior), `?latency=800` (ver esqueletos).

## Conferir gastando pouco

1. `browser_navigate` para `http://localhost:1430/<rota>`.
2. `browser_snapshot` **uma vez por tela**: a árvore de acessibilidade diz papéis, nomes, estados
   (`expanded`, `pressed`, `disabled`) e textos. É o que você lê; não peça print para isso.
3. Ações pelo `ref` do snapshot (`browser_click`, `browser_type`, `browser_press_key` — ex.:
   Escape para popups). Depois da ação, leia só o que mudou com `browser_evaluate`, por exemplo:
   - `() => document.activeElement?.getAttribute('aria-label')` (foco voltou?);
   - `() => getComputedStyle(document.body).userSelect` (seleção desligada?);
   - `() => [...document.querySelectorAll('[data-row]')].length` (quantas linhas);
   - `() => document.querySelector('.player-bar') !== null` (componente na tela?).
4. Print (`browser_take_screenshot`) **só** quando o visual é o objeto (alinhamento, cor), no
   máximo um por tela; ele vai para `target/playwright/` e você o lê com a ferramenta de arquivo
   só se precisar.
5. Feche com `browser_close` e pare o `npm run dev`.

## Resultado

Para cada item: o que foi feito (rota, ação), o que o snapshot/evaluate mostrou, ok ou achado com
`arquivo:linha`. Diga o que não dá para provar no mock.
