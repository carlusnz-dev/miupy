# Pausa durante a resolução do streaming

- **Data:** 2026-10-14
- **Autor:** agente pausa
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** preservar pausa e retomada enquanto uma URL de streaming é resolvida.

## O que foi feito

- O player guarda o pedido de pausa durante a resolução e carrega o stream sem iniciar o áudio quando a URL fica pronta. A retomada durante a resolução cancela o pedido; depois do carregamento, inicia a reprodução.
- `next` e `previous` agora invalidam a resolução pendente quando selecionam outra faixa, impedindo que a URL antiga substitua a seleção.
- O `FakeResolver` recebeu um canal de controle para testes sem espera arbitrária. Foram adicionados testes para pausa, retomada e navegação durante a resolução.
- O contrato IPC explicita os estados emitidos, e a matriz de rastreabilidade foi regenerada.

## Decisões

- Enquanto a URL ainda é resolvida, o status permanece `loading`, inclusive com pausa solicitada. Quando o áudio fica pronto, o estado passa a `paused` na posição 0. O backend real pode continuar em `loading` durante a abertura HTTP, até ter áudio carregado.
- A retomada de URL expirada e o avanço automático resolvem sob a trava do player. Um comando de pausa só entra depois dessa resolução, então não sofrem a mesma corrida. Essa resolução síncrona pode atrasar o comando; alterar esse fluxo ficou fora deste escopo.
- `docs/arquitetura/arquitetura.json` não mudou porque a estrutura dos módulos permaneceu igual.

## Verificação (comandos rodados e resultado real)

- Antes da correção: `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test -p miupy pausa_durante_resolucao_carrega_sem_tocar_e_permite_retomar -- --nocapture` falhou: recebeu `Playing` em vez de `Paused`.
- Após a correção: o mesmo teste passou (1 aprovado). `cargo test -p miupy player::tests -- --nocapture`, com os mesmos limites de CPU, passou (25 aprovados, 1 ignorado).
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets`: passou sem avisos.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace`: passou (248 aprovados, 14 ignorados, 0 falhas; inclui 1 doctest aprovado).
- `npm run docs:matriz`: passou; 37 requisitos, 30 verificados, 4 implementados, 3 pendentes.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- Testes de rede e dispositivo de áudio permanecem ignorados, conforme suas marcações existentes. A abertura HTTP do backend real não foi exercitada manualmente nesta sessão.

## Próximos passos

- Revisar e integrar a mudança na branch de desenvolvimento conforme o fluxo do projeto.
