# Recuperação da GPU em Wayland/NVIDIA

- **Data:** 2026-10-14
- **Autor:** agente gpu
- **Modelo:** GPT-6; esforço não informado
- **Escopo:** impedir que o modo de GPU salvo prenda o app em um ciclo de quedas no início.

## O que foi feito

- A decisão de GPU agora distingue modo salvo, `MIUPY_GPU` e override do WebKit. O motivo registrado para `on` ou `off` salvo menciona as configurações, não a variável de ambiente.
- Em Wayland/NVIDIA, `on` salvo grava `gpu-startup-pending` no diretório de dados antes da janela. Se a sentinela ainda estiver lá na próxima abertura, o app inicia por software, restaura `auto` no banco e informa o motivo em `GpuStatus.reason`.
- A interface confirma a abertura após cinco segundos montada; saída normal também apaga a sentinela. Uma nova escolha nas configurações descarta uma sentinela antiga, permitindo tentar novamente.
- A tela de Configurações avisa sobre o risco ao selecionar “Ativada”. O contrato IPC e a arquitetura registram o comando de confirmação e o arquivo em disco.

## Decisões

- A sentinela é criada só quando a aceleração vem do modo salvo `on` e a combinação Wayland/NVIDIA é detectada. `MIUPY_GPU` explícita preserva a precedência e não recebe sentinela.
- A confirmação parte de `App.svelte` cinco segundos após montar. Isso exige que o frontend carregue e que o processo sobreviva ao intervalo; uma falha antes disso deixa a sentinela para a próxima abertura.
- Se a sentinela não puder ser escrita, a abertura recua para software. O teste da decisão não depende de WebView.

## Verificação (comandos rodados e resultado real)

- Teste de regressão escrito primeiro: `cargo test -p miupy --lib gpu::tests::motivo_identifica_modo_salvo -- --exact` falhou como esperado; motivo real `MIUPY_GPU=on`, esperado `GPU ativada nas configurações`.
- `cargo test -p miupy --lib gpu::tests`: 5 aprovados, 0 falhas antes do último teste de descarte. Depois, `cargo test --workspace -q`: **250 aprovados, 0 falhas, 14 ignorados**; a casca teve 83 aprovados e 2 ignorados.
- `cargo fmt --all`: concluído. `cargo clippy --workspace --all-targets`: concluído sem warnings.
- `npm ci`: concluído; 79 pacotes instalados, 0 vulnerabilidades informadas. `npm run check`: 0 erros e 0 warnings. `npm test`: 18 aprovados em 4 arquivos. `npm run build`: concluído.
- `npm run docs:matriz`: 37 requisitos, 30 verificados, 4 implementados, 3 pendentes.
- `git diff --check`: concluído sem erros.

## Pendências e problemas conhecidos

- O ciclo real de queda e recuperação ainda precisa ser observado em uma sessão Wayland/NVIDIA; os testes cobrem a decisão e a persistência sem abrir janela.

## Próximos passos

- Validar manualmente a abertura com `gpuMode=on` em Wayland/NVIDIA e a recuperação após uma queda antes dos cinco segundos.
