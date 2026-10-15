# Favoritas em playlist

- **Data:** 2026-10-14
- **Autor:** agente favoritas
- **Modelo:** GPT-6 (esforço padrão)
- **Escopo:** RF-04 e RF-30, migração 0007, comando IPC e interface

## O que foi feito

- A migração 0007 marca uma playlist "Favoritas" existente sem diferenciar caixa ou cria uma nova, acrescenta as favoritas antigas em ordem sem repetir e substitui a tabela `favorites` por uma view da playlist marcada.
- O núcleo usa a playlist marcada para favoritar, desfavoritar e trocar a marcação. A FK limpa a marcação ao apagar a playlist. A busca no YouTube devolve `trackId` e `isFavorite` para faixas conhecidas.
- A casca expõe `favorites_set_playlist` e emite `library://changed` com `reason: 'favorites'`. O frontend inclui tipos, cliente, mock, botão de coração na busca, ação para marcar playlist, selo e aviso antes de apagar a marcada.
- Documentação do banco, tabela de invalidação do IPC e arquitetura JSON atualizadas.

## Decisões

- Se houver mais de uma playlist chamada "Favoritas" sem diferenciar caixa, a migração escolhe a mais antiga por `created_at` e depois por ID.
- A migração cria "Favoritas" mesmo quando o banco anterior não possui faixas favoritas, conforme o contrato. O ID novo usa UUID versão 7 com milissegundos da migração.
- Marcar a mesma playlist não emite evento. Apagar uma playlist comum emite só `playlist_deleted`; apagar a marcada emite também `favorites`.
- O contrato IPC permanece o mesmo; a tabela de invalidação passou a incluir playlists no motivo `favorites` porque a marcação e a contagem mudam.

## Verificação (comandos rodados e resultado real)

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test --workspace`: passou; casca 102 testes, 2 ignorados; áudio 33, 1 ignorado; núcleo 84; mídia 68; testes de rede 11 ignorados; doc-test 1.
- `npm ci`: passou, 79 pacotes instalados, nenhuma vulnerabilidade reportada.
- `npm run check`: passou, 0 erros e 0 avisos. Primeira tentativa falhou porque `node_modules` ainda não existia; passou após `npm ci`.
- `npm test`: passou, 53 testes em 14 arquivos.
- `npm run build`: passou.
- `node scripts/rastreabilidade.mjs --check`: passou, 50 requisitos, nenhum ID desconhecido.
- `git diff --check`: passou.
- Teste intermediário do núcleo falhou por expectativas antigas de quantidade de playlists e por `NULL` em `isFavorites` após exclusão; ambos foram corrigidos e a suíte completa passou.

## Pendências e problemas conhecidos

- O evento de atividade `track_favorited` só é registrado por `library_set_favorite`. Faixa que
  entra na playlist marcada por `playlist_add_tracks` (ou pela migração) não gera o evento; a
  pontuação lê a view `favorites` e fica certa (achado da revisão).

- Não houve verificação visual em janela Tauri ou Android nesta sessão.
- Testes de rede seguem ignorados pela suíte padrão do projeto.

## Próximos passos

- Revisar as fatias abaixo, criar commits e abrir PR para `develop` pela orquestração.

## Fatias de commit

1. `feat(db): migre favoritas para playlist marcada` — `crates/miupy-core/migrations/0007_favorites_playlist.sql`, registro da migração e testes em `crates/miupy-core/src/db.rs`.
2. `feat(core): use playlist marcada como fonte das favoritas` — `crates/miupy-core/src/model.rs`, `profile.rs`, `playlists.rs`, `tracks.rs`, expectativas em `stats.rs`.
3. `feat(shell): exponha marcação e estado das favoritas` — `src-tauri/src/commands/library.rs`, `commands/playlist.rs`, `commands/profile.rs`, `dto.rs`, `lib.rs`, `youtube.rs`.
4. `feat(ipc): tipifique e simule favoritas em playlist` — `src/lib/ipc/api.ts`, `types.ts`, `tauri.ts`, `mock.ts`, `mock.test.ts`, `src/lib/stores/stores.test.ts`, `src/lib/util/libraryEvents.ts`.
5. `feat(ui): permita favoritar na busca do youtube` — `src/routes/YouTube.svelte`.
6. `feat(ui): marque e identifique playlist de favoritas` — `src/routes/PlaylistDetail.svelte`, `src/routes/Playlists.svelte`.
7. `docs(docs): atualize esquema e arquitetura das favoritas` — `docs/arquitetura/banco-de-dados.md`, `arquitetura.json`, `ipc.md`, este relatório.
