# miupy — guia para agentes

Player de música offline (Rust + Tauri 2, desktop e Android). Leia antes de mexer:
`docs/requisitos/requisitos.md`, `docs/arquitetura/arquitetura.md`, `docs/arquitetura/ipc.md`,
`docs/stacks/stacks.md`.

## Ambiente

- Use o Rust do rustup: `export PATH="$HOME/.cargo/bin:$PATH"` (o `~/.zshenv` já faz isso; o
  `/usr/bin/cargo` do Arch é outro build e invalida o `target/` inteiro).
- Compilação: sccache + clang/mold em `~/.cargo/config.toml`, `jobs = 6`. Worktree novo:
  `scripts/semear-target.sh <worktree>` antes de compilar. Não exporte `RUSTFLAGS` nem
  `CARGO_TARGET_DIR`. Detalhes em `docs/contribuicao/compilacao.md`.
- Gerenciador de pacotes JS: `npm`. CLI do Tauri: `npx tauri`.
- Adicione dependências com `cargo add` / `npm install` (versão atual), não chute versões.

## Regras

- `crates/miupy-core`, `miupy-audio` e `miupy-media` não dependem de Tauri nem umas das outras.
  `src-tauri` é a única cola.
- O contrato frontend ⇄ Rust é `docs/arquitetura/ipc.md`. Mudou o contrato, atualize o arquivo.
- Todo código que implementa um requisito leva `// @req RF-NN`; todo teste leva `// @test RF-NN`.
  A matriz (`npm run docs:matriz`) só é gerada no branch de versão; nos PRs de trabalho não rode
  (o `node scripts/rastreabilidade.mjs --check` local continua útil para achar ID inexistente).
- Interface e documentação em pt-BR. Identificadores de código em inglês.
- Antes de encerrar: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (o CI roda os mesmos, com `--locked`).

## Skills do projeto

Em `.claude/skills/`: `revisar-pr` (antes de todo merge), `codigo-enxuto` (antes de implementar),
`desenhar-interface` e `revisar-design` (interface no tema magma), `revisar-playwright`
(navegador pelo MCP do `.mcp.json`, sem prints), `relatorio` (fim de frente ou sessão) e
`briefing-frente` (orquestração, ao soltar um agente). `relatorio` cobre o passo 3 de "Fim de sessão".

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
- O CI (`.github/workflows/`, ver "Integração contínua" em `docs/contribuicao/git.md`) barra PR com
  `rust`, `frontend`, `docs` ou `regras-do-fluxo` vermelho. Versão sai por tag anotada `vX.Y.Z` na
  `main`; o `release.yml` publica os instaladores.
- PR só entra depois de revisado (skill `revisar-pr`). Squash para `develop`; merge commit de
  `develop` para `main`.

## Fim de sessão

1. Atualize `docs/arquitetura/arquitetura.json` se a estrutura mudou.
2. A matriz só é gerada no branch de versão (ver "Como lançar uma versão" em `docs/contribuicao/git.md`).
3. Salve um relatório em `docs/relatorios/` seguindo `docs/relatorios/README.md`.
