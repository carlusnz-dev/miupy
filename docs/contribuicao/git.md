# Fluxo de git do miupy

Vale para pessoas e para agentes. O resumo está no [`AGENTS.md`](../../AGENTS.md); em caso de
dúvida, este arquivo é a referência.

## Branches

| Branch | Para quê | Nasce de | Vai para |
|---|---|---|---|
| `main` | O que está publicado. Sempre verde e sempre com tag de versão. | — | — |
| `develop` | Integração. Tudo que está pronto para a próxima versão. | `main` | `main` |
| `feat/<nome>` | Funcionalidade nova. | `develop` | `develop` |
| `fix/<nome>` | Correção de defeito que ainda não foi publicado, ou que pode esperar a próxima versão. | `develop` | `develop` |
| `chore/<nome>` | Manutenção sem efeito para o usuário: dependências, configuração, scripts. | `develop` | `develop` |
| `docs/<nome>` | Só documentação. | `develop` | `develop` |
| `refactor/<nome>` | Reorganização sem mudar comportamento. | `develop` | `develop` |
| `test/<nome>` | Só testes. | `develop` | `develop` |
| `hotfix/<nome>` | Correção urgente de algo que já está na `main`. | `main` | `main`, e depois de volta para `develop` |

Regras:

- Ninguém faz commit direto em `main` nem em `develop`. Toda mudança entra por pull request.
- Só `develop` e `hotfix/*` entram na `main`.
- `<nome>` em minúsculas, com hífens, curto e descritivo: `feat/recomendacoes`,
  `fix/duracao-m4a`. Se houver issue, o número vai na frente: `fix/42-duracao-m4a`.
- Um branch, um assunto. Se a mudança precisa de dois tipos, são dois branches.
- Branch integrado é apagado, no remoto e local, junto com o worktree dele.

```
main     ●───────────────────────────●──────────●   (tags v0.2.0, v0.2.1)
          \                         ╱ ╲        ╱
develop    ●────●─────────●────────●   ╲      ╱ ╲────●
                 \       ╱ \      ╱     hotfix/x      (volta para develop)
                 feat/a ╱   fix/b
```

## Mensagens de commit: Conventional Commits

Formato ([Conventional Commits 1.0.0](https://www.conventionalcommits.org/pt-br/v1.0.0/)):

```
<tipo>(<escopo opcional>): <descrição>

<corpo opcional: o porquê, não o como>

<rodapés opcionais>
```

| Tipo | Quando usar | Muda a versão |
|---|---|---|
| `feat` | Funcionalidade nova para o usuário. | minor |
| `fix` | Correção de defeito. | patch |
| `perf` | Melhora de desempenho sem mudar comportamento. | patch |
| `refactor` | Reorganização de código sem mudar comportamento. | não |
| `docs` | Só documentação. | não |
| `test` | Só testes. | não |
| `build` | Sistema de build e dependências (Cargo, npm, Tauri). | não |
| `ci` | Integração contínua. | não |
| `chore` | Manutenção que não cabe nos outros. | não |
| `style` | Formatação, sem mudança de código. | não |
| `revert` | Desfaz um commit anterior. | depende |

Regras do miupy:

- **Idioma:** tipo e escopo em inglês (são o padrão); descrição e corpo em pt-BR.
- **Descrição:** no imperativo, minúscula, sem ponto final, até 72 caracteres:
  `feat(core): adiciona ranking de recomendações`.
- **Escopo:** a parte do projeto. Use um destes: `core`, `audio`, `media`, `shell` (o
  `src-tauri`), `ui` (o `src/`), `ipc` (o contrato), `db` (migrações), `docs`, `deps`.
- **Mudança que quebra compatibilidade:** `!` depois do tipo ou do escopo e um rodapé
  `BREAKING CHANGE: <o que quebra e como migrar>`. Vale para o contrato de IPC, o esquema do banco
  sem migração automática e a API pública das crates.
- **Requisitos:** quando o commit implementa ou corrige um requisito, cite no rodapé:
  `Refs: RF-19, RF-20`.
- **Um commit, uma ideia.** Não misture `feat` com `refactor` no mesmo commit.
- **Coautoria de agente** vai no rodapé `Co-Authored-By:`.

Exemplos:

```
feat(media): resolve URL de áudio para tocar sem baixar

Refs: RF-18
```

```
fix(media): usa a URL do vídeo para obter o mix do YouTube

A forma playlist?list=RD<id> responde "unviewable"; watch?v=<id>&list=RD<id>
devolve as faixas parecidas.

Refs: RF-19
```

```
feat(ipc)!: devolve a fila como Track[] em player_get_state

BREAKING CHANGE: PlayerState.queue deixa de ser string[]. O frontend deve
ler queue[i].id.
```

## Pull requests

1. Branch a partir de `develop` (ou de `main`, se for `hotfix/*`).
2. Antes de abrir: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`; se tocou no frontend, `npm run check`, `npm test` e `npm run build`.
   Não gere a matriz de rastreabilidade no PR de trabalho: ela é gerada no branch de versão (ver
   "Matriz de rastreabilidade").
3. O **título do PR segue o mesmo formato de commit**, porque vira a mensagem do commit de
   integração: `feat(audio): toca M4A por streaming`.
4. Revisão antes do merge: quem revisa lê o diff inteiro e roda a verificação. Para agentes, é a
   skill `revisar-pr` (`.claude/skills/revisar-pr/`).
5. Merge:
   - `feat/*`, `fix/*`, `chore/*`, `docs/*`, `refactor/*`, `test/*` → `develop`: **squash**. Um
     commit por PR na `develop`, com o título do PR.
   - `develop` → `main`: **merge commit** (sem squash), para a `main` guardar o histórico da
     versão. Título: `chore(release): vX.Y.Z`.
   - `hotfix/*` → `main`: **squash**.

## Integração contínua

Os workflows ficam em `.github/workflows/`. Não há servidor nem nuvem: o "deploy" é publicar os
instaladores na release do GitHub.

| Workflow | Quando roda | O que faz |
|---|---|---|
| `ci.yml` | PR para `develop` ou `main`; push em `develop` e `main` | Os portões abaixo. |
| `release.yml` | push de tag `vX.Y.Z`; `workflow_dispatch` com a tag | Constrói os instaladores e publica a release. |

### Portões (`ci.yml`)

Os nomes dos jobs são os *checks* obrigatórios do ruleset (ver "Proteção de branches").

| Job | Roda | Falha quando |
|---|---|---|
| `rust` | sempre | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` ou `cargo test --workspace` falham. Testes `#[ignore]` (rede real, áudio) não rodam. Linux com as dependências do Tauri 2. |
| `frontend` | sempre | `npm run check`, `npm test` ou `npm run build` falham. |
| `docs` | sempre | há ID `@req`/`@test` que não existe nos requisitos; os testes de `scripts/ci/` falham; ou, no PR e no push para a `main`, a matriz está desatualizada (ver abaixo). |
| `regras-do-fluxo` | só em PR | o título não é Conventional Commits com os tipos e escopos acima; o branch de origem não é `<tipo>/<nome>`; o par origem/destino viola o fluxo (PR para `main` só de `develop` ou `hotfix/*`; PR para `develop` não vem de `main`, salvo o retorno de hotfix `chore: traz hotfix vX.Y.Z para develop`); no PR para `main`, as versões de `Cargo.toml`, `package.json` e `tauri.conf.json` não são iguais ou não são maiores que a última tag `v*`. O PR `develop` -> `main` precisa se chamar `chore(release): vX.Y.Z`. |

As regras ficam em `scripts/ci/regras.mjs` (testadas com `node --test scripts/ci/`). Para checar um
título antes de abrir o PR: `node scripts/ci/verificar-pr.mjs --head <branch> --base develop --titulo "<título>"`.

### Matriz de rastreabilidade

A matriz (`docs/rastreabilidade/`) é gerada uma vez por versão, no branch de versão
(`chore/release-vX.Y.Z`, passo 1 abaixo), com `npm run docs:matriz`. Nos PRs de trabalho para
`develop` ela pode ficar atrasada e ninguém a regenera; quem implementa só anota `@req`/`@test`.
O job `docs` cobra a matriz em dia só no PR e no push para a `main`.

- Até a sessão 06 havia um `matriz.yml` que commitava a matriz no branch de cada PR. O CI do
  commit do bot rodava por `workflow_dispatch`, que o ruleset nem sempre ligava ao PR, e os PRs
  ficavam bloqueados com tudo verde. Por isso nenhum bot commita mais em branch de PR.
- PR `hotfix/*` → `main` é cobrado como qualquer PR para a `main`: se o hotfix mexe em
  `@req`/`@test`, rode `npm run docs:matriz` no próprio branch. O retorno do hotfix para
  `develop` não é cobrado.
- `node scripts/rastreabilidade.mjs --check` continua útil localmente para achar ID inexistente
  (o job `docs` roda esse check em todo PR).

### Como lançar uma versão

1. Em um branch `chore/` a partir de `develop`, suba a versão nos três arquivos (`Cargo.toml`,
   `package.json`, `src-tauri/tauri.conf.json`) e o `Cargo.lock`, rode `npm run docs:matriz` e
   inclua a matriz; PR para `develop`.
2. PR `develop` -> `main` com o título `chore(release): vX.Y.Z`. O `regras-do-fluxo` confere as
   versões. Merge **commit** (sem squash).
3. Tag anotada na `main`, com as notas em pt-BR (seções Novidades, Correções, Conhecido; a
   primeira linha, `miupy vX.Y.Z`, não entra nas notas):
   ```sh
   git switch main && git pull upstream main
   git tag -a vX.Y.Z      # abre o editor: escreva as notas
   git push upstream vX.Y.Z
   ```
4. O `release.yml` confere que a tag é anotada, está na `main` e bate com as versões; cria a release
   em rascunho com as notas da tag (ou reaproveita uma release que já exista); constrói Linux
   (AppImage e .deb), Windows (NSIS e MSI) e macOS (.dmg universal); confere que os cinco tipos de
   instalador estão anexados e então publica a release como última versão (`latest`).
5. Para refazer: Actions > Release > Run workflow, informando a tag existente.

Pendências conhecidas: os instaladores não são assinados (sem certificado de código no Windows nem
notarização no macOS; o sistema avisa na instalação) e o Android não é construído.

### Proteção de branches

O ruleset do GitHub está descrito em [`protecao-de-branches.json`](protecao-de-branches.json):
`develop` e `main` sem push direto nem force-push, PR obrigatório, checks `rust`, `frontend`,
`docs` e `regras-do-fluxo`, merge commit e squash nas duas: squash nos PRs de trabalho para `develop`, merge commit no PR de versão para `main` e no retorno de hotfix para `develop`.

## Versões

- Versão semântica (`MAJOR.MINOR.PATCH`), a mesma em `Cargo.toml`, `package.json` e
  `src-tauri/tauri.conf.json`.
- Enquanto for `0.x`: `feat` sobe o minor, `fix` e `perf` sobem o patch, e mudança que quebra
  compatibilidade sobe o minor.
- Cada merge de `develop` na `main` recebe uma tag anotada `vX.Y.Z` na `main`. A tag dispara o
  workflow de release, que publica os instaladores (ver "Como lançar uma versão").

## Hotfix

1. `git switch -c hotfix/<nome> main`
2. Corrija, com teste que reproduz o defeito. Commit `fix(...)`.
3. PR para `main`, squash, tag com o patch seguinte.
4. Leve a correção para `develop` na mesma hora: PR de `main` para `develop` (merge commit),
   título `chore: traz hotfix vX.Y.Z para develop`. Sem isso, a próxima versão perde a correção.

## Histórico anterior a esta regra

Os commits até o PR #2 (sessão 02, 2026-10-13) não seguem o formato e entraram direto na `main`.
O histórico não foi reescrito: a `main` já estava publicada.
