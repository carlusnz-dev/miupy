# Limites de requisições de metadados

- **Data:** 2026-10-14
- **Autor:** agente limite
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** limites e novas tentativas de MusicBrainz/LRCLIB; recuperação de letra e foto após falha temporária.

## O que foi feito

- `miupy-media` passou a reservar uma vaga para **cada** tentativa no MusicBrainz (1 req/s) e no LRCLIB (intervalo de 2 s), inclusive após 503/429 ou falha de transporte. Até três tentativas, espera crescente (1 s, 2 s), `Retry-After` em segundos ou data HTTP e limite total de 15 s por operação. Se o servidor pedir mais de 8 s, a operação termina sem enviar uma chamada cedo demais.
- A casca removeu o segundo laço de tentativas do lote, que multiplicava as tentativas e as esperas feitas pelo provedor.
- Erro ou timeout de letra, enriquecimento do artista ou foto usa um intervalo curto de 10 s. Resposta sem dados mantém os 5 min existentes. O controle continua em memória na casca; o banco não precisou de migração.
- `get_artist` agora obtém o MBID antes de procurar a foto na mesma consulta. Isso permite que a chamada disparada após um download grave a foto sem esperar uma segunda consulta ao artista.
- Contrato IPC atualizado para o novo comportamento; matriz de rastreabilidade gerada.

## Decisões

- Investigação: `ratelimit.rs` reserva vagas em série sem rajadas; `mb_get` é a passagem de todas as chamadas do MusicBrainz (`find_recording`, `find_artist`, `artist_details` e `artist_photo`). `src-tauri/src/lib.rs` cria uma instância compartilhada de `OnlineMetadata`. O Cover Art Archive e as miniaturas usam `download_image`; Wikidata e Commons usam chamadas diretas independentes; o LRCLIB usava chamadas diretas sem limitador. O lote percorre faixas em série, mas letra e foto também são pedidas por comandos e downloads em paralelo. Por isso um limitador próprio do LRCLIB é necessário mesmo com o lote serial.
- O LRCLIB não publica cota: 2 s entre chamadas é uma escolha conservadora para evitar a sequência de 503 observada. O MusicBrainz mantém 1 s entre chamadas na instância compartilhada. Espera maior que 8 s indicada por `Retry-After` encerra a operação para não prender a fila, sem desrespeitar o servidor.
- O intervalo de 5 min estava no `HashMap` em memória de `MetadataService`, não no core ou no banco. Não foi criada migração.

## Verificação (comandos rodados e resultado real)

- Antes da correção, `cargo test -p miupy-media metadata::tests::musicbrainz_repete_503_respeitando_retry_after_e_limite -- --exact` **falhou**, pois o 503 retornou erro sem nova tentativa. O teste novo da foto na primeira consulta também **falhou** antes da mudança de ordem.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou, zero warnings.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace`: passou antes do último teste de data HTTP: 251 testes unitários aprovados, 14 ignorados, 1 teste de documentação aprovado.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace --quiet`: verificação final passou com 252 testes unitários aprovados, 14 ignorados e 1 teste de documentação aprovado.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test -p miupy-media -- --ignored --test-threads=1`: 10 testes de rede aprovados, 1 falhou (`ytdlp_stream_real_responde_audio_parcial`). Execução única e sequencial para moderar chamadas; o número exato de requisições HTTP feitas pelo yt-dlp não foi instrumentado. Não houve repetição após a falha.
- `npm run docs:matriz`: passou; 37 requisitos, 30 verificados, 4 implementados, 3 pendentes.
- `cargo fmt --all`: passou.

## Pendências e problemas conhecidos

- O teste de streaming real do yt-dlp falhou porque o YouTube respondeu `Sign in to confirm you’re not a bot` ao vídeo `jNQXAC9IVRw`. É uma restrição externa ao assunto deste agente; o teste de rede não está totalmente verde.
- Os intervalos são por instância do app. Outras instâncias do miupy no mesmo IP não compartilham a fila.

## Próximos passos

- Reavaliar o teste de streaming com uma origem que permita acesso anônimo, sem ampliar as chamadas nesta sessão.
