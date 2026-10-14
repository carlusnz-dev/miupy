---
name: revisar-pr
description: Revisa um PR do miupy antes do merge. Lê o diff inteiro, roda a verificação do projeto no worktree do branch e só então aprova e faz o merge. Use quando um agente terminar uma frente, quando pedirem "revise o PR", "pode dar merge?" ou antes de qualquer merge na main.
---

# Revisar PR do miupy

O resumo do autor do PR não é prova. O merge só acontece depois de você ler o diff e rodar os
comandos. Argumento: número do PR ou nome do branch.

## 1. Situar

```sh
gh pr view <pr> --json number,title,headRefName,baseRefName,files,additions,deletions
git fetch upstream
git worktree list            # o branch do PR já tem worktree? rode a verificação lá
```

Se o branch estiver atrás da `main`, atualize antes de verificar (`git merge upstream/main` no
worktree do branch) e resolva os conflitos você. Verificar um branch desatualizado não prova nada
sobre o resultado do merge.

## 2. Ler o diff inteiro

```sh
gh pr diff <pr>
```

Confira, nesta ordem:

1. **Escopo.** O PR mexe só nos diretórios da frente dele? Arquivo fora do escopo precisa de
   motivo.
2. **Fronteiras.** `miupy-core`, `miupy-audio` e `miupy-media` não dependem de Tauri nem umas das
   outras (`Cargo.toml` de cada uma). Só `src-tauri` cola.
3. **Contrato.** O que mudou em comandos, argumentos, tipos ou eventos bate com
   `docs/arquitetura/ipc.md`? Se o contrato mudou, o arquivo mudou junto, e os dois lados
   (`src-tauri/src/commands/` e `src/lib/ipc/`) também.
4. **Banco.** Migração nova é um arquivo novo em `crates/miupy-core/migrations/`; migração já
   publicada não é editada. `docs/arquitetura/banco-de-dados.md` acompanha.
5. **Testes de verdade.** Teste que não falharia se o código estivesse errado não conta: asserção
   vazia, `#[ignore]` novo sem motivo, fixture ajustada para passar, teste apagado.
6. **Rastreabilidade.** Código de requisito com `// @req RF-NN`, teste com `// @test RF-NN`.
7. **Idioma.** Interface e documentação em pt-BR; identificadores em inglês.
8. **Decisões do autor do projeto.** Licença, fonte de biografia e foto de artista, e yt-dlp do
   sistema ou embutido não podem ser fechadas por um PR. Se o PR fecha alguma, pare e avise.
9. **Relatório.** Existe `docs/relatorios/AAAA-MM-DD_sessao-NN_agente-<nome>.md`, e a seção de
   verificação traz resultado real, com o que não foi rodado dito com clareza.

## 3. Rodar

No worktree do branch, com `export PATH="$HOME/.cargo/bin:$PATH"`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets      # 0 warnings
cargo test --workspace
```

Se o PR tocou em `src/`, `package.json` ou configuração do frontend:

```sh
npm ci            # só se node_modules não existir no worktree
npm run check
npm run build
npm test          # se o script existir
```

Se tocou em anotações `@req` / `@test`: `npm run docs:matriz` e confira se a matriz gerada está no
PR.

Anote o resultado real de cada comando (contagem de testes, falhas, ignorados). Teste de rede
ignorado é "não rodado", não "passou".

## 4. Decidir

- **Tudo certo:** comente no PR o que foi lido e o resultado dos comandos, e faça o merge:

  ```sh
  gh pr comment <pr> --body "<o que foi verificado e o resultado>"
  gh pr merge <pr> --merge --delete-branch
  ```

  Depois: `git pull` na `main`, rode `cargo test --workspace` de novo na `main` se outro PR entrou
  no meio, e remova o worktree integrado.

- **Problema pequeno e óbvio** (formatação, warning, anotação faltando): corrija no branch, faça o
  commit, rode o passo 3 de novo.

- **Problema de verdade:** não faça o merge. Devolva ao agente autor com o trecho do diff, o
  comando que falhou e a saída. Se o autor não existe mais, corrija você ou abra uma tarefa nova.

## 5. Relatar

Para cada PR, diga em separado o que foi **verificado rodando** e o que só foi **lido**. Não
escreva "testado" para algo que só compilou.
