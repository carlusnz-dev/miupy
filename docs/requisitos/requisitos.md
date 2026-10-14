# Requisitos do miupy

Fonte única dos requisitos. A [matriz de rastreabilidade](../rastreabilidade/matriz.md) é gerada
a partir das tabelas deste arquivo: cada linha cujo primeiro campo é um ID (`RF-NN` / `RNF-NN`)
vira uma linha da matriz.

Prioridade: **M** (must, base do produto), **S** (should), **C** (could).

## Requisitos funcionais

| ID | Requisito | Prioridade | Descrição |
|---|---|---|---|
| RF-01 | Reprodução | M | Tocar, pausar e retomar uma faixa local. |
| RF-02 | Volume | M | Aumentar, diminuir e silenciar o volume. |
| RF-03 | Fila e navegação | M | Fila de reprodução, próxima/anterior e *seek* dentro da faixa. |
| RF-04 | Favoritos | M | Adicionar e remover faixas das favoritas. |
| RF-05 | Metadados da faixa | M | Exibir título, artista, álbum, duração, capa e demais metadados da faixa. |
| RF-06 | Playlists | M | Criar, renomear, excluir playlists e adicionar/remover faixas escolhidas pelo usuário. |
| RF-07 | Ordenação | M | Ordenar as faixas de uma playlist (título, artista, duração, data, posição manual). |
| RF-08 | Pesquisa local | M | Pesquisar faixas dentro de uma playlist e na biblioteca. |
| RF-09 | Pesquisa no YouTube | M | Pesquisar músicas no YouTube e adicioná-las à biblioteca. |
| RF-10 | Download de selecionadas | M | Baixar para uso offline as faixas selecionadas. |
| RF-11 | Download de playlist | M | Baixar uma playlist inteira de uma vez. |
| RF-12 | Metadados via API | S | Enriquecer faixa e artista com dados de APIs externas (MusicBrainz, Cover Art Archive, LRCLIB). |
| RF-13 | Registro de atividade | M | Registrar ações do usuário: faixa tocada, tempo efetivamente ouvido, pulos, adições. |
| RF-14 | Estatísticas da discoteca | S | Totais da discoteca: faixas, playlists, artistas, tempo ouvido, mais tocadas. |
| RF-15 | Retrospectiva anual | S | "Anual" do usuário: top faixas/artistas, minutos ouvidos, por mês, no estilo do Wrapped. |
| RF-16 | Perfil | M | Gerenciar perfil: nome, nome de usuário, e-mail e telefone. |
| RF-17 | Tema da lava | M | Trocar a cor da lava entre laranja/vermelho (padrão), roxo e azul. |
| RF-18 | Tocar sem baixar | S | Tocar uma faixa do YouTube que ainda não foi baixada, por streaming, quando há rede. Sem rede, o erro é claro e a discoteca baixada continua funcionando. |
| RF-19 | Recomendações | S | Sugerir faixas para adicionar à discoteca ou a uma playlist a partir do que o usuário ouviu: tempo ouvido, conclusões, pulos, favoritos, artistas e gêneros. A pontuação é calculada localmente, no SQLite. |
| RF-20 | Metadados da faixa no banco | S | Gêneros, ano de lançamento e identificadores externos ficam gravados na tabela de faixas, para alimentar as recomendações sem rede. |
| RF-21 | Foto do perfil | S | Escolher, trocar e remover a foto do usuário no perfil. |
| RF-22 | Capa da playlist | S | Escolher uma capa para a playlist; sem capa escolhida, a playlist mostra um mosaico dos 4 artistas com mais faixas nela. |
| RF-23 | Sugestões de busca | C | Sugerir o texto da busca no YouTube enquanto o usuário digita. |
| RF-24 | Aleatório e *smart random* | S | Tocar a fila em ordem aleatória, ou em sorteio ponderado que puxa para as faixas mais bem pontuadas e próximas da que está tocando. |
| RF-25 | Pontuação e DNA | S | Mostrar a pontuação de cada faixa e uma tela "DNA" com o estilo atual do usuário: artistas, gêneros, taxa de pulos, o que subiu e o que caiu. |
| RF-26 | yt-dlp gerenciado pelo app | S | O app baixa, confere e atualiza o yt-dlp sozinho, sem depender de instalação no sistema. |
| RF-27 | Foto de artista | S | Mostrar a foto do artista, vinda do Wikimedia Commons pelo Wikidata, com o crédito de autor e licença. |

## Requisitos não funcionais

| ID | Requisito | Prioridade | Descrição |
|---|---|---|---|
| RNF-01 | Banco local SQLite | M | Toda a persistência é local, em SQLite, com migrações versionadas. |
| RNF-02 | Offline-first | M | Tudo que já foi baixado funciona sem rede; rede só para busca, download e enriquecimento. |
| RNF-03 | Multiplataforma | M | Roda nativamente em desktop (Linux, Windows, macOS) e Android, via Rust + Tauri v2. |
| RNF-04 | Interface flat "magma" | M | Visual flat e simples, sem sombras pesadas nem glass/neumorfismo, com bordas de cor fraca. |
| RNF-05 | Gradientes | S | Gradientes bonitos, alguns em *mesh* animado, respeitando `prefers-reduced-motion`. |
| RNF-06 | Documentação viva | M | `docs/` com README por pasta, arquitetura, stacks, requisitos, relatórios de sessão, matriz gerada e JSON da arquitetura. |
| RNF-07 | Integrações externas | C | Núcleo desacoplado da UI para permitir integrar serviços externos do usuário no futuro. |
| RNF-08 | Animações | S | Movimento com física de mola (*spring*): entrada, saída, bounce, hover e fade-in/out, sempre respeitando `prefers-reduced-motion`. |
| RNF-09 | Aceleração de hardware | S | A interface é composta pela GPU quando o sistema permite, com recuo automático para renderização por software onde a GPU derruba o app. |
| RNF-10 | Transições com elementos compartilhados | S | Na troca de tela, elementos presentes nas duas (capa, título, player) animam da posição antiga para a nova (*layout presence*); modais abrem centralizados com animação de pop-up. Respeita `prefers-reduced-motion`. |

## Fora de escopo (por enquanto)

- Contas online e sincronização entre dispositivos.
- iOS.
- Qualquer DRM ou distribuição de conteúdo: o app é de uso pessoal.

## Como marcar a implementação

Coloque uma anotação em comentário, em qualquer arquivo de código:

```rust
// @req RF-04
pub fn set_favorite(...) { ... }

// @test RF-04
#[test]
fn favorita_e_desfavorita() { ... }
```

`@req` marca implementação, `@test` marca verificação. Vários IDs podem ir na mesma linha
(`// @req RF-10 RF-11`). Depois rode `npm run docs:matriz`.
