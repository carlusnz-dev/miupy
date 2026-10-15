---
name: desenhar-interface
description: Princípios de UI/UX para criar ou mudar telas e componentes do miupy no tema magma. Use antes de escrever Svelte novo ou mudar o visual de uma tela.
---

# Desenhar interface no miupy

Adaptado da skill `frontend-design` da Anthropic para um app que já tem identidade: aqui não se
inventa estilo, se **estende o magma** (`src/app.css`, cabeçalho "Design tokens magma").

## Identidade (não negociável)

- Basalto quase preto e quente (`--bg`, `--surface-1..3`), lava como único destaque
  (`--lava*`, `--grad-lava*`), flat: sem sombra, separação por borda de 1px (`--line`,
  `--line-strong`) e por superfície. Raio `--radius`. Cores sempre por token, nunca hex solto.
- Texto `--fg`, `--fg-muted`, `--fg-faint`; erro `--danger`, sucesso `--ok`.
- Um ponto ousado por tela (capa, cabeçalho do artista, gradiente da lava); o resto contido.

## Componentes

- Use os do app antes de criar: `src/lib/components/` (Modal, Toasts, Skeleton/SkeletonList,
  TrackRow, PageHeader, Cover, State, Icon…). Controle nativo com cara de sistema (select,
  checkbox cru) não entra: use/crie o componente no tema.
- Estados obrigatórios em toda tela com dados: esqueleto no formato do conteúdo (RNF-11), vazio
  que convida a agir, erro que diz o que houve e como resolver (sem pedir desculpas).
- O app não permite selecionar texto de interface (só campos de texto).

## Movimento

- Poucos e com propósito: abrir, expandir, confirmar. Molas do projeto (`src/lib/motion.ts`, primitivas
  existentes), não transições soltas.
- Listas: a entrada animada vale para as linhas visíveis de forma consistente (todas com a mesma
  animação até o limite, sem metade com mola e metade só com fade). Respeite
  `prefers-reduced-motion`.
- Nada que custe quadro: meta do RNF-13 (`docs/contribuicao/desempenho.md`).

## Layout e texto

- Alinhamento deliberado; controles de reprodução centrados; nada que quebre em mais de duas
  linhas sem motivo. Funciona de 960 px até tela cheia (e no Android, RNF).
- Foco visível no teclado, alvos de clique de 32 px ou mais, `aria-*` corretos.
- Texto em pt-BR, frase com inicial maiúscula, verbo que diz o resultado ("Salvar perfil", não
  "Enviar"); o mesmo nome do começo ao fim do fluxo.
- Evite: palavra destacada no título, rótulo em CAIXA ALTA espaçada sem função, cards todos iguais
  com sombra, setas enfeitando botões, numeração 01/02/03 sem sequência real.

Depois de mudar, rode `revisar-design` no diff.
