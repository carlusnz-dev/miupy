# Normalização de volume (RF-28)

- **Data:** 2026-10-14
- **Autor:** agente normalizacao
- **Modelo:** GPT-6
- **Escopo:** Medição EBU R128, ganho por faixa, cadeia DSP, persistência, casca e configuração de áudio.

## O que foi feito

- Migração `0004_loudness.sql` adiciona loudness, pico verdadeiro e modo de normalização com padrão `normal`.
- `miupy-audio` mede loudness integrado e pico verdadeiro sem dispositivo; a cadeia do rodio aplica ganho suavizado, estágio plano de EQ, volume e limitador com medição de pico verdadeiro por blocos antecipados. O `NullBackend` expõe o ganho para testes.
- Download mede a faixa após gravar o arquivo. Uma fila sequencial de baixa prioridade mede as faixas antigas fora da trava do player, sem repetir na sessão os arquivos que não produziram medida. No Linux, a thread usa `nice` 10 para ceder CPU à reprodução. A casca aplica o ganho ao carregar e ao trocar o modo.
- Nova seção Áudio nas Configurações; tipos e mock atualizados. READMEs de áudio/core e esquema de migrações atualizados.

## Decisões

- A [documentação do Spotify](https://support.spotify.com/br-pt/artists/article/loudness-normalization/) confirma os alvos −19/−14/−11 LUFS e a restrição de ganho positivo pelo pico em −1 dBTP nos modos Baixo/Normal. Ela descreve o limitador do modo Alto como limite de **pico de amostra** em −1 dB, ataque de 5 ms e decaimento de 100 ms. O contrato do miupy exige limitação por **pico verdadeiro** no fim da cadeia em todos os modos; foi seguido o contrato. A crate [`ebur128` 0.1.10](https://docs.rs/crate/ebur128/latest/source/Cargo.toml) usa licença MIT.
- A medição silenciosa devolve `None`; faixas sem medida tocam com 0 dB. Uma falha de medição não falha o download.
- O estágio de EQ foi deixado plano com interface `EqStage` para a frente RF-29. O limitador usa blocos de 5 ms, antecipação de um bloco e margem de 0,3 dB para a junção; a medição de pico verdadeiro é feita pelo `ebur128` com sobreamostragem.

## Verificação (comandos rodados e resultado real)

- `npm ci`: passou; 79 pacotes instalados.
- `npm run check`: primeira execução falhou por `Track` de teste sem `loudnessLufs`; corrigido e nova execução passou sem erros ou avisos.
- `npm test`: 4 arquivos, 18 testes passaram.
- `npm run build`: passou.
- `npm run docs:matriz`: passou; matriz gerada.
- `git diff --check`: passou.
- `cargo fmt --all`: passou.
- `cargo check --workspace`: primeira execução falhou ao baixar `dasp_frame` porque `~/.cargo/registry` era somente leitura; com `CARGO_HOME=/tmp/miupy-cargo`, passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: primeiras execuções apontaram `Default` derivável, argumentos `NonZero` no teste e falta da dev-dependency `tempfile`; corrigidos. Última execução passou.
- `cargo test --workspace` (execução final com `--quiet`): primeira execução falhou em três testes de `seek` porque o wrapper DSP não repassava `try_seek`; corrigido. Um teste ampliado do limitador revelou pico de −0,14 dBTP a 19 kHz, corrigido com análise EBU por blocos. Execução final passou: casca 96/98 (2 ignorados), áudio 28/29 (1 ignorado), core 79/79, media 68/68, rede 11 ignorados e doctests passaram.
- `cargo test -p miupy-audio --lib`: passou após a correção do `seek` (28 passaram, 1 ignorado).
- `cargo test --workspace dsp::tests::boosted_signal_stays_below_true_peak_ceiling`: passou com seno de 1 kHz, 19 kHz e impulsos, todos sob −1 dBTP.

## Pendências e problemas conhecidos

- A execução de Cargo precisou de `CARGO_HOME=/tmp/miupy-cargo` porque o sandbox não permitiu baixar `ebur128` em `~/.cargo/registry`; a configuração do sccache foi copiada, sem mudar `RUSTFLAGS` ou `CARGO_TARGET_DIR`.
- A medição e o DSP do rodio estão disponíveis no desktop; o Android continua usando `NullBackend` enquanto o plugin Media3 não existe. O pico verdadeiro foi verificado em sinais de 1 kHz, 19 kHz e impulso; esses testes não cobrem todos os sinais possíveis nem uma escuta manual.

## Próximos passos

- Revisar o encaixe do equalizador RF-29 sobre o estágio plano.
