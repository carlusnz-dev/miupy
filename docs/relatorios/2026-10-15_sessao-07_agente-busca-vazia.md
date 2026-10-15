# Busca vazia com sugestões (RF-34)

- **Data:** 2026-10-15
- **Autor:** agente implementador (frente `feat/busca-vazia`)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** histórico de buscas do YouTube, tela de busca vazia e botão na Discoteca (migração 0008)

## O que foi feito

- Migração `0008_search_history.sql` (`search_history`, STRICT, chave sem caixa/acento, data em ms) e teste.
- Núcleo: `record_search`, `recent_searches` e `clear_search_history` (50 guardadas, 10 listadas).
- Casca: `youtube_search` grava a consulta; `youtube_search_history` e `youtube_search_history_clear`
  registrados em `lib.rs`. `youtube_suggest` não grava.
- Frontend: API, cliente Tauri e mock; `SearchHistoryStore`; tela do YouTube com buscas recentes
  (clicar refaz a busca, "Limpar histórico") e `Recommendations` reaproveitado; botão "Adicionar
  músicas" na Discoteca.
- Docs: 0008 marcada como implementada, tabela descrita em `banco-de-dados.md`, `arquitetura.json`.

## Decisões

- A consulta só entra no histórico depois que a busca na rede dá certo (busca que falha, por
  exemplo sem internet, não grava). O contrato não dizia; o `ipc.md` não mudou.
- Repetir uma consulta apaga e reinsere a linha, para o desempate por `rowid` funcionar quando duas
  buscas caem no mesmo milissegundo (o upsert mantinha o `rowid` antigo).
- Espaços internos da consulta são colapsados ao gravar (`"daft   punk"` vira `"daft punk"`).
- "Caixa vazia" vale também depois de uma busca: apagar o texto volta às recentes e recomendações.
- `youtube_suggest` é função livre de rede, sem acesso ao banco; a garantia de que não grava é
  estrutural e não tem teste na casca (o mock tem teste).

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: sem diferenças.
- `cargo clippy --workspace --all-targets -- -D warnings`: sem avisos.
- `cargo test --workspace`: tudo passou (core 95, shell 108, mídia 72, áudio 35).
- `npm run check`: 0 erros, 0 avisos. `npm test`: 14 arquivos, 58 testes passando.
- `npm run build`: concluído. `node scripts/rastreabilidade.mjs --check`: ok, nenhum ID desconhecido.

## Pendências e problemas conhecidos

- Nenhum teste de componente Svelte (o projeto não tem); a tela é coberta só por `npm run check` e
  pelo teste do store.

## Próximos passos

- Revisão do PR.
