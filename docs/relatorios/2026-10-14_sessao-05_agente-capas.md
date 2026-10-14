# Capas locais no WebKitGTK

- **Data:** 2026-10-14
- **Autor:** agente capas
- **Modelo:** GPT-6, esforço médio
- **Escopo:** RF-05; carregamento de imagens locais em desenvolvimento e no app instalado

## O que foi feito

- A causa está na classificação do esquema `asset` pelo WebKitGTK. No Linux, `convertFileSrc` gera `asset://localhost/...`; o wry 0.57 registra esse esquema como *secure*, mas não como *CORS-enabled*. O WebKit bloqueia a imagem pedida pela página `http://localhost:1420` antes de chegar ao handler do Tauri. A [documentação do WebKitGTK](https://webkitgtk.org/reference/webkit2gtk/2.38.2/method.SecurityManager.register_uri_scheme_as_cors_enabled.html) descreve o registro usado na correção.
- `src-tauri/src/asset_webkit.rs` habilita CORS para `asset` no contexto WebKit da janela principal, na inicialização. Isso vale para a origem HTTP de desenvolvimento e a origem do app instalado. O handler do Tauri já devolve `Access-Control-Allow-Origin` para a origem da janela e mantém a conferência de `assetProtocol.scope`.
- `coverPath` permanece um caminho local; `api.resolveAsset` continua usando `convertFileSrc`. O escopo e o CSP não foram abertos nem desligados. A dependência `webkit2gtk` 2.0.2, licença MIT, foi acrescentada somente para Linux.
- Acrescentei testes para o registro do esquema, o escopo/CSP de ambos os modos e o encaminhamento de caminhos locais por `resolveAsset`. Atualizei a arquitetura e a matriz de rastreabilidade.

## Decisões

- Nenhuma mudança no contrato IPC. O detalhe de implementação não exigiu alteração de `docs/arquitetura/ipc.md`, `mock.ts` ou tipos.
- O áudio local é entregue diretamente ao backend Rust a partir de `file_path`; não usa uma URL `asset://` no frontend. O problema de capas não pode ser avaliado pela reprodução do áudio.
- `layoutPresence.ts` anima o contêiner da capa, sem clonar a imagem. O `append` do Svelte é a inserção do `<img>`; `loading="lazy"` pode adiar a carga fora da tela, mas não explica o bloqueio de todas as imagens. O bloqueio ocorreu na checagem de origem do WebKit, antes da checagem do CSP ou do escopo do Tauri.

## Verificação (comandos rodados e resultado real)

- Teste inicial com `WebContext::new()` sem GTK: **falhou** antes da asserção (`GTK has not been initialized`), porque o teste não dispõe de janela. Substituído por teste do registro injetado.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test -p miupy asset_scheme_is_registered_for_cors --lib`: **falhou como esperado** antes da correção (`None` em vez de `Some("asset")`); depois, `cargo test -p miupy asset_webkit --lib`: **2 passaram**.
- `npm ci`: **passou**, 79 pacotes instalados, 0 vulnerabilidades.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo fmt --all`: **passou**.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo clippy --workspace --all-targets -- -D warnings`: **passou**.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo test --workspace`: **passou**; 95 testes da casca passaram, 2 ignorados; áudio 26 passaram e 1 ignorado; core 77 passaram; media 68 passaram e 11 testes de rede ignorados; 1 doc-test passou.
- `npm run check`: **passou**, 0 erros e 0 avisos. `npm test`: **passou**, 5 arquivos e 20 testes. `npm run build`: **passou**, 177 módulos transformados.
- `npm run docs:matriz`: **passou**, 50 requisitos, 30 verificados, 4 implementados e 16 pendentes.
- `timeout 25s npx tauri dev`: **falhou antes de abrir a janela**, pois a porta 1420 já estava ocupada.
- `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 npx tauri build --debug --no-bundle`: **passou**, binário criado em `target/debug/miupy`.
- Após acrescentar a asserção de inicialização do WebKit, `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace -q`, `npm run docs:matriz` e `git diff --check` **passaram novamente**. `CARGO_BUILD_JOBS=3 RUST_TEST_THREADS=3 nice -n 10 cargo build -p miupy` também **passou**.
- `timeout 15s target/debug/miupy`: **falhou na inicialização do plugin de log**, porque o diretório de dados padrão é somente leitura neste sandbox. Com `XDG_DATA_HOME`, `XDG_CONFIG_HOME` e `XDG_CACHE_HOME` apontando para `/tmp/miupy-capas-runtime/`, o binário **permaneceu aberto por 15 segundos**, sem pânico, e o `timeout` encerrou com código 124. Não havia biblioteca real nesse diretório temporário, portanto as capas não foram inspecionadas visualmente.
- Após a última compilação, a execução com esses diretórios temporários e `GSETTINGS_BACKEND=memory` **permaneceu aberta por 5 segundos** (código 124 do `timeout`), sem pânico na asserção que consulta `uri_scheme_is_cors_enabled("asset")` no contexto WebKit real.
- `git diff --check`: **passou**.

## Pendências e problemas conhecidos

- (Orquestração, após a revisão) A falha foi observada só no `npx tauri dev` (origem `http://localhost:1420`); ninguém reproduziu no build instalado (`tauri://localhost`). O registro CORS vale nos dois e é inofensivo onde não havia problema. Os `expect` de `asset_webkit.rs` viraram `log::error!` (uma falha ali só esconde as capas) e o teste que só chamava uma closure saiu.

- Confirmar as capas numa janela com a biblioteca real. Os testes sem janela não provam o resultado visual no WebKitGTK, embora o binário tenha iniciado e o registro do esquema esteja coberto.
- A tentativa de abrir o binário com dados temporários ainda emitiu avisos de dconf e Mesa do ambiente; não houve erro fatal depois de redirecionar os diretórios XDG.

## Próximos passos

1. Em um terminal sem outro Vite na porta 1420, iniciar `npx tauri dev` e abrir DevTools. Na Discoteca, confirmar que uma faixa com `coverPath` carrega a capa; abrir Tocando e conferir a capa grande e a fila. Na aba Network, uma requisição `asset://localhost/...` de capa deve retornar a imagem. O console não deve mostrar `Unsafe attempt to load URL asset://...`; tampouco deve mostrar erro de CSP ou 403 de escopo. `loading="lazy"` exige rolar a lista para testar capas fora da tela.
2. Fechar o modo dev, executar o binário de `npx tauri build --debug --no-bundle` com os dados reais e repetir Discoteca, Tocando e fila, conferindo também a ausência desses erros no console.
3. Se uma capa específica ainda falhar, inspecionar o caminho e a resposta dessa requisição; isso separa arquivo ausente ou fora do escopo do bloqueio de origem corrigido aqui.
