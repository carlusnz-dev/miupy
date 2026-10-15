---
name: briefing-frente
description: Monta o briefing de uma frente de trabalho do miupy para um agente (Sonnet no Orca ou Codex) com as regras fixas do projeto. Use na orquestração antes de soltar um agente.
---

# Briefing de frente

Escreva só o que é **desta frente**; as regras fixas vêm do bloco abaixo, copiado como está (troque
`<...>`). Salve em `~/orca/workspaces/miupy/.briefing-<frente>.md` e lance com o arquivo.

## Parte específica (você escreve)

- Branch `<tipo>/<nome>` (nome do assunto), worktree, requisitos (`RF-NN`), migração reservada
  (`docs/arquitetura/banco-de-dados.md`), seção do `ipc.md`.
- O problema visto pelo autor, com o cenário concreto (tela, passos, o que acontece).
- O resultado esperado e as decisões já fechadas pelo autor. O que estiver em aberto: "pergunte
  no relatório", nunca "decida".
- Arquivos prováveis (2 a 6) e o que já existe para reaproveitar.
- Commits esperados (5 a 10, um passo lógico cada).

## Regras fixas (copie)

```text
Leia AGENTS.md e use as skills do projeto: codigo-enxuto ao implementar, relatorio no fim,
revisar-design/revisar-playwright se mexer na interface.
Cargo: CARGO_BUILD_JOBS=<n> RUST_TEST_THREADS=<n> nice -n 10; nunca exporte RUSTFLAGS nem
CARGO_TARGET_DIR. Nos worktrees do Orca, node_modules é link somente leitura: não rode npm install. Não rode
npm run docs:matriz. // @req RF-NN no código, // @test RF-NN nos testes. pt-BR na interface e nos
docs; identificadores em inglês. Commits em Conventional Commits (tipo/escopo em inglês; descrição
em pt-BR no imperativo do projeto ("adiciona", "corrige"), minúscula, sem ponto, até 72 caracteres; escopos core, audio,
media, shell, ui, ipc, db, docs, deps). Antes de terminar, rode e registre: cargo fmt --all;
cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; npm run check;
npm test; npm run build; node scripts/rastreabilidade.mjs --check. Nada em segundo plano. Tarefa
grande demais: pare, registre e diga onde dividir.
```

### Sonnet no Orca (abre PR, revisa e mescla)

```text
Commite localmente. Push por HTTPS:
git -c url."https://github.com/".insteadOf="git@github.com:" -c credential.helper= -c credential.helper='!gh auth git-credential' push -u upstream <branch>
Confira o título: node scripts/ci/verificar-pr.mjs --head <branch> --base develop --titulo "<t>".
gh pr create --base develop. Depois lance UM subagente revisor (ferramenta Agent, model sonnet,
sem segundo plano) com a skill revisar-pr só até a decisão (passos 1 a 3), proibido de fazer merge/comentar/push: comentário e merge são seus; corrija os
achados e devolva ao mesmo revisor. Aprovado: gh pr comment com o parecer, espere
gh pr checks <n> --watch verde e gh pr merge <n> --squash. Nunca --admin, nunca force-push, nunca
fechar/reabrir PR; branch atrás da develop: git merge upstream/develop. Não apague o worktree nem o branch
(a orquestração apaga os dois). Termine com o número do PR e o estado.
```

### Codex

```text
O sandbox não commita: deixe tudo na árvore e ponha no relatório a seção "Fatias de commit"
(mensagem exata e arquivos de cada commit). Proibido push, PR, commit.
```
