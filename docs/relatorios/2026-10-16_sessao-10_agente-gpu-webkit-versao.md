# GPU automática pela versão do WebKitGTK

- **Data:** 2026-10-16
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RNF-09 e documentação de fluidez do RNF-13 em Wayland + NVIDIA.

## O que foi feito

- A casca consulta a versão da biblioteca WebKitGTK carregada antes de criar a janela, usando o crate Linux já presente no projeto.
- O Automático usa GPU com WebKitGTK anterior a 2.54 e software a partir de 2.54 ou quando a versão é desconhecida, apenas em Wayland + NVIDIA.
- A tentativa automática com GPU arma a sentinela; uma queda faz a próxima abertura recuar para software, restaurar `auto` e remover a sentinela.
- Testes cobrem a fronteira 2.53/2.54, versão desconhecida, outras combinações de ambiente, precedência de modos e variáveis, e recuperação após queda.
- O contrato textual do IPC, a descrição da sentinela na arquitetura e o guia de desempenho refletem a nova escolha. O guia registra a observação informada pelo autor: 2.50.4 fluido no AppImage oficial e 2.54.1 engasgando no Arch.

## Decisões

- Nenhuma dependência nova: `webkit2gtk = "2"` já era direta no Linux e expõe as funções C de versão.
- `Decision.reason` passou a `String` para incluir a versão no motivo devolvido por `system_gpu_status`; a forma do IPC não mudou.
- O recuo da sentinela no modo Automático segue o fluxo existente: restaura `auto` e remove o marcador. Uma abertura futura poderá tentar GPU novamente se a versão ainda for anterior a 2.54.
- `docs/arquitetura/arquitetura.json` e `.md` só tiveram a descrição da sentinela corrigida; a estrutura não mudou.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou, sem avisos.
- `cargo test --workspace` e nova execução com `--quiet` após os últimos testes: passaram. Casca: 134 passaram, 3 ignorados; áudio: 35 passaram, 1 ignorado; núcleo: 103 passaram; mídia: 90 passaram, 12 testes de rede ignorados; doctest: 1 passou.
- `npm run check`: primeira tentativa falhou com `EROFS` em `node_modules/.vite-temp`, link somente leitura; repetição com links temporários locais para os pacotes passou, 0 erros e 0 avisos.
- `npm test`: primeira repetição com o link original guardado dentro da árvore encontrou dois testes da dependência `devalue` e falhou por `uvu` ausente; execução com o link removido temporariamente passou, 23 arquivos e 102 testes. O link original de `node_modules` foi restaurado.
- `npm run build`: passou com os links temporários locais, 215 módulos transformados.
- `node scripts/rastreabilidade.mjs --check`: passou, 56 requisitos e nenhum ID desconhecido.
- `git diff --check`: passou.

## Pendências e problemas conhecidos

- A escolha não foi medida em uma janela real neste worktree; a fluidez de 2.50.4 e o engasgo de 2.54.1 são observações fornecidas pelo autor.
- A sentinela detecta uma abertura que não chegou à confirmação, como antes; uma queda posterior à confirmação não é capturada por ela.

## Fatias de commit

1. `fix(shell): lê a versão do webkitgtk antes da janela` — `src-tauri/src/gpu.rs` (consulta da biblioteca e passagem da versão à decisão). Rodapé: `Refs: RNF-09`.
2. `fix(shell): usa gpu no automático com webkitgtk anterior ao 2.54` — `src-tauri/src/gpu.rs` (regra, motivo e sentinela). Rodapé: `Refs: RNF-09`.
3. `test(shell): cobre a decisão por versão do webkitgtk` — `src-tauri/src/gpu.rs` (testes). Rodapé: `Refs: RNF-09`.
4. `docs(docs): registra a diferença entre webkitgtk 2.50 e 2.54` — `docs/contribuicao/desempenho.md`, `docs/arquitetura/ipc.md`, `docs/arquitetura/arquitetura.md`, `docs/arquitetura/arquitetura.json`, `docs/relatorios/2026-10-16_sessao-10_agente-gpu-webkit-versao.md`. Rodapé: `Refs: RNF-09 RNF-13`.

## Próximos passos

- Medir no app real as duas versões do WebKitGTK com o Automático e confirmar o motivo exibido em Configurações.
