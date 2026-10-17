# Áudio nativo no Android — primeira metade

- **Data:** 2026-10-17
- **Autor:** Codex
- **Modelo:** GPT-6
- **Escopo:** RF-44: serviço Media3, plugin Kotlin, backend Rust e ponte de eventos.

## O que foi feito

- RF-44, prioridade Should, registrado nos requisitos; dependências Media3 1.11.1 já previstas no Gradle foram mantidas.
- `PlaybackService` usa ExoPlayer e MediaSessionService para tocar em segundo plano, com notificação padrão, foco de áudio e pausa ao desconectar o fone. Manifesto declara serviço e permissões; Android 13+ solicita notificação uma vez.
- `AudioPlugin` recebe arquivo local ou URL com cabeçalhos, controla reprodução no looper principal e envia estado, progresso a cada 500 ms, fim natural e erro por `Channel`.
- `AndroidAudioBackend` implementa toda a trait na casca; `build_state` o seleciona no Android. `state()` lê espelho em `Mutex`. Volume segue a curva `volume²`; normalização só atenua; mudo zera. Ganhos de EQ são guardados nesta metade.
- Cálculo do volume e interpolação das oito bandas em funções puras, com dois testes. IPC, arquitetura e JSON atualizados.

## Decisões

- O backend não muda `miupy-audio` nem `AudioEvent`. Próxima/anterior e metadados da sessão são a segunda metade e ainda não foram ligados à fila.
- O CLI Tauri regenera `tauri.settings.gradle` com a pasta Cargo somente leitura. Para gerar o APK no sandbox, o wrapper Gradle apontou temporariamente para uma cópia em `target/`; o wrapper foi restaurado.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all`: passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `cargo clippy -p miupy --target aarch64-linux-android -- -D warnings`: passou.
- `cargo test --workspace`: falhou no link de `miupy` por `mold: fatal: library not found: webkit2gtk-4.1`.
- `cargo test -p miupy-core -p miupy-audio -p miupy-media`: 228 testes passaram, 13 ignorados, 0 falhas; mais 1 doc-test passou.
- Testes puros novos extraídos do próprio módulo e compilados com `rustc --test`: 2 passaram, 0 falhas. O teste da casca via Cargo ficou impedido pelo link Linux.
- `npm run check` com configuração temporária em `/tmp`: 0 erros, 0 avisos.
- `npm test -- --configLoader runner --config vite.worktree.config.ts`: 107 testes passaram.
- `npm run build -- --configLoader runner --config vite.worktree.config.ts`: passou, 216 módulos.
- `node scripts/rastreabilidade.mjs --check`: passou, 58 requisitos, nenhum ID desconhecido.
- `:app:compileArmDebugKotlin --offline`: passou.
- `npx tauri android build --apk --target aarch64 --debug` com comando Vite de worktree e cópia temporária do módulo Android Tauri: passou; APK em `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk` (140.933.267 bytes).
- `git diff --check`: passou. Nenhum commit, push ou PR foi criado.

### Orquestração (fora do sandbox, 2026-10-17)

- Corrigido em `AudioPlugin.kt`/`PlaybackService.kt`: o `startService` só cria o serviço depois, no
  looper principal, então o primeiro comando (o primeiro `load` ou o volume da abertura) caía em
  "Serviço de áudio indisponível". Agora o comando espera numa fila (`PlaybackService.pending`)
  que o `onCreate` executa.
- `cargo fmt --all --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok;
  `cargo test --workspace`: casca 136 (3 ignorados), áudio 35 (1 ignorado), core 103, media 90, doc-test 1.
  O erro de link do sandbox era real: a `webkit2gtk-4.1` tinha saído do sistema junto com outro
  pacote; o autor a reinstalou.
- `npm run check` 0 erros; `npm test` 107; rastreabilidade ok; `npm run android:apk` ok.
- Revisão do #89 (corrigido): `load` com `playWhenReady = false` (a faixa nova não toca
  sozinha); `setWakeMode(WAKE_MODE_NETWORK)` para a tela apagada; o serviço recriado volta com o
  último volume; `startService` pelo `applicationContext`, com `try/catch` para o app em segundo
  plano; geração nos `load`/`stop` para descartar eventos atrasados (o `explicitStop` era código
  morto); estado otimista em tocar/pausar; atributos de música; volume ausente é rejeitado; testes
  do `gpu.rs` só no Linux.
- Se o serviço não sobe (app em segundo plano com o serviço morto), o erro de geração 0 faz o
  `player.rs` descartar a faixa e avançar; com falhas seguidas, a fila inteira passa. Raro.
- **Prova no celular real** (realme RMX3930, Android 14, adb por Wi-Fi; discoteca copiada do desktop
  com os caminhos trocados): tocar, pausar, buscar, volume e troca de faixa ok, e o som continua
  com a tela apagada. Faltava a notificação: sem `MediaController` conectado, o
  `MediaSessionService` não publicava a sessão. Corrigido com `addSession(session)`. Depois disso,
  a notificação e a tela de bloqueio mostraram anterior, pausar e próxima, e o autor confirmou
  que os três funcionam. Próxima e anterior passam por um `ForwardingPlayer` que manda os
  eventos `next`/`previous` ao Rust, e a casca chama `Player::next`/`previous`, então a interface
  acompanha. Tocar no cartão abre o app (`setSessionActivity`).
- **Defeito aberto:** fechar o app pelos recentes encerra o processo inteiro (o lado Rust sai com
  código 0 e há `pthread_mutex_lock called on a destroyed mutex` no log), e a música para. Um
  player nativo continuaria tocando.
- Fica para a metade 2: `setSessionActivity` (feito na prova acima) (tocar na notificação abre o app); erro de
  reprodução exige novo `prepare()` antes de tocar; serviço recriado pelo sistema não avisa o Rust
  (o espelho segue com a faixa até o próximo `load`).

## Pendências e problemas conhecidos

- Sem aparelho nesta frente: áudio real, continuidade com tela apagada, notificação, tela de bloqueio e fone ainda dependem de prova manual da orquestração.
- Metadados/capa, próxima/anterior da sessão e `audiofx.Equalizer` são a segunda metade. `set_equalizer` só guarda os ganhos até lá.
- O APK requer desvio temporário de build neste sandbox porque a pasta do Tauri no registro Cargo é somente leitura. Não há mudança permanente no wrapper Gradle.

## Roteiro manual no celular

1. Tocar arquivo local e streaming; pausar, retomar, buscar posição, mudar volume e mudo.
2. Apagar a tela e verificar som contínuo, notificação e tocar/pausar na tela bloqueada.
3. Desconectar fone e confirmar pausa. Testar normalização; EQ fica para a segunda metade.

## Fatias de commit

- `docs(docs): registra requisito de áudio nativo no android` — `docs/requisitos/requisitos.md`. Rodapé: `Refs: RF-44`.
- `test(audio): verifica volume e bandas no android` — `src-tauri/src/android_audio.rs` (funções puras e testes). Rodapé: `Refs: RF-02 RF-28 RF-29 RF-44`.
- `feat(audio): adiciona serviço de reprodução media3` — `src-tauri/gen/android/app/build.gradle.kts`, `src-tauri/gen/android/app/src/main/AndroidManifest.xml`, `src-tauri/gen/android/app/src/main/java/dev/miupy/app/PlaybackService.kt`. Rodapé: `Refs: RF-44`.
- `feat(audio): integra plugin e backend android` — `src-tauri/gen/android/app/src/main/java/dev/miupy/app/AudioPlugin.kt`, `src-tauri/src/android_audio.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/gpu.rs`. Rodapé: `Refs: RF-02 RF-18 RF-28 RF-29 RF-44`.
- `docs(docs): documenta áudio android da primeira metade` — `docs/arquitetura/ipc.md`, `docs/arquitetura/arquitetura.md`, `docs/arquitetura/arquitetura.json`, este relatório. Rodapé: `Refs: RF-44`.
