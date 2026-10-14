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
2. Antes de abrir: `cargo fmt --all`, `cargo clippy --workspace --all-targets`,
   `cargo test --workspace`; se tocou no frontend, `npm run check`, `npm test` e `npm run build`.
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

## Versões

- Versão semântica (`MAJOR.MINOR.PATCH`), a mesma em `Cargo.toml`, `package.json` e
  `src-tauri/tauri.conf.json`.
- Enquanto for `0.x`: `feat` sobe o minor, `fix` e `perf` sobem o patch, e mudança que quebra
  compatibilidade sobe o minor.
- Cada merge de `develop` na `main` recebe uma tag anotada `vX.Y.Z` na `main`.

## Hotfix

1. `git switch -c hotfix/<nome> main`
2. Corrija, com teste que reproduz o defeito. Commit `fix(...)`.
3. PR para `main`, squash, tag com o patch seguinte.
4. Leve a correção para `develop` na mesma hora: PR de `main` para `develop` (merge commit),
   título `chore: traz hotfix vX.Y.Z para develop`. Sem isso, a próxima versão perde a correção.

## Histórico anterior a esta regra

Os commits até o PR #2 (sessão 02, 2026-10-13) não seguem o formato e entraram direto na `main`.
O histórico não foi reescrito: a `main` já estava publicada.
