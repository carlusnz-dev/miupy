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
| RF-04 | Favoritos | M | Adicionar e remover faixas das favoritas, de qualquer tela (inclusive dos resultados da busca no YouTube). As favoritas são as faixas da playlist que o usuário marcou como "favoritas" (RF-30). |
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
| RF-24 | Aleatório e Surfle | S | Tocar a fila em ordem aleatória, ou em sorteio ponderado que favorece a pontuação da faixa e os gêneros preferidos, aproxima gêneros e evita repetir o artista quando há alternativa. |
| RF-25 | Pontuação e DNA | S | Mostrar a pontuação de cada faixa e uma tela "DNA" com o estilo atual do usuário: artistas, gêneros, taxa de pulos, o que subiu e o que caiu. |
| RF-26 | yt-dlp gerenciado pelo app | S | O app baixa, confere e atualiza o yt-dlp sozinho, sem depender de instalação no sistema. |
| RF-27 | Foto de artista | S | Mostrar a foto do artista, vinda do Wikimedia Commons pelo Wikidata, com o crédito de autor e licença. |
| RF-28 | Normalização de volume | S | Igualar o volume percebido entre faixas, como no Spotify: modos Desligado, Baixo (−19 LUFS), Normal (−14 LUFS, padrão) e Alto (−11 LUFS, com limitador). Loudness EBU R128 medido por faixa no download e, nas faixas antigas, em segundo plano; faixa ainda não medida toca sem ganho. |
| RF-29 | Equalizador | S | Equalizador de 8 bandas no motor de áudio, com 8 perfis prontos (Plano, Graves, Agudos, Vocal, Rock, Pop, Eletrônica, Acústico) e até 3 perfis personalizados criados pelo usuário. Escolha nas Configurações e na tela Tocando. |
| RF-30 | Playlist de favoritas | S | O usuário marca qual playlist é a de favoritas. Sem marcação, o app cria "Favoritas" com as favoritas existentes. Trocar a marcação copia as faixas da antiga para a nova; a antiga continua como playlist comum. |
| RF-31 | Baixar playlist automaticamente | S | Uma playlist com "baixar automaticamente" baixa em segundo plano o que falta nela e toda faixa nova adicionada, com aviso em toast. Sem rede, as faixas esperam e o app tenta de novo depois. |
| RF-32 | Andamento do reprocessamento de metadados | S | A tela de Configurações mostra o estado do reprocessamento em lote: em andamento, quantas faixas feitas de quantas, falhas e quando terminou. O usuário pode pedir para parar: a faixa atual termina e o lote para antes da próxima, com resumo das faixas processadas. Enquanto roda, um aviso fixo aparece em qualquer tela, com andamento e botão Parar; fechar o aviso apenas o esconde durante aquele lote. |
| RF-33 | Menu lateral redimensionável | C | O usuário arrasta a borda do menu lateral para mudar a largura, entre 220 e 400 px; a largura fica salva entre aberturas. |
| RF-34 | Busca vazia com sugestões | C | A tela de busca no YouTube, antes de digitar, mostra recomendações (RF-19) e as buscas recentes. A Discoteca tem um botão "Adicionar músicas" que leva a essa tela. |
| RF-35 | Dados de artista em segundo plano | S | Foto, país, gêneros e biografia do artista aparecem na hora a partir do que está guardado e são atualizados em segundo plano quando passam de 30 dias. Opção nas Configurações para desativar a atualização. |
| RF-36 | Tamanho e limpeza do cache | S | As Configurações mostram quanto ocupa o cache de dados vindos da rede (fotos e dados de artista, letras, recomendações, cache do WebView) e permitem apagá-lo, avisando que algumas informações vão demorar mais para carregar. Áudio, capas de álbum, avatar e capas de playlist não entram. |
| RF-37 | Registros do sistema | C | Com o modo de depuração ligado nas Configurações, o app grava e mostra numa tela própria os eventos do sistema: o que tocou, faixas puladas, downloads, reprocessamento de metadados, cargas em segundo plano e erros. Guarda até 30 dias ou 5.000 registros. |
| RF-38 | Menu lateral recolhível | C | O usuário recolhe o menu lateral num trilho só com ícones (botão na borda ou Ctrl+B) e o expande de novo; cada ícone tem nome acessível e dica. O estado fica salvo entre aberturas. |
| RF-39 | Painel da direita | C | Painel recolhível à direita com a faixa atual e a fila: capa, título, artista com link e favoritar; a fila destaca a faixa atual e o clique toca a faixa. Botão na borda e Ctrl+J abrem e recolhem; a escolha fica salva entre aberturas. Aberto por padrão a partir de 1280 px (abaixo disso abre por cima do conteúdo); some em Tocando e no celular. |
| RF-40 | Cores da capa no Tocando | C | O fundo da tela Tocando é um mesh com as cores da capa da faixa atual, com transição ao trocar de faixa; controles seguem o tema. |
| RF-41 | Início em painel | S | O Início reúne cartões de mesh estático para a playlist ou fila atual, estatísticas da discoteca, playlists, mais tocadas e atalhos. Ao abrir um cartão, ele acende por um instante (250 ms) antes da navegação; com movimento reduzido, abre imediatamente. Ações de reprodução respondem na hora. |
| RF-42 | Playlist tocando | S | O player sabe de onde veio a fila (playlist, artista, discoteca…); o Início mostra a playlist que está tocando e a tela da playlist troca Tocar por Pausar quando é ela que toca. |
| RF-43 | Voltar à tela anterior | S | Ao abrir uma tela por link ou atalho, mostrar “← Voltar” no topo para retornar à tela anterior no histórico do app. Não mostrar a seta na abertura do app nem ao navegar pelo menu. Alt+← e o botão lateral de voltar do mouse também retornam quando houver uma tela anterior no histórico do app. |
| RF-46 | Player no celular | S | Abaixo de 768 px, mostrar abas fixas embaixo (Início, Busca, Discoteca, Playlists e Mais) com áreas de toque de pelo menos 48 px e respeito à área segura. O mini player fica acima delas; um toque abre Tocando em tela cheia sobre a interface, com capa, dados, seek, transporte, fila e letra em coluna. Recolher ou arrastar a capa/topo para baixo fecha e volta à tela anterior. Deslizar o mini player para os lados troca de faixa. Gestos respeitam movimento reduzido. |

## Requisitos não funcionais

| ID | Requisito | Prioridade | Descrição |
|---|---|---|---|
| RNF-01 | Banco local SQLite | M | Toda a persistência é local, em SQLite, com migrações versionadas. |
| RNF-02 | Offline-first | M | Tudo que já foi baixado funciona sem rede; rede só para busca, download e enriquecimento. |
| RNF-03 | Multiplataforma | M | Roda nativamente em desktop (Linux, Windows, macOS) e Android, via Rust + Tauri v2. |
| RNF-04 | Interface flat "magma" | M | Visual flat e simples, sem sombras pesadas nem glass/neumorfismo, com bordas de cor fraca. Botões sem gradiente; o `primary` ganha só um brilho pequeno no hover. **Exceção autorizada pelo autor (2026-10-14):** o toast usa vidro translúcido com *backdrop blur*, flat e sem bordas complexas. **Exceção autorizada pelo autor (sessão 08):** o player flutuante do desktop usa fundo mais escuro que a página com *backdrop blur* leve (10 px), com recuo sólido; a rolagem por baixo dele ainda não foi medida (ver relatório da sessão 09). **Exceção autorizada pelo autor (sessão 09):** o menu lateral usa fundo translúcido com *backdrop blur* leve (10 px) e recuo sólido, tanto recolhido quanto expandido. **Tipografia (autor, sessão 08):** títulos (`h1`/`h2`) e números grandes de estatística em Source Serif 4 (`--font-display`); corpo, listas e controles sem serifa. |
| RNF-05 | Gradientes | S | Gradientes bonitos, alguns em *mesh* animado, respeitando `prefers-reduced-motion`. |
| RNF-06 | Documentação viva | M | `docs/` com README por pasta, arquitetura, stacks, requisitos, relatórios de sessão, matriz gerada e JSON da arquitetura. |
| RNF-07 | Integrações externas | C | Núcleo desacoplado da UI para permitir integrar serviços externos do usuário no futuro. |
| RNF-08 | Animações | S | Movimento com física de mola (*spring*): entrada, saída, bounce, hover e fade-in/out, sempre respeitando `prefers-reduced-motion`. |
| RNF-09 | Aceleração de hardware | S | A interface é composta pela GPU quando o sistema permite, com recuo automático para renderização por software onde a GPU derruba o app. |
| RNF-10 | Modais com pop-up | S | Modais abrem centralizados com animação de pop-up. Respeita `prefers-reduced-motion`. A transição de elementos compartilhados entre telas (*layout presence*) foi retirada a pedido do autor (sessão 09). |
| RNF-11 | Carregamento sem espera | S | Toda tela mostra *skeleton* em vez de spinner enquanto não tem dados. Os dados ficam em cache entre telas: ao voltar a uma tela, o cache aparece na hora e é atualizado em segundo plano. |
| RNF-12 | Imagens compactas | S | Imagens guardadas em WebP com perda (qualidade 82): capas e fotos de artista até 600 px no maior lado, avatar e capa de playlist em 512×512. As imagens antigas são convertidas e os originais apagados. |
| RNF-13 | Fluidez | S | Sem engasgos ao trocar de tela ou ao mudar dados: no roteiro de medição (`docs/contribuicao/desempenho.md`, criado pela frente que implementa este requisito), nenhum quadro acima de 50 ms e pelo menos 95% dos quadros abaixo de 16,7 ms. |

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
