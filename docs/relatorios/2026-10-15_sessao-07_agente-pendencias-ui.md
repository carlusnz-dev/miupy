# Pendências de UI das sessões 05 e 06

- **Data:** 2026-10-15
- **Autor:** agente implementador `fix/pendencias-ui`
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** seis pendências só de frontend (`src/`), um commit por item.

## O que foi feito

1. **Seletor de perfil do EQ (Tocando):** novo `src/lib/util/dismiss.ts` (lógica sem DOM + action `dismissable`). O popup fecha com Escape ou clique fora; Escape, "Fechar" e a escolha de um perfil devolvem o foco ao botão. O botão virou alternador (clicar de novo fecha). Nas Configurações o seletor é um `<select>` nativo, que já fecha sozinho; nada a mudar lá.
2. **Svelte comprimido:** `AudioSection.svelte` e `NowPlaying.svelte` reformatados (funções e marcação em várias linhas), sem mudar comportamento.
3. **Esqueleto com consulta nova sem cache:** em `Resource.prime`, sem cache para a chave nova e `dataKey` diferente, `data`, `dataKey` e `error` são zerados, então `loading` volta a ser verdadeiro. Corrige Discoteca, Playlist e Artista de uma vez, sem cada tela comparar `dataKey`.
4. **Erro de atualização com cache:** `Resource.stale` e um toast informativo, uma vez por sequência de falhas (não repete a cada evento); os dados em cache ficam. Volta a `false` no próximo sucesso.
5. **Favorito otimista:** `stores/favorites.svelte.ts` guarda o valor pedido por faixa; `TrackRow`, `PlayerBar` e `NowPlaying` leem `favorites.is(track)`. `toggleFavorite` aplica na hora, reverte e mostra toast de erro se falhar. O valor otimista segue por 1,5 s após a confirmação, para não piscar até o `library://changed` atualizar a lista. Toque mais novo na mesma faixa prevalece sobre a falha do anterior.
6. **Menu lateral por teclado:** `createDeferredSave` (400 ms de pausa) em `util/sidebar.ts`; as teclas só pré-visualizam e a gravação ocorre na pausa, ao perder o foco ou ao desmontar. Arraste e duplo clique cancelam o que estava pendente.

## Decisões

- Corrigir o item 3 dentro do `Resource` em vez de nas telas; `Wrapped` continua com sua comparação de `dataKey` (inofensiva).
- Item 5: sobreposição por faixa em vez de mutar os objetos, porque as listas guardam `$state.raw`.
- Sem mudança de contrato IPC nem de Rust.
- `// @req` e `// @test` com RF-04, RF-29, RF-33 e RNF-11.

## Verificação (comandos rodados e resultado real)

- `npm run check`: 397 arquivos, 0 erros, 0 avisos.
- `npm test`: 16 arquivos, 72 testes, todos passando.
- `npm run build`: concluído.
- `node scripts/rastreabilidade.mjs --check`: ok, 50 requisitos, nenhum ID desconhecido.
- Não rodados: cargo (fora do escopo), `npm run docs:matriz` (só no branch de versão).
- Não testado em app real: os comportamentos de DOM (foco, clique fora, teclado) têm só a lógica coberta por teste; não há ambiente DOM no vitest.

## Pendências e problemas conhecidos

- Escape/clique fora e foco do botão no popup, e o debounce no `Nav.svelte`, precisam de uma conferência manual no app.
- O aviso de falha de atualização é um toast genérico; telas que quiserem um indicador fixo podem ler `Resource.stale`.

## Próximos passos

- Conferir manualmente os itens 1, 5 e 6 no app antes do merge.
