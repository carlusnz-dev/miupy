# miupy — guia para agentes

Player de música offline (Rust + Tauri 2, desktop e Android). Leia antes de mexer:
`docs/requisitos/requisitos.md`, `docs/arquitetura/arquitetura.md`, `docs/arquitetura/ipc.md`,
`docs/stacks/stacks.md`.

## Ambiente

- Rust não está no `PATH` do shell não interativo: `export PATH="$HOME/.cargo/bin:$PATH"`.
- Gerenciador de pacotes JS: `npm`. CLI do Tauri: `npx tauri`.
- Adicione dependências com `cargo add` / `npm install` (versão atual), não chute versões.

## Regras

- `crates/miupy-core`, `miupy-audio` e `miupy-media` não dependem de Tauri nem umas das outras.
  `src-tauri` é a única cola.
- O contrato frontend ⇄ Rust é `docs/arquitetura/ipc.md`. Mudou o contrato, atualize o arquivo.
- Todo código que implementa um requisito leva `// @req RF-NN`; todo teste leva `// @test RF-NN`.
  Depois: `npm run docs:matriz`.
- Interface e documentação em pt-BR. Identificadores de código em inglês.
- Antes de encerrar: `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.

## Git

Detalhes em `docs/contribuicao/git.md`. Resumo:

- `main` é o publicado; `develop` é a integração. Nunca faça commit direto em nenhuma das duas.
- Trabalhe em `feat/<nome>`, `fix/<nome>`, `chore/<nome>`, `docs/<nome>`, `refactor/<nome>` ou
  `test/<nome>`, criado a partir de `develop`, e abra PR para `develop`. `develop` vai para `main`
  por PR de versão.
- Exceção: `hotfix/<nome>` nasce da `main`, volta para a `main` e depois é levado para `develop`.
- Commits e títulos de PR em Conventional Commits: `<tipo>(<escopo>): <descrição>`. Tipo e escopo
  em inglês, descrição em pt-BR, no imperativo, minúscula, sem ponto final. Escopos: `core`,
  `audio`, `media`, `shell`, `ui`, `ipc`, `db`, `docs`, `deps`. Mudança que quebra compatibilidade
  leva `!` e rodapé `BREAKING CHANGE:`. Requisitos no rodapé: `Refs: RF-NN`.
- PR só entra depois de revisado (skill `revisar-pr`). Squash para `develop`; merge commit de
  `develop` para `main`.

## Fim de sessão

1. Atualize `docs/arquitetura/arquitetura.json` se a estrutura mudou.
2. Rode `npm run docs:matriz`.
3. Salve um relatório em `docs/relatorios/` seguindo `docs/relatorios/README.md`.
