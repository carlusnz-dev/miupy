---
name: codigo-enxuto
description: Escrever código no miupy com o menor diff e o menor gasto de tokens. Use antes de implementar qualquer coisa (função, componente, comando, teste) e ao revisar código próprio antes do commit.
---

# Código enxuto

Objetivo: resolver com o que já existe, no menor diff que se lê de uma vez.

## Antes de escrever

1. **Já existe?** Procure antes de criar, com `grep -rn` pelo nome provável e pelo comportamento:
   - Rust: `crates/miupy-core/src/` (domínio), `src-tauri/src/` (cola), `crates/miupy-media/src/`.
   - Svelte: `src/lib/components/`, `src/lib/stores/`, `src/lib/util/`, `src/lib/ipc/`.
   - Exemplos que já existem: `Resource` (cache por consulta), `Chunked` (listas longas),
     `SkeletonList`, `Modal`, `Toasts`/`notifyError`, `name_key` (chave sem caixa e acento),
     `request_limited` (ritmo de API), `FakeResolver` (testes do player).
2. **Precisa mesmo?** Se a mudança cabe numa função existente com um parâmetro a mais, não crie
   outra. Se só um lugar usa, não abstraia.
3. **Leia só o necessário:** `grep -n` para achar, depois `sed -n 'a,bp'` do trecho. Não leia
   arquivo inteiro de 800 linhas para mudar 5.

## Ao escrever

- Siga o estilo do arquivo vizinho (nomes, ordem, tratamento de erro).
- **Comentário só quando explica o porquê** que o código não mostra (regra de negócio, corrida,
  limite de API). Nada de comentário que repete o nome da função ou narra a linha.
- `// @req RF-NN` / `// @test RF-NN` são obrigatórios (rastreabilidade), não contam como ruído.
- Sem dependência nova se a biblioteca padrão ou uma já usada resolve. Precisando, `cargo add` /
  `npm install` com a versão atual, e diga por quê no relatório.
- Sem código morto, sem `TODO` sem dono, sem `console.log`/`dbg!` esquecido.
- Teste o comportamento, não a implementação; um teste que falharia se o código estivesse errado.

## Antes do commit

`git diff --stat` e `git diff`: cada linha precisa estar ali? Apague o que sobrou de tentativas.
Um commit por passo lógico.
