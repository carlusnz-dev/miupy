# Testes do yt-dlp gerenciado

- **Data:** 2026-10-14
- **Autor:** agente ytdlp
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** RF-26 em `miupy-media`: variável `MIUPY_YTDLP`, intervalo de 24 horas e investigação de teste intermitente.

## O que foi feito

- Injetei no gerenciador a leitura de `MIUPY_YTDLP` e o relógio, preservando a API pública. Os testes não alteram o ambiente global nem dependem da hora real.
- Acrescentei testes para caminho definido existente, caminho definido inexistente, valor vazio, primeira consulta, limite exato de 24 horas, consulta antes do limite e nova tentativa após falha.
- Investiguei o teste `falha_do_processo_vira_erro_tipado_e_limpa`. Ele passou 50/50 antes das mudanças e 500/500 em uma segunda rodada. O fluxo atual espera o encerramento do filho e a leitura de `stderr` antes de classificar o erro e limpar o arquivo parcial. Sem falha reproduzida ou evidência de causa, não alterei o processamento de `ytdlp.rs`.

## Decisões

- Uma consulta que falha não grava `checked_at`: o carimbo só é escrito depois da verificação bem-sucedida. Assim a próxima chamada pode tentar de novo. O contrato não especifica o comportamento da falha.
- `MIUPY_YTDLP` definido e existente evita consulta mesmo quando `force = true`; valor vazio equivale a ausência da variável. Isso segue a exceção de desenvolvimento do contrato.
- A estrutura de módulos, o contrato IPC e o esquema do banco não mudaram; `arquitetura.json` e `banco-de-dados.md` dispensam alteração.

## Verificação (comandos rodados e resultado real)

- `for i in $(seq 50); do CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test -p miupy-media falha_do_processo_vira_erro_tipado_e_limpa --quiet; done`: 50/50 passaram; 0 falhas.
- Mesma repetição com `seq 500`: 500/500 passaram; 0 falhas. A segunda rodada ocorreu durante a edição do gerenciador; o teste e o código do processo não foram alterados.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test -p miupy-media ytdlp_manager --quiet`: 5 testes passaram.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo fmt --all`: passou. `cargo fmt --all -- --check` também passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou sem avisos.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test --workspace`: passou; `miupy` 101, `miupy-audio` 33, `miupy-core` 82 e `miupy-media` 71 testes passaram, 14 testes de rede/áudio ignorados, 1 doctest passou.
- `node scripts/rastreabilidade.mjs --check`: passou; 50 requisitos, nenhum ID desconhecido.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- A falha histórica do teste de processo não foi reproduzida em 550 execuções isoladas nem em
  40 execuções da suíte inteira do `miupy-media` com 4 threads (feitas pela orquestração). Sua causa permanece desconhecida; portanto a correção dessa falha específica não pode ser declarada concluída. Houve registro anterior de 1 falha em 8 execuções (sessão 02), mas sem diagnóstico preservado.

## Próximos passos

- Se a falha reaparecer, guardar a saída completa, o erro do sistema ao iniciar o filho e os arquivos temporários para identificar a causa antes de alterar o fluxo de download.

## Fatias de commit

1. `test(media): permita injetar ambiente e relógio no gerenciador` — `crates/miupy-media/src/ytdlp_manager.rs` (campos, construtor e leituras).
2. `test(media): cubra o caminho configurado do yt-dlp` — `crates/miupy-media/src/ytdlp_manager.rs` (testes do caminho definido e vazio).
3. `test(media): cubra o intervalo diário e a falha de consulta` — `crates/miupy-media/src/ytdlp_manager.rs` (teste com relógio e servidor locais).
4. `docs(docs): registre a investigação do yt-dlp` — este relatório.

Nenhum commit foi criado neste worktree.
