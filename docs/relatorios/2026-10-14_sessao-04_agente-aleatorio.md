# Aleatório no meio da fila

- **Data:** 2026-10-14
- **Autor:** agente aleatorio
- **Modelo:** Codex (GPT-6; esforço não informado)
- **Escopo:** corrigir a troca de modo aleatório após avançar na fila (RF-24).

## O que foi feito

- `Player::set_shuffle` agora reordena a fila em vigor, preservando o prefixo já tocado e a posição da ocorrência atual.
- `off` ordena apenas as ocorrências restantes pela fila original. `PlayQueue` conserva essa ordem após reordenações e a atualiza quando uma faixa é removida.
- O sorteio `smart` recua para o sorteio uniforme se faltarem metadados de uma faixa, para não perder ocorrências da fila.
- O contrato em `docs/arquitetura/ipc.md` descreve a troca de modo, inclusive com IDs repetidos. A matriz de rastreabilidade foi regenerada.

## Decisões

- O teste anterior de fila nova com aleatório esperava que `off` movesse a faixa atual de volta à posição original. Ajustei a expectativa: a faixa pedida continua no começo e só o restante volta à ordem original, conforme a regra do briefing.
- Um teste fixa uma permutação possível após ligar `random`, para que a regressão não dependa da semente baseada no relógio. Outros testes exercitam as trocas reais de modo e as ocorrências repetidas.
- `docs/arquitetura/arquitetura.json` não mudou porque a estrutura de módulos não mudou.

## Verificação (comandos rodados e resultado real)

- Antes da correção, `cargo test -p miupy --lib player::tests::alternar_apos_avancar_no_aleatorio_preserva_prefixo_tocado` falhou: a fila perdeu o prefixo tocado. `cargo test -p miupy --lib player::tests::id_repetido_na_posicao_atual_preserva_ocorrencias` falhou: o índice voltou para a primeira ocorrência.
- `cargo fmt --all`: passou.
- `cargo test -p miupy --lib player::tests::`: 28 passaram, 1 ignorado, 0 falhas após a correção.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou, 0 avisos.
- `cargo test --workspace`: 251 passaram, 14 ignorados, 0 falhas (inclui 1 doc-test). Os ignorados exigem rede ou dispositivo de áudio.
- `npm run docs:matriz`: passou; 37 requisitos, 30 verificados, 4 implementados, 3 pendentes.
- Todos os comandos `cargo` de compilação e teste rodaram com `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10`, conforme o briefing.

## Pendências e problemas conhecidos

- Nenhum problema conhecido neste escopo. Os testes de integração com rede e dispositivo de áudio permaneceram ignorados pela configuração do projeto.

## Próximos passos

- Revisão e integração das mudanças pela orquestração. O working tree foi deixado sem commit, conforme o briefing.
