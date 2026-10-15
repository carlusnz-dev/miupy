# Equalizador de oito bandas

- **Data:** 2026-10-14
- **Autor:** agente equalizador
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** RF-29, com correção do limitador RF-28

## O que foi feito

- Implementei oito biquads por canal no motor de áudio, com coeficientes calculados para a taxa da faixa, calibração da resposta nas frequências centrais, pré-ganho e cruzamento de 20 ms ao trocar o perfil.
- Substituí a medição EBU por bloco na thread de áudio por interpolação de pico 4× com histórico entre blocos, antecipação e rampa de ganho do limitador. Incluí testes de pico verdadeiro, troca de ganhos e seek.
- Registrei a migração 0005, implementei os oito perfis prontos e o CRUD de até três personalizados, com validação e códigos de erro do contrato.
- Liguei os comandos Tauri, a aplicação imediata do perfil e a restauração na abertura; acrescentei teste com backend falso.
- Acrescentei seletor e editor nas Configurações, seletor rápido em Tocando, tipos e clientes IPC real e mock. Atualizei a documentação do banco e a arquitetura JSON.

## Decisões

- Calibrei os ganhos dos biquads pela resposta conjunta nas oito frequências centrais. Filtros adjacentes se sobrepõem; aplicar literalmente o ganho solicitado em cada filtro não produziria a curva definida no contrato.
- A interpolação 4× do limitador usa coeficientes sinc com janela e memória entre blocos. O cálculo e os buffers são preparados antes de processar as amostras; a thread de áudio não chama `ebur128` nem faz alocação por bloco.
- O contrato IPC não foi alterado.

## Verificação (comandos rodados e resultado real)

- `cargo test -p miupy-core --lib`: primeira rodada falhou em cinco testes com expectativas da versão 4/serialização antiga; após atualizar, a suíte passou.
- `cargo test -p miupy-audio --lib builtin_frequency_response`: primeira rodada falhou porque a soma dos filtros não atingia a curva; após calibrar, passou.
- `npm ci`: passou, 79 pacotes instalados. `npm run check` falhou inicialmente por dependências ausentes e depois por um nome incorreto no editor; correções aplicadas.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: primeira rodada encontrou dois laços desnecessários; após correção, passou.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace --quiet`: passou; casca 101 testes, 2 ignorados; áudio 33, 1 ignorado; core 80; media 68 e um doc-test, 11 testes de rede ignorados.
- `npm run check`: passou, zero erros e avisos. `npm test`: passou, 22 testes. `npm run build`: passou.
- `npm run docs:matriz`: passou, 50 requisitos; 33 verificados, 4 implementados, 13 pendentes.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- A integração visual no WebKitGTK e a audição em dispositivo real não foram verificadas neste worktree. O Android ainda usa o backend nulo já existente.
- Os testes de rede e dispositivo marcados como ignorados continuam fora da suíte padrão.

## Próximos passos

- Revisar o diff e executar uma prova auditiva no desktop antes do merge.
