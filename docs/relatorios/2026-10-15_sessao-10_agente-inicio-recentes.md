# Início: recentes e reprodução das mais tocadas

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** RF-14 e RF-41 no painel do Início

## O que foi feito

- O cartão **Adicionadas recentemente** volta ao painel com as seis faixas já consultadas pelo Início. Cada linha toca a faixa com fila formada pelas seis recentes.
- As três faixas exibidas em **Mais tocadas** viraram botões de reprodução. A fila contém somente essas três, na ordem mostrada.
- A ação principal do `DashCard` saiu do posicionamento absoluto e ocupa o fluxo normal. Tocar uma linha acende o cartão por 1,5 s e não navega.

## Decisões

- O cartão de recentes fica abaixo de Playlists/Mais tocadas e usa duas colunas a partir de `xl`; em larguras menores, uma coluna preserva títulos legíveis. A quantidade segue as seis faixas do Início antigo.
- Reutilizei `statsLibrary` e `libraryListTracks`; sem IPC, migração, dependência ou mudança de estrutura da arquitetura.
- As linhas usam botões dentro do slot de ação do `DashCard`, acima do link de navegação, para manter reprodução imediata e acesso por teclado.

## Verificação (comandos rodados e resultado real)

- `npm run check`: falhou por EROFS em `node_modules/.vite-temp` durante o pré-processamento; informou 2 erros desse tipo e 0 avisos. Repetição com configuração temporária teve o mesmo EROFS; arquivo temporário removido.
- `npm run build -- --configLoader runner`: passou, 214 módulos transformados.
- `npx vitest run --maxWorkers=2`: falhou ao carregar a configuração pelo mesmo EROFS.
- `npm test -- --maxWorkers=2 --configLoader runner`: passou, 22 arquivos e 98 testes.
- `node scripts/rastreabilidade.mjs --check`: passou, 55 requisitos, nenhum ID desconhecido.
- `git diff --check`: passou.
- `cargo fmt`, `cargo clippy` e `cargo test`: não rodados, conforme exceção desta frente sem alterações Rust.
- `npm run docs:matriz`: não rodado, conforme regra da branch de versão.

## Revisão de design

- Diff revisado com `revisar-design`: só tokens e componentes existentes; botões com foco global visível e alvos de 48 px; esqueleto e vazio presentes; entrada de linhas usa `enterStyle` já existente. Nenhum achado pendente pelo código.
- `revisar-playwright` não foi necessário para o comportamento verificável pelo código. Layout e áudio reais não foram vistos em janela do app.

## Pendências e problemas conhecidos

- Não há teste de componente do Início nem infraestrutura de DOM neste checkout. Não criei teste que só espelhasse a marcação; a interação deve ser conferida no app real.
- **Pergunta ao autor:** as seis recentes em cartão largo, abaixo de Playlists/Mais tocadas, têm a densidade e posição desejadas? A fila já cobre as seis.

## Fatias de commit

1. `feat(ui): volta as adicionadas recentemente ao início` — `src/routes/Home.svelte` (cartão de recentes); rodapé `Refs: RF-41`.
2. `feat(ui): toca direto das mais tocadas no início` — `src/routes/Home.svelte` (linhas de mais tocadas); rodapé `Refs: RF-14, RF-41`.
3. `refactor(ui): tira o posicionamento absoluto do botão do cartão` — `src/lib/components/DashCard.svelte`, `src/routes/Home.svelte` (ação principal); rodapé `Refs: RF-41`.
4. `docs(docs): registra a frente de início recentes` — este relatório.

## Próximos passos

- Conferir a densidade do cartão e a reprodução no app real; ajustar a posição se o autor preferir outra ordem.
