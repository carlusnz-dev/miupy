# Casca IPC, streaming, imagens e metadados

- **Data:** 2026-10-14
- **Autor:** agente shell
- **Modelo:** GPT-6; esforço não informado
- **Escopo:** ligar os contratos novos de RF-21 a RF-27, RNF-09, o streaming sem bloqueio e o fluxo de metadados na casca Tauri.

## O que foi feito

- Registrei `player_set_shuffle`, `playlist_set_cover`, `youtube_suggest`, `ytdlp_status`, `ytdlp_update`, `metadata_refresh_library`, `stats_track_scores`, `stats_dna`, `profile_set_avatar` e `system_gpu_status` no `invoke_handler`. `PlayerState.shuffle`, eventos `player://error`, `metadata://progress` e `ytdlp://status` também estão conectados.
- `player_play` de stream agora publica `loading` e retorna enquanto um worker resolve a URL. Um contador de geração descarta respostas obsoletas. Na falha, emite erro e estado parado. Um teste com resolução falsa de 350 ms verifica o limite de 100 ms, troca por uma segunda faixa e usa pause/estado durante a resolução.
- A fila mantém a ordem original para restaurar `off`; `random` e `smart` alteram apenas o trecho depois da faixa atual. O modo é persistido no banco e restaurado no boot. `player_previous` segue a ordem ativa.
- Imagens JPEG, PNG e WebP em base64 são limitadas a 10 MB, recortadas no centro, reduzidas a 512×512 e gravadas como JPEG com nome novo. Avatar e capa removem o arquivo anterior; `null` remove a imagem. O escopo `asset` agora inclui `avatars/` e `playlists/`.
- O yt-dlp gerenciado inicia atualização/verificação em segundo plano. O resolver aponta para o caminho do gerenciador, incluindo o override `MIUPY_YTDLP`; eventos de estado são repassados ao frontend. O runtime JS não precisou ser fixado: o binário oficial fez a prova de busca sem argumento extra.
- O reprocessamento em lote retorna a quantidade enfileirada, processa uma faixa por vez, emite progresso e evento final; um segundo lote ativo recebe `conflict`. Erros 503/limite do MusicBrainz recebem até três novas tentativas com espera crescente.
- Downloads e reprocessamento reaproveitam capas do Cover Art Archive por release, gravam JPEG sob `covers/albuns/` e apontam as faixas para o mesmo caminho. A remoção de faixa preserva uma capa ainda referenciada por outra faixa. Capas WebP antigas são convertidas no reprocessamento; miniaturas WebP novas também são convertidas antes da gravação/embutimento.
- Fotos de artista são buscadas após download e em `metadata_get_artist`, gravadas em `covers/artistas/<id>.jpg` com crédito. O serviço de metadados compartilha o intervalo de retentativa para evitar consultas repetidas.
- Conectei pontuação por faixa e DNA ao core. Para RNF-09, leitura/gravação de `gpuMode`, precedência de `MIUPY_GPU`, estado atual e aviso de reinício estão na interface. A configuração salva é lida do banco Linux antes de criar a janela.
- Atualizei `arquitetura.md` com `bin/`, `avatars/` e `playlists/`. Em `ipc.md`, documentei que um lote de metadados ativo rejeita outro pedido com `conflict`.

## Decisões

- Imagens enviadas ficam em `avatars/` e `playlists/`; capas de álbum e artista permanecem sob `covers/`, permitindo o mesmo protocolo `asset` usado para as capas existentes.
- A chave do cache de capa de álbum é o MBID do release retornado pelo Cover Art Archive. A mesma release é baixada uma vez e compartilhada pelas faixas que a usam.
- `metadata_refresh_library` usa `conflict` enquanto outro lote está ativo; essa decisão agora está anotada em `ipc.md`.
- O app foi iniciado com `XDG_DATA_HOME=/tmp/miupy-asset-proof` para não acessar o banco real do autor. A janela iniciou, mas uma chamada externa `curl http://asset.localhost/...` não alcança o protocolo interno do WebView; por isso, não afirmo uma prova HTTP do arquivo servido. O protocolo já tinha escopo de `covers/**`; o diagnóstico confirmado no log é a rejeição de WebP pelo tagger, corrigida com conversão JPEG. A CSP já autorizava `asset:` e `http://asset.localhost`.
- `npm run docs:matriz` não foi executado: ele escreve arquivos gerados fora da lista explícita de arquivos autorizados para esta frente.

## Verificação (comandos rodados e resultado real)

### Verificado rodando

- `cargo fmt --all`: concluído.
- `cargo clippy --workspace --all-targets`: concluído sem warnings.
- `cargo test --workspace`: **244 aprovados, 0 falhas, 14 ignorados**, incluindo 1 doctest. A casca teve 77 aprovados e 2 ignorados; áudio 26/1; core 77/0; media 63/11.
- Prova integrada ignorada executada explicitamente: `cargo test -p miupy integracao_real_ytdlp_stream_sugestoes_e_foto_de_artista -- --ignored --nocapture`: **1 aprovada**. Baixou yt-dlp para diretório temporário, obteve sugestões reais, baixou foto para MBID real e reproduziu uma faixa do YouTube com `NullBackend`, retornando `loading` em menos de 100 ms e chegando a `playing`.
- A prova integrada também verificou que `MetadataService::get_artist` gravou o JPEG em disco e persistiu o crédito da foto.
- Streaming com resolver falso lento: passou; retorno abaixo de 100 ms, segundo pedido vence e chamadas concorrentes não bloqueiam.
- `npm run check`: 0 erros e 0 warnings; `npm test`: 18 aprovados; `npm run build`: concluído.
- `npx tauri dev` iniciou uma janela com dados isolados em `/tmp`; `curl` para `asset.localhost` no host falhou com conexão recusada, portanto a URL servida pelo WebView não foi confirmada por curl.
- `git diff --check`: concluído sem erros.
- Após a prova integrada, tentei repetir `cargo test --workspace`; a criação de `libmiupy_lib.a` falhou por falta de espaço. A execução completa aprovada acima foi depois da última alteração de código de produção; depois disso, apenas o corpo do teste ignorado foi reforçado e executado com sucesso. `cargo clippy --workspace --all-targets` e `cargo fmt --all` foram repetidos após essa alteração.

### Só escrito, sem verificação específica

- Não há teste IPC que invoque diretamente cada wrapper Tauri novo em isolamento. Os caminhos de domínio relacionados têm testes no core/casca, incluindo shuffle, imagem, reprocessamento, estatísticas e o fluxo de streaming.
- Não executei `npm run docs:matriz`, conforme a restrição de escrita dos arquivos gerados.
- A leitura antecipada do modo GPU foi implementada para Linux; não foi feita prova em Windows/macOS nem Android.

## Pendências e problemas conhecidos

- A URL `asset` não foi verificada por requisição externa: o protocolo é interceptado no contexto do WebView. A prova de escopo é a configuração Tauri e a inicialização da janela isolada; ainda falta observação visual de uma capa na janela real.
- Os comandos Tauri não têm teste individual de serialização/dispatch para cada wrapper recém-registrado.
- A instalação do yt-dlp e as chamadas de rede podem falhar se os serviços externos estiverem indisponíveis; os erros continuam tipados e o app mantém os dados locais.

## Próximos passos

- Abrir o app com o banco real, conferir as capas JPEG convertidas e o campo de álbum; executar o reprocessamento para preencher as lacunas existentes.
- Rodar `npm run docs:matriz` pela orquestração, que controla os arquivos gerados da matriz.
- Revisar visualmente a seção de aceleração de hardware e confirmar o comportamento do protocolo `asset` na janela.
