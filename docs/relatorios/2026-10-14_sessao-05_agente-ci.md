# Integração contínua e release automática

- **Data:** 2026-10-14
- **Autor:** agente `ci` (orquestrado)
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** workflows de GitHub Actions (portões de PR, matriz automática, release com instaladores), scripts de regras do fluxo e documentação. Branch `chore/integracao-continua`, sem push nem PR.

## O que foi feito

- `scripts/ci/`: `regras.mjs` (título Conventional Commits com tipos e escopos do `git.md`, nome de branch, par origem/destino, versões), `verificar-pr.mjs`, `verificar-versoes.mjs` e `regras.test.mjs` (6 testes com `node --test`, sem dependências).
- `.github/workflows/ci.yml`: jobs `rust`, `frontend`, `docs`, `regras-do-fluxo`. `concurrency` cancela execuções antigas do mesmo PR; `permissions: contents: read`.
- `.github/workflows/matriz.yml`: gera a matriz em PR para `develop` de branch do próprio repositório e commita no branch.
- `.github/workflows/release.yml`: tag `vX.Y.Z` (ou `workflow_dispatch` com a tag) -> valida -> release em rascunho -> instaladores (Linux AppImage+deb, Windows NSIS+MSI, macOS dmg universal) -> confere os instaladores -> publica como `latest`.
- Documentação: seção "Integração contínua" em `docs/contribuicao/git.md`, `AGENTS.md`, skill `revisar-pr`, `docs/arquitetura/arquitetura.json` (entrada `ferramentas`) e `docs/contribuicao/protecao-de-branches.json` (ruleset descrito, não aplicado).

## Decisões

- **Checks obrigatórios:** `rust`, `frontend`, `docs`, `regras-do-fluxo`.
- **Nada no repositório assume sccache/mold:** não existe `.cargo/config.toml`; a configuração fica em `~/.cargo/config.toml` da máquina. O CI usa o toolchain estável puro e `Swatinem/rust-cache`.
- **Stub de `dist/`:** `tauri::generate_context!` exige `frontendDist` existente. O job `rust` cria `dist/index.html` mínimo em vez de depender do job `frontend`, para os dois rodarem em paralelo.
- **Matriz e GITHUB_TOKEN:** push do bot não dispara `pull_request`, então o commit da matriz ficaria sem checks e bloquearia o PR. Solução: depois do push, o `matriz.yml` roda `gh workflow run ci.yml --ref <branch>`; `workflow_dispatch` é a exceção que o GITHUB_TOKEN pode disparar, e os checks se anexam ao commit novo. O `ci.yml` aceita `workflow_dispatch`; `regras-do-fluxo` fica "skipped" (conta como aprovado). Sem laço: o push do bot não dispara o `matriz.yml`, e sem mudança não há commit. Para não ficar vermelho antes do bot, o `docs` só cobra "matriz em dia" fora do caso PR-do-repositório-para-develop (fork, PR para `main`, push e dispatch cobram). Nota: `scripts/rastreabilidade.mjs --check` só valida IDs, não frescor; o frescor é um `git diff --exit-code` depois de regenerar.
- **macOS universal:** um único .dmg serve Apple Silicon e Intel; custa compilar duas arquiteturas num job. Alternativa (dois jobs, dois .dmg) fica para quem preferir builds menores.
- **Release em rascunho até o fim:** o job `preparar` cria (ou reaproveita) a release; os três builds anexam usando `releaseId` (evita a corrida de três jobs criando a mesma release); o job `publicar` confere os cinco tipos de instalador e só então faz `--draft=false --latest`. Se a release já existia (feita à mão), os arquivos são anexados a ela e as notas dela não são tocadas.
- **Notas:** vêm da mensagem da tag anotada, sem a primeira linha (`miupy vX.Y.Z`). Tag sem mensagem falha.
- **Ações fixadas por SHA** com a versão no comentário (checkout 7.0.1, setup-node 7.1.0, rust-cache 2.9.2, tauri-action 1.0.0, rust-toolchain `stable`).
- **Ruleset:** `main` ficou com `merge` e `squash` permitidos, porque o ruleset não distingue a origem e o `git.md` manda squash em hotfix para `main`; "merge commit só de develop" é garantido pelo `regras-do-fluxo`. O retorno de hotfix (`main` -> `develop`) exige merge commit, que o `develop` (squash apenas) não aceita: anotado no JSON.
- Nada indicava deploy em nuvem ou OIDC no repositório; nada foi implementado nesse sentido.

## Verificação (comandos rodados e resultado real)

- `actionlint 1.7.12` (binário oficial, em diretório temporário) em `.github/workflows/*.yml`: sem erros nem avisos. `shellcheck` não está instalado, então os scripts `run:` não passaram por ele.
- `node --test scripts/ci/`: 6 testes, 6 aprovados (títulos bons e ruins, branches, fluxo de PR, versões, última tag por número).
- CLI com casos reais: `verificar-pr.mjs --head feat/x --base develop --titulo "feat(ui): ok"` -> ok (rc 0); `--head feat/x --base main --titulo "Feat"` -> 2 erros (rc 1); `verificar-versoes.mjs --maior-que-a-ultima-tag` no estado atual (0.2.1 = tag v0.2.1) -> erro, como esperado (rc 1); `--tag v0.2.1` -> ok; `--tag v0.2.2` -> erro.
- Extração das notas da tag testada num repositório temporário com tag anotada: saída correta, sem o título.
- `node scripts/rastreabilidade.mjs --check` não foi alterado.
- Não foram rodados: `cargo` (limite de CPU pedido), `npm run build` e qualquer workflow no GitHub.

## Pendências e problemas conhecidos

- **Só se prova rodando no GitHub:** instalação das dependências do Linux no `ubuntu-24.04` (lista tirada da documentação do Tauri 2 mais `libasound2-dev`); o stub de `dist/` satisfazer o `generate_context!`; `cargo clippy -D warnings` limpo no ambiente do CI (versão do toolchain estável pode trazer lints novos); o `gh workflow run` disparado pelo GITHUB_TOKEN anexar checks ao PR; "skipped" de `regras-do-fluxo` contar como aprovado no ruleset; AppImage com `NO_STRIP`; build do Windows e do macOS universal (o autor só usa Linux; `yt-dlp` do app e áudio nunca foram vistos nesses sistemas); `tauri-action` aceitar `releaseId` de rascunho.
- Se "Workflow permissions" do repositório estiver em somente leitura, o `permissions` explícito dos jobs ainda vale, mas a opção "Allow GitHub Actions to create and approve pull requests" não é necessária.
- Os instaladores **não são assinados**: sem certificado de código no Windows e sem assinatura/notarização no macOS; o sistema vai avisar na instalação. Não há atualizador automático.
- Android fora do CI e da release.
- O `matriz.yml` gera a matriz a partir do branch do PR (não do merge com a `develop`); dois PRs que mexem na matriz podem conflitar no segundo merge. A resolução está no `git.md`.
- `regras-do-fluxo` roda o script do próprio PR (não o da base); aceitável com um autor, mas um PR de fork poderia alterá-lo.
- Nada foi aplicado no GitHub: o ruleset, a proteção de branch e as configurações continuam como estavam.

## Próximos passos

- Abrir o PR, ver o primeiro ciclo real do `ci.yml` e ajustar o que só o GitHub mostra.
- Aplicar o ruleset de `docs/contribuicao/protecao-de-branches.json` depois que os checks existirem (o GitHub só oferece check já visto).
- Testar a release com uma tag de verdade (ou `workflow_dispatch`) antes da v0.3.0.
