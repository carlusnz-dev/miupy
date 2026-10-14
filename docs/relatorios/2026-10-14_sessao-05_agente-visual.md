# Botão flat, toast de vidro e menu lateral ajustável

- **Data:** 2026-10-14
- **Autor:** agente visual
- **Modelo:** GPT-6, esforço padrão
- **Escopo:** RNF-04 e RF-33 no frontend, núcleo, casca e documentação.

## O que foi feito

- O botão primário e o botão de transporte usam lava sólida, com brilho suave no hover e foco visível.
- O toast ganhou espaço interno, ícone por tipo e vidro translúcido com `backdrop-filter` e `-webkit-backdrop-filter`. Sem suporte ao filtro, usa superfície opaca. O texto usa `--fg`, de alto contraste nos três temas.
- O menu lateral tem alça de 8 px com ponteiro, setas de 8 px, limites de 220 a 400 px, duplo clique para 280 px e atributos ARIA. O arraste escreve apenas `--sidebar-w`; `settings_update` roda ao soltar. A largura em cache é aplicada no `index.html` antes da primeira pintura e confirmada pelo banco ao iniciar.
- A migração `0006_sidebar_width.sql` adiciona a coluna com padrão 280 e `CHECK`; núcleo, DTO, tipos e mock expõem `sidebarWidth`. Testes cobrem migração aditiva, padrão, corte, arraste, teclado e salvamento.
- Atualizados o esquema documentado, `arquitetura.json` e a matriz de rastreabilidade.

## Decisões

- A migração reservada `0006` é a quinta entrada da lista atual, pois `0005` pertence à frente do equalizador ainda não integrada. Nesta branch, `PRAGMA user_version` passa de 4 para 5. A integração deverá inserir `0005` antes de `0006` e conferir bancos já migrados.
- O cache `miupy.sidebarWidth` serve somente para evitar o salto visual na abertura; o banco continua sendo a fonte da verdade.
- Salvamentos por teclado são enfileirados para manter a ordem. Respostas antigas não desfazem a pré-visualização de uma tecla mais recente.
- A alça usa `role="separator"` ajustável, com supressão localizada dos avisos do compilador Svelte, que classifica esse papel como não interativo.

## Verificação (comandos rodados e resultado real)

- `npm ci`: passou; 79 pacotes instalados, 0 vulnerabilidades.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo fmt --all`: passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 nice -n 10 cargo test --workspace`: passou, sem falhas; testes de rede marcados como ignorados.
- `npm run check`: passou, 0 erros e 0 avisos após corrigir os avisos iniciais da alça.
- `npm test`: passou, 8 arquivos e 25 testes.
- `npm run build`: passou, 180 módulos transformados.
- `npm run docs:matriz`: passou; 50 requisitos, 33 verificados, 4 implementados, 13 pendentes.
- `git diff --check`: passou.
- `npm run dev -- --host 127.0.0.1`: iniciou e respondeu HTTP 200. `firefox --headless --window-size 1440,900 --screenshot /tmp/miupy-visual-flat.png http://127.0.0.1:1420`: informou `RenderCompositorSWGL failed mapping default framebuffer` e não criou a captura. Segunda tentativa com renderização por software encerrou por tempo limite de 15 s, também sem captura.

## Pendências e problemas conhecidos

- O filtro de fundo foi implementado com as duas propriedades aceitas pelos motores WebKit, mas o efeito visual no WebKitGTK não pôde ser observado neste ambiente.
- Não há captura da interface por causa da falha do Firefox headless descrita acima.
- A ordem das migrações `0005` e `0006` precisa ser reconciliada na integração com a frente do equalizador.

## Próximos passos

- Conferir o toast e o arraste numa janela Tauri com WebKitGTK e integrar as migrações na ordem reservada.
