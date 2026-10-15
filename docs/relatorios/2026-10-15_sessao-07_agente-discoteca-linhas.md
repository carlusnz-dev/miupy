# Discoteca e linhas de faixa

- **Data:** 2026-10-15
- **Autor:** agente Sonnet no Orca (`feat/discoteca-linhas`)
- **Modelo:** Sonnet 5.5
- **Escopo:** link do artista, miniatura sem download, remover da discoteca, colunas extras e animação das linhas.

## O que foi feito

- **Contrato:** `Track.thumbnailUrl: string | null` (`model.rs`, `TRACK_COLUMNS`, `types.ts`, mock, `ipc.md`, `banco-de-dados.md`). Mudança aditiva.
- **Capa:** `Cover` ganhou `thumb`; usa `coverPath` e, sem ele, a miniatura (fallback roxo se a imagem falhar). Passado em `TrackRow`, `PlayerBar`, `MiniPlayer`, `NowPlaying` e `Downloads`. A CSP já permite `https:` em `img-src`; nada mudou.
- **Link do artista:** o nome vira `<a>` para `paths.artist`. O botão de tocar passou a envolver só capa (e o título repete o clique), pois link dentro de botão é inválido.
- **Remover:** `removeTracks` (`actions.ts`) confirma pelo `Modal` com caixa "Apagar também o arquivo baixado" (só se alguma faixa tem arquivo) e chama `library_delete_track`. Botão de lixeira na linha da Discoteca e "Remover" na barra de seleção. `TrackToolbar` não foi tocado.
- **Mais informações:** interruptor na Discoteca (`role="switch"`, salvo em `localStorage` com try/catch) mostra "adicionada há" (`util/age.ts`) e gêneros.
- **Animação:** as 14 primeiras linhas usam a mesma `spring-in` com atraso `índice × 20 ms` (inline); antes só as filhas 2, 3 e 4 de cada dez tinham atraso (`.motion-item:nth-child(10n+…)`). Da 15ª em diante continua sem animar (RNF-13).

## Decisões

- Lixeira só na Discoteca: nas playlists a ação da linha é "tirar da playlist", e outra lixeira confundiria.
- Atraso por índice em vez de `sibling-index()` do CSS, ainda sem suporte amplo.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: sem diferenças.
- `cargo clippy --workspace --all-targets -- -D warnings`: limpo.
- `cargo test --workspace`: todos passaram (core 99; shell 116 passaram, 2 ignorados).
- `npm run check`: 0 erros, 0 avisos. `npm test`: 75 passaram. `npm run build`: ok.
- `node scripts/rastreabilidade.mjs --check`: ok, 50 requisitos.
- `revisar-design`: feita pelo código, sem achados. **Não rodei `revisar-playwright`**: o visual (interruptor, linha com link, animação) não foi conferido no navegador.

## Pendências e problemas conhecidos

- Animação: não consegui reproduzir no app o sintoma "só as 3 primeiras animam"; a correção vem da leitura do CSS. Conferir no app real.
- Miniaturas remotas dependem de rede; sem rede cai no quadrado roxo.

## Próximos passos

- Conferir no navegador/app real o interruptor, o link e a animação; avaliar remover também nas playlists.
