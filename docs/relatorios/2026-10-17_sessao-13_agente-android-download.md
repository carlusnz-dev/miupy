# Busca e download no Android (RF-45)

- **Data:** 2026-10-17
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** resolvedor RustyPipe para busca, streaming e download M4A no Android.

## O que foi feito

- Registrei RF-45 e documentei o fluxo Android e o sysroot do bindgen.
- Integrei `RustyPipeResolver` à trait inteira e selecionei essa implementação na casca Android.
- O download grava arquivo parcial, publica progresso, valida M4A/AAC e limpa o parcial em falhas ou cancelamento.
- O cliente HTTP da mídia usa raízes WebPKI no Android para evitar o pânico do verificador da plataforma.
- O app não inicia a atualização do yt-dlp no Android e oculta sua seção nas Configurações.

## Decisões

- Mantive o RustyPipe atrás da feature `rustypipe` para compilar o teste de rede no desktop; a casca ativa a feature apenas no Android.
- Não adicionei BotGuard. Um `403 Forbidden` na transferência real impede afirmar que o download funciona; a causa exata ainda precisa de prova no aparelho.
- O teste usa a faixa curta encontrada na busca real e exige arquivo M4A/AAC válido; não aceita lista vazia.

## Verificação

- `cargo fmt --all`: passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `cargo clippy -p miupy -p miupy-media --target aarch64-linux-android -- -D warnings`: passou com NDK e bindgen configurados.
- `cargo test --workspace`: passou; suíte da casca 149 testes, áudio 35, core 103, mídia 90 e doc-test 1; testes de rede ignorados.
- `npm run check` com config temporária sem preprocessamento de CSS: 0 erros, 0 avisos. A cópia literal do `svelte.config.js` em `/tmp` não carregou o plugin; a cópia no `target/` ainda acionava o cache Vite de `node_modules` (EROFS).
- `npm test -- --configLoader runner --config vite.worktree.config.ts`: 24 arquivos, 112 testes passaram.
- `npm run build -- --configLoader runner --config vite.worktree.config.ts`: passou, 218 módulos.
- `node scripts/rastreabilidade.mjs --check`: 61 requisitos, nenhum ID desconhecido.
- Teste de rede `rustypipe_busca_stream_e_download_reais` executado uma vez: busca encontrou `Intro` (`xMV6l2y67rk`), stream M4A de 130 kbps resolvido, mas download falhou com resposta exata `HTTP status client error (403 Forbidden)`.
- Build APK pelo comando Tauri: biblioteca Rust `aarch64` compilou e ligou; Gradle falhou porque `:tauri-android` aponta para o cache Cargo fora da área gravável do sandbox. A tentativa com cópia do módulo em `target/` passou dessa etapa, mas o Gradle avulso depende de `cli-options-server.json`, criado somente durante o build Tauri. APK não produzido.
- `git diff --check` e JSON da arquitetura: passaram. Nenhum commit, push ou PR foi feito.

## Pendências e problemas conhecidos

- A resposta 403 impede validar download e reprodução no celular. Pode envolver PO token; não foi possível atribuir a causa só pela resposta HTTP. O stream direto foi resolvido, mas não foi reproduzido.
- Playlist, recomendações e metadados do RustyPipe compilaram, mas não foram provados na rede; os metadados musicais estruturados do `corrected_track` ainda não têm equivalência verificada nessa implementação.
- O job Android do `release.yml` na frente `chore/ci-android` precisa definir `BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android` com o sysroot do NDK.
- Prova no aparelho pela orquestração: buscar, tocar por streaming, baixar, conferir na discoteca e abrir metadados/letra sem pânico TLS.
- Revisão de design pelo diff: a única mudança visual é ocultar a seção yt-dlp no Android; sem novos tokens, controles ou estados. Playwright não provaria áudio ou rede nativos e não foi rodado.

## Fatias de commit

1. `docs(docs): registra o rf-45 de busca e download no android` — `docs/requisitos/requisitos.md`.
2. `build(deps): adiciona o rustypipe para o android` — `Cargo.lock`, `crates/miupy-media/Cargo.toml`, `src-tauri/Cargo.toml`, `package.json`.
3. `fix(media): evita o pânico do verificador tls no android` — `crates/miupy-media/src/http.rs`, `lib.rs`, `metadata.rs`, `suggest.rs`, `ytdlp_manager.rs`.
4. `feat(media): adiciona o resolvedor do youtube pelo rustypipe` — `crates/miupy-media/src/rustypipe.rs`, `lib.rs`.
5. `test(media): prova busca, stream e download do rustypipe na rede` — `crates/miupy-media/tests/network.rs` (teste falha no 403, conforme acima).
6. `feat(shell): usa o rustypipe no android e dispensa o yt-dlp` — `src-tauri/src/lib.rs`, `state.rs`, `dto.rs`.
7. `feat(ui): esconde o yt-dlp nas configurações do android` — `src/routes/Settings.svelte`.
8. `docs(docs): documenta busca e download no android` — `docs/arquitetura/arquitetura.md`, `arquitetura.json`, `docs/contribuicao/compilacao.md`.
9. `docs(docs): registra a frente android-download da sessão 13` — este relatório.

## Próximos passos

- No aparelho, capturar a resposta da URL M4A e decidir com o autor como tratar o bloqueio 403 sem BotGuard; repetir a prova completa após essa decisão.

## Atualização da orquestração (metade 1)

- Causa do 403 medida no desktop com o cliente iOS do RustyPipe (o único que decifra o player
  atual; TV, Android, Mobile e Desktop falham com `could not get deobf data`/`could not extract
  sig fn name`): pedaços até ~256 KB a partir do início voltam 200, a partir de ~500 KB do arquivo
  voltam 403. É a trava do YouTube para pedidos sem PO token; vale também para o streaming.
- Download passou a ser em pedaços pelo parâmetro `range` (como o yt-dlp) e o 403 vira uma
  mensagem clara. O teste de rede foi dividido: `rustypipe_busca_e_stream_reais` passa;
  `rustypipe_download_real` só passa com o PO token.
- Decisão do autor: gerar o PO token numa WebView oculta (metade 2, frente própria). Esta
  metade entra com a busca e a correção do pânico de TLS.
- O APK foi gerado pela orquestração (`npm run android:apk` com a variável do bindgen); no
  sandbox o Gradle não escreve no módulo do Tauri no cache do Cargo.
