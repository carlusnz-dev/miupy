# Título e artista dos vídeos do YouTube

- **Data:** 2026-10-15
- **Autor:** Codex
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** RF-09 e RF-12; parser, busca, yt-dlp e reprocessamento

## O que foi feito

- A separação agora escolhe o primeiro separador real entre hífen e travessões misturados. A comparação com o canal ignora caixa e acentos e aceita prefixo por palavra; `VEVO` e ` - Topic` já eram removidos.
- A busca e a inclusão na biblioteca passam os artistas locais ao parser puro para desempatar títulos quando o canal não ajuda. As participações extraídas de `feat.`/`ft.` entram no crédito exibido do artista.
- O processamento de JSON do yt-dlp aceita `artists` estruturado, mantém a prioridade de `track` e `artist`, e só infere álbum no segundo lado de duas partes quando ele tem pelo menos seis palavras: um lado curto pode ser o artista.
- Na primeira etapa, o reprocessamento consultava o MusicBrainz nas duas ordens do título original e escolhia a correspondência completa de maior score quando o artista salvo coincidia com o canal. A continuação abaixo acrescenta a exceção autorizada.

## Exceção para artista invertido

- Por decisão do autor, o lote também corrige `Negro Drama` como artista e `Racionais` como título quando o título do vídeo indica a inversão e o MusicBrainz confirma gravação completa, título e artista correspondentes, com score ≥ 90. Sem essa confirmação, a faixa fica intacta, inclusive sem usar o fallback do vídeo. A atualização manual fora do lote não usa a exceção.
- O provedor aceita correspondências completas a partir de 70; a exceção usa o limiar 90 autorizado pelo autor.
- O modelo `Track` não registra se título ou artista foram editados à mão; por isso não há como excluir essas faixas individualmente sem mudar o modelo ou o banco.

## Pesquisa e decisões

- O [MetadataFilter do Web Scrobbler](https://github.com/web-scrobbler/metadata-filter) remove ruído de vídeo como `Official Music Video` antes da separação; o [Web Scrobbler](https://github.com/web-scrobbler/web-scrobbler/wiki/Connector-API-changes) também documenta `processYtVideoTitle`. Mantive a limpeza existente e corrigi apenas a ordem e os créditos.
- O [yt-dlp](https://github.com/yt-dlp/yt-dlp/blob/master/README.md) distingue `track`, `artist`, `artists`, `creator` e `uploader`; o fallback documentado do metadata embed chega até uploader. `creator` e uploader não são prova de artista, por isso o parser continua conferindo o título.
- A [API de busca do MusicBrainz](https://musicbrainz.org/doc/MusicBrainz_API/Search) aceita campos `recording` e `artist` e retorna score. As duas ordens usam o provedor já limitado em taxa; não há rede no parser.
- `featured` permanece como campo estruturado no parser; `artist_credit()` monta `Artista feat. Convidado` para busca e yt-dlp. Não se perde participação do título.

## Casos de tabela e comportamento anterior

- **Falhavam antes:** `NEGRO DRAMA - RACIONAIS MC'S` com canal `Racionais`; `Kenny G - Matuê` com `MatueVEVO`; `Racionais - Negro Drama - Ao Vivo` (virava `Ao Vivo`); travessões misturados; artista local no lado direito com canal de terceiro; crédito `feat.` visível.
- **Já passavam antes:** `Racionais MC's - Negro Drama`, `Racionais - Negro Drama`, variante com `Clipe Oficial` e canal terceiro, `Matuê - Kenny G`, `Matuê - Topic`, título sem separador. Ficaram na tabela para impedir regressões; não seria correto afirmar que todos os exemplos falhavam antes.
- O teste do reprocessamento verifica três consultas (original e duas ordens) com artista salvo igual ao canal e uma consulta quando não é igual.

## Verificação

- `cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace -q`: passou na rodada final; 126 shell, 35 audio, 103 core, 88 media, um doctest; 15 ignorados no total.
- `npm run check`: falhou com `EROFS` ao gravar `node_modules/.vite-temp` (link somente leitura do worktree); repetição com `--config svelte.config.js` teve o mesmo erro.
- `npm test`: 22 arquivos e 98 testes passaram.
- `npm run build`: mesmo `EROFS`; `npm run build -- --configLoader runner` passou (213 módulos).
- `node scripts/rastreabilidade.mjs --check`: passou, 54 requisitos e nenhum ID desconhecido.
- **Continuação — exceção:** `cargo fmt --all`, `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings` e `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo test --workspace`: passaram. O teste completo inclui os novos casos de score forte, fraco, ausente, resultado só por título e faixa já correta.
- **Continuação — frontend:** `npm test`: 22 arquivos, 98 testes passaram. `npm run check` e `npm run build`: falharam com `EROFS` em `node_modules/.vite-temp`, link somente leitura deste worktree. `node scripts/rastreabilidade.mjs --check`: passou, 55 requisitos e nenhum ID desconhecido.

## Pendências e problemas conhecidos

- A busca do YouTube não consulta MusicBrainz para cada resultado; quando nem canal nem discoteca ajudam, mantém a ordem usual `Artista - Faixa`. Isso evita dezenas de consultas com limite de taxa durante uma busca.
- A autorização da exceção foi atendida na continuação desta frente; a falta de marca de edição manual no modelo limita sua proteção.
- Não foi feita validação visual no app. A rodada 3 consultou YouTube e MusicBrainz com rede real.

## Rodada 3: rede real

- As correções de busca e reprocessamento, junto do teste ignorado, já estavam na árvore sem commit ao início desta rodada. A prova real confirmou que funcionam; esta rodada acrescentou a tabela pura, executou as verificações e registrou os resultados.
- A busca ficou sem consulta ao MusicBrainz. Para desempatar `Negro drama - Racionais Mcs` com canal de terceiro, o parser reconhece `Racionais MC's` como artista de outro resultado da mesma busca. A comparação por palavras também permite que `Racionais TV` confirme `Racionais MC's` sem confundir `Negro Drama` com artista. O parser remove `(Letras)`, `(Letra)` e `(Lyrics)` como ruído.
- O reprocessamento consultou também `Negro Drama` como gravação e `Racionais` como artista, a ordem trocada dos campos salvos. A correção exigiu gravação completa, score ≥ 90 e artista do MusicBrainz igual ao artista salvo trocado ou começando por ele como palavra inteira.
- `integracao_real_titulo_e_artista` ficou permanente e ignorado por depender de yt-dlp, YouTube e MusicBrainz. Os asserts buscam cada `video_id`, sem depender da ordem. A tabela pura `resultados_reais_com_artistas_dos_outros_videos` cobre os três casos e as variantes de letra.
- Saída da prova real (`CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo test -p miupy --lib integracao_real_titulo_e_artista -- --ignored --nocapture`):

```text
BRUTO | tWSr-NDZI4s | Negro drama - Racionais Mcs | Andre Coutinho
BRUTO | mrAT_xG-opk | DVD - A Thousand Trouts, A Thousand Troubles - Negro Drama | Racionais TV
BRUTO | u4lcUooNNLY | Negro Drama - Nada Como Um Dia Após O Outro Dia (Chora Agora) | Racionais TV
BRUTO | DaCK8snx9ac | Racionais MC's - Negro Drama (Racionais 3 Décadas Ao Vivo) | Racionais TV
BRUTO | APvnqdcg8Z4 | Racionais Mc´s Nego Drama ( ao vivo) | MAMON666
BRUTO | dGjcBzD7z9Q | Negro Drama (Ao Vivo Na Quinta Da Boa Vista) | Seu Jorge
BRUTO | sSmrrVTv01Q | Edi Rock e Mano Brown - Negro Drama (Ao Vivo) | Edi Rock
BRUTO | vGAoGRq4oVI | Racionais Mc's - Negro Drama (Letras) | MiNi MaX
BRUTO | BFhhnHvfZmo | Negro Drama (Ao Vivo) | Racionais TV
BRUTO | 5Hp8v0ty1qQ | First Time Hearing Negro Drama - Racionais - English Lyrics - Reaction | NSP Reacts
META | u4lcUooNNLY | Some("Negro Drama - Nada Como Um Dia Após O Outro Dia (Chora Agora)") | Some("Racionais TV")
META | tWSr-NDZI4s | Some("Negro drama - Racionais Mcs") | Some("Andre Coutinho")
BUSCA | tWSr-NDZI4s | Negro drama | Racionais Mcs
BUSCA | mrAT_xG-opk | A Thousand Trouts, A Thousand Troubles - Negro Drama | DVD
BUSCA | u4lcUooNNLY | Negro Drama | Racionais
BUSCA | DaCK8snx9ac | Negro Drama | Racionais MC's
BUSCA | dGjcBzD7z9Q | Negro Drama (Ao Vivo Na Quinta Da Boa Vista) | Seu Jorge
BUSCA | sSmrrVTv01Q | Negro Drama (Ao Vivo) | Edi Rock e Mano Brown
BUSCA | APvnqdcg8Z4 | Racionais Mc´s Nego Drama | MAMON666
BUSCA | vGAoGRq4oVI | Negro Drama | Racionais Mc's
BUSCA | LEVLz2WcTdE | Nego Drama - AO VIVO - #LIVERACIONAIS 2021 | Racionais Mc's
BUSCA | BFhhnHvfZmo | Negro Drama (Ao Vivo) | Racionais
ANTES | Racionais | Some("Negro Drama")
DEPOIS | Negro Drama | Some("Racionais MC's") | Some("1000 trutas, 1000 tretas")
test state::tests::integracao_real_titulo_e_artista ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 131 filtered out; finished in 22.42s
```

- Verificação final: `cargo fmt --all` passou; `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings` passou; `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo test --workspace` passou (129 shell, 35 audio, 103 core, 89 media e 1 doctest; testes de rede ignorados por padrão). `npm test` passou (22 arquivos, 98 testes). `node scripts/rastreabilidade.mjs --check` passou (55 requisitos, nenhum ID desconhecido). `npm run check` e `npm run build` falharam com `EROFS` em `node_modules/.vite-temp`, link somente leitura do checkout principal. `npm run build -- --configLoader runner` passou (214 módulos), contornando a escrita do carregador padrão.

## Fatias de commit

Sem commits nesta frente, conforme o sandbox. Fatias sugeridas, nesta ordem:

1. `fix(media): normaliza títulos e identifica artistas` — `crates/miupy-media/src/title.rs`, `crates/miupy-media/src/lib.rs`, `crates/miupy-media/src/ytdlp.rs`, `src-tauri/src/youtube.rs` (implementação e testes locais desses arquivos). Rodada 3: tabela dos títulos reais em `title.rs`.
2. `fix(shell): corrige campos invertidos com confirmação forte` — `src-tauri/src/metadata.rs` (implementação e testes locais).
3. `test(shell): comprova títulos e artista com rede real` — `src-tauri/src/state.rs` (teste ignorado por padrão).
4. `docs(docs): registra a correção de título e artista` — `docs/relatorios/2026-10-15_sessao-09_agente-titulo-artista.md`.

## Rodada 4: achados do PR #81

- A troca pela discoteca agora exige coincidência inteira do lado direito, rejeita prefixo de artista à esquerda e preserva lados esquerdos com aparência de nome próprio composto. Assim, `Seu Jorge - Michael Jackson` e `Queen Of The Night - Queen` mantêm a ordem usual. Essa guarda é conservadora: alguns títulos capitalizados que realmente venham em ordem invertida permanecerão na ordem usual sem confirmação pelo canal.
- Prefixo de artista exige duas palavras; `Racionais TV` ainda confirma `Racionais MC's` porque `TV` é sufixo genérico de canal. Um canal de uma palavra, como `Racionais`, sozinho deixa de confirmar o nome mais longo. O reprocessamento pode recuperar a inversão quando o título bruto e uma gravação completa do MusicBrainz a confirmam com score ≥ 90.
- A busca e a inclusão usam só artistas extraídos dos títulos de pelo menos dois outros resultados do mesmo lote. Cada resultado guarda suas pistas com o cache; resultados de outras buscas e nomes de canal não entram na votação. Um cover isolado acompanhado de um react com canal do artista não troca a ordem.
- Erro de rede nas consultas extras ao MusicBrainz registra aviso e preserva a primeira resposta; o `video_metadata` é obtido uma vez por faixa e reaproveitado.

### Verificação da Rodada 4

- `integracao_real_titulo_e_artista` (ignorado por padrão, executado com `--ignored --nocapture`): 1 passou, 0 falhou. Busca: `tWSr-NDZI4s` → `Negro drama` / `Racionais Mcs`; `DaCK8snx9ac` → `Negro Drama` / `Racionais MC's`; `vGAoGRq4oVI` → `Negro Drama` / `Racionais Mc's`; `u4lcUooNNLY` → `Negro Drama` / `Racionais`. Reprocessamento: `Racionais` / `Negro Drama` → `Negro Drama` / `Racionais MC's`, álbum `1000 trutas, 1000 tretas`. A composição dos dez resultados do YouTube mudou, mas esses IDs e o reprocessamento mantiveram os resultados anteriores.
- `cargo fmt --all`: passou. `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou. `CARGO_BUILD_JOBS=6 RUST_TEST_THREADS=6 nice -n 10 cargo test --workspace -q`: passou (132 shell, 35 audio, 103 core, 90 media, 1 doctest; 16 testes ignorados).
- `npm test`: 23 arquivos, 102 testes passaram. `node scripts/rastreabilidade.mjs --check`: passou, 56 requisitos, nenhum ID desconhecido.
- `npm run check` e `npm run build`: falharam com `EROFS` na escrita de `node_modules/.vite-temp` por ser link somente leitura do worktree. `npm run build -- --configLoader runner`: passou, 215 módulos.

### Fatias de commit da Rodada 4

Nenhum commit, push ou alteração no PR foi feito. Fatias sugeridas (cada uma com `Refs: RF-09 RF-12` no rodapé):

1. `fix(media): restringe a inversão de título e artista` — `crates/miupy-media/src/title.rs`, `crates/miupy-media/src/ytdlp.rs`.
2. `fix(shell): isola pistas por busca e exige repetição` — `src-tauri/src/youtube.rs`.
3. `fix(shell): preserva resposta inicial do musicbrainz` — `src-tauri/src/metadata.rs`, `src-tauri/src/downloads.rs`, `docs/relatorios/2026-10-15_sessao-09_agente-titulo-artista.md`.
