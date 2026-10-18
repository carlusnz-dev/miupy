# Ações da playlist redesenhadas (Tocar em destaque, menu Mais, modal de detalhes)

- **Data:** 2026-10-17
- **Autor:** Sonnet 5.5 (Claude Code no Orca), frente `feat/playlist-acoes`
- **Modelo:** claude-sonnet-5-5
- **Escopo:** só frontend. Cabeçalho de `PlaylistDetail.svelte` trocado de oito botões de texto por Tocar + Aleatório + Baixar + menu Mais. Sem requisito novo.

## O que foi feito

1. `feat(ui): adiciona o menu de mais opções no tema` — `Menu.svelte`: botão `…` com `role="menu"`, itens `menuitem`/`menuitemcheckbox`, setas, Home/End, inicial, Enter/Espaço, Escape e clique fora (`dismissable`); o foco vai ao menu ao abrir e volta ao botão ao fechar. Reaproveita `listboxKey` e `springTransition`.
2. `feat(ui): junta nome e capa da playlist no modal de editar detalhes` — `PlaylistForm.svelte` ganhou miniatura 96 px com Escolher capa e Remover capa ao lado do nome e da descrição. A capa fica pendente até **Salvar** (Cancelar descarta) e o erro de imagem aparece junto da capa. A lista de Playlists usa o mesmo modal ("Editar detalhes de X").
3. `feat(ui): redesenha as ações da playlist com tocar em destaque` — fileira abaixo do cabeçalho: Tocar/Pausar/Continuar redondo de 48 px só com ícone (`aria-label` muda), Aleatório (`player.playShuffled`), Baixar com estado e `…`. Menu: Editar detalhes, Baixar automaticamente (marcado), Marcar como favoritas (só se não é a de favoritas) e, separado, Apagar playlist em `--danger`. A capa grande abre o modal. "Playlists" virou link de voltar acima do cabeçalho. Alvos de 48 px no celular, 40 px no desktop, uma linha só.
4. `test(ui): cobre o tocar aleatório da playlist` — dois testes de `player.playShuffled`.

## Decisões

- **Baixar automaticamente (RF-31)** não estava no desenho e o checkbox cru não cabe na fileira nova: virou item marcável do menu Mais.
- **Voltar:** segui `Artist.svelte` (link pequeno "← Playlists" acima do cabeçalho), não à esquerda do título; o cabeçalho tem menu aberto para baixo e o mesh tem `overflow-hidden`, por isso a fileira de ações fica fora do mesh.
- **Aleatório:** `playShuffled` liga `random` só se estava `off` (mantém Surfle) e começa por uma faixa sorteada; o backend já embaralha a fila nova mantendo a primeira (`ipc.md`, "Aleatório e Surfle"). Contrato inalterado.
- **Progresso do Baixar:** sem indicador por playlist no app; usei `downloads.active` cruzado com as faixas da lista. Com jobs ativos o ícone fica lava e o rótulo diz "Baixando N faixas".
- Sem vidro: o menu usa `bg-s2` + borda, como o `Dropdown`. Não toquei em `src/app.css`.

## Verificação

- `npm run check`: 421 arquivos, 0 erros, 0 avisos.
- `npm test`: 24 arquivos, 110 testes passaram.
- `npm run build`: concluído.
- `node scripts/rastreabilidade.mjs --check`: ok, 59 requisitos, nenhum ID desconhecido.
- `revisar-design` pelo código: sem cor literal nova, sem controle cru, Escape/foco tratados.
- Cargo: não rodado (frente só de frontend).

## Pendências e problemas conhecidos

- **Não conferido no navegador:** as ferramentas do Playwright MCP não estavam disponíveis nesta sessão. Falta ver alinhamento, o menu aberto (posição, `right-0` no celular estreito) e a quebra em 360 px.
- Não há testes de DOM no projeto (sem jsdom): o menu e o modal só têm cobertura da lógica de teclado já existente (`listboxKey`) e do `playShuffled`.
- Vidro no menu fica para depois de `feat/vidro-base`.
