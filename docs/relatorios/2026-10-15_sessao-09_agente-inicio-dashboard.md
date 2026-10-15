# Início em painel

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RF-14, RF-41 e RF-42 no frontend.

## O que foi feito

- O `DashCard` usa o `MeshGradient` estático, acende por opacidade durante 1,5 s antes de navegar e abre imediatamente com movimento reduzido. O link funciona por clique, Enter e Espaço; ações internas executam de imediato.
- O cartão principal mostra playlist ativa com capa ou mosaico, contexto e faixa atual. Sem reprodução, oferece as faixas recentes. As cores da capa tingem o mesh quando a extração da paleta funciona.
- Estatísticas, playlists, mais tocadas e os quatro atalhos passaram a cartões. Há esqueletos, estados vazios e erro de consulta.
- A playlist atual mostra Pausar quando toca e Continuar quando está pausada; as outras mantêm Tocar.
- RF-41 foi descrito nos requisitos; a arquitetura registra o novo componente do frontend.

## Decisões

- O cartão de playlists abre a lista completa, e o de mais tocadas abre Estatísticas. Isso preserva uma única área de clique por cartão, inclusive no teclado.
- A navegação usa um link sobre o conteúdo e um botão de reprodução separado para evitar controles interativos aninhados.
- Não alterei `MeshGradient.svelte`, `NowPlaying.svelte` nem `App.svelte`.
- O contrato IPC não mudou. Não há animação contínua nem transição de filtro, sombra ou blur.

## Verificação (comandos rodados e resultado real)

- `npm run check`: falhou por `EROFS` ao gravar `node_modules/.vite-temp` no link somente leitura do worktree. `npm run check -- --config /tmp/miupy-svelte-check.config.mjs`: passou, 0 erros e 0 avisos; a configuração temporária omite apenas o pré-processamento de CSS.
- `npm test`: 22 arquivos, 98 testes passaram.
- `npm run build`: falhou pelo mesmo `EROFS`. `npm run build -- --configLoader runner`: passou, 214 módulos transformados.
- `node scripts/rastreabilidade.mjs --check`: passou, 55 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou. `docs/arquitetura/arquitetura.json`: JSON válido.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`: não rodados; frente só de frontend, e o worktree não tem `target/`, conforme o briefing. Registrar também no futuro PR.
- `revisar-design`: revisão do código confirmou tokens magma, link com foco visível, acesso por teclado, estados de dados e movimento apenas temporário. O MCP do Playwright não está disponível nesta sessão; `revisar-playwright` não rodado.

## Pendências e problemas conhecidos

- Não foi observado o DOM, a duração percebida de 1,5 s, a paleta das capas nem o desempenho da GPU no app real. O mock e a janela real ainda precisam de revisão visual.
- **Pergunta para o autor:** o destaque de 1,5 s parece lento no uso? Mantive a duração decidida nesta sessão.
- Nenhum commit, push ou PR foi criado, conforme a restrição do sandbox.

## Fatias de commit

1. `feat(ui): adiciona cartão de mesh com destaque temporário` — `src/lib/components/DashCard.svelte`, regras `.dash-card` em `src/app.css`. Rodapé: `Refs: RF-41`.
2. `feat(ui): mostra playlist e faixa atual no início` — trecho principal de `src/routes/Home.svelte`. Rodapé: `Refs: RF-41 RF-42`.
3. `feat(ui): reúne estatísticas em cartão` — trecho de estatísticas de `src/routes/Home.svelte`. Rodapé: `Refs: RF-14 RF-41`.
4. `feat(ui): mostra playlists e mais tocadas no painel` — trechos das listas de `src/routes/Home.svelte`. Rodapé: `Refs: RF-14 RF-41`.
5. `feat(ui): organiza atalhos em cartões` — trecho de atalhos de `src/routes/Home.svelte`. Rodapé: `Refs: RF-41`.
6. `feat(ui): alterna tocar e pausar na playlist atual` — `src/routes/PlaylistDetail.svelte`. Rodapé: `Refs: RF-42`.
7. `docs(docs): registra início em painel` — `docs/requisitos/requisitos.md`, `docs/arquitetura/arquitetura.json`. Rodapé: `Refs: RF-41`.
8. `docs(docs): relata frente do início em painel` — este relatório. Rodapé: `Refs: RF-14 RF-41 RF-42`.

## Próximos passos

- Conferir no app real a duração do destaque e a fluidez da transição de cor; ajustar só após retorno do autor.
