# Surfle (RF-24)

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6, esforço alto
- **Escopo:** troca do modo inteligente por Surfle, sorteio diverso e migração do SQLite

## O que foi feito

- Renomeei o modo salvo e o contrato para `surfle`. O botão mostra Surfle e o ciclo do frontend inclui o novo valor.
- O núcleo evita duas faixas do mesmo artista em sequência enquanto há outra opção, penaliza o artista nas duas posições anteriores, combina pontuação, gênero da última faixa e afinidade do perfil de gosto. Usa os gêneros do artista quando a faixa não os tem.
- A casca fornece `top_genres` ao sorteio e conserva o tratamento existente de ocorrências repetidas, prefixo tocado, trocas de modo e faixa inicial. Os testes existentes desse fluxo passaram com Surfle.
- Criei a migração 0010, que reconstrói `settings` com todas as colunas, padrões, verificações, `STRICT` e FK, convertendo o valor salvo `smart` em `surfle`. O teste parte da versão 9 com a linha preenchida e confirma dados, integridade referencial e `ON DELETE SET NULL`.
- Atualizei o contrato, a fórmula, os requisitos, a documentação do banco e o mapa da arquitetura.

## Decisões

- A repetição imediata recebe peso zero apenas enquanto existir uma candidata de outro artista. Assim que só restam faixas do mesmo artista, todas voltam a concorrer com peso positivo e nenhuma ocorrência se perde.
- O saldo negativo de faixa fica no piso positivo do sorteio. Isso reduz sua chance sem removê-la.
- Menções ao valor antigo permanecem somente onde documentam ou testam a conversão da migração 0010, na migração histórica 0003, em relatórios históricos, na matriz gerada de uma versão anterior e em URLs de fixtures externos. A matriz será gerada no branch de versão, conforme `AGENTS.md`.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou, sem avisos.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace`: passou. Casca: 116 testes passaram, 2 ignorados; núcleo: 102 passaram. Outros testes e doc tests passaram, com testes de rede ignorados conforme configuração.
- `npm run check`: passou, 0 erros e 0 avisos. A tentativa inicial encontrou `EROFS` no cache do Vite; a execução válida usou links temporários locais e restaurou o link original de `node_modules`.
- `npm test`: passou, 16 arquivos e 73 testes. Uma tentativa com o link temporário dentro do workspace descobriu dois testes de uma dependência e falhou por ausência de `uvu`; a execução válida colocou o link original em `/tmp` enquanto testava e o restaurou ao fim.
- `npm run build`: passou, 202 módulos transformados. A tentativa inicial encontrou `EROFS` no cache do Vite; a execução válida usou cache local temporário.
- `node scripts/rastreabilidade.mjs --check`: passou, 50 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.
- Não rodei `npm run docs:matriz`.

## Fatias de commit

Mensagens sugeridas para a orquestração, nesta ordem. Cada fatia inclui `Refs: RF-24` no rodapé.

1. `feat(db): migra as configurações salvas para surfle` — `crates/miupy-core/migrations/0010_surfle.sql`, `crates/miupy-core/src/db.rs` (registro da migração, versão 10 e teste de migração), `docs/arquitetura/banco-de-dados.md` (esquema e migrações reservadas).
2. `feat(ipc)!: renomeia o modo inteligente para surfle` — `crates/miupy-core/src/model.rs` (enum e tipo da faixa), `crates/miupy-core/src/profile.rs` (testes do enum salvo), `crates/miupy-core/src/lib.rs` (exportação), `src/lib/ipc/types.ts`, `docs/arquitetura/ipc.md`. Rodapés: `BREAKING CHANGE: ShuffleMode substitui smart por surfle no IPC e no banco` e `Refs: RF-24`.
3. `feat(core): evita o mesmo artista em sequência no surfle` — `crates/miupy-core/src/shuffle.rs` (algoritmo e testes determinísticos).
4. `feat(core): usa os gêneros do artista no surfle` — `crates/miupy-core/src/recommendations.rs` (fallback e teste).
5. `feat(shell): aplica o gosto por gênero ao surfle` — `src-tauri/src/player.rs` (chamada ao núcleo e testes de fila).
6. `feat(ui): exibe o modo surfle no botão de aleatório` — `src/lib/util/shuffle.ts`, `src/lib/util/features.test.ts`, `src/lib/components/ShuffleButton.svelte`, `src/lib/ipc/mock.ts`.
7. `docs(docs): descreve a fórmula e o requisito do surfle` — `docs/arquitetura/recomendacao.md`, `docs/requisitos/requisitos.md`, `docs/arquitetura/arquitetura.json`, este relatório.

## Pendências e problemas conhecidos

- A matriz de rastreabilidade ainda cita o nome antigo porque só deve ser regenerada no branch de versão. Relatórios anteriores registram o contrato da época. Nenhum código ativo usa o nome antigo fora da migração e de seus testes.
- Não houve verificação manual em janela desktop ou Android.

## Próximos passos

- A orquestração pode separar as fatias acima, criar os commits e revisar o PR para `develop`.
