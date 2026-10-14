# miupy-audio

Motor de reprodução do miupy. Toca **um arquivo por vez**; fila, próxima/anterior e banco ficam na
camada de cima (`src-tauri`). Não depende de Tauri nem das outras crates.

## API

- `trait AudioBackend: Send + Sync` (métodos com `&self`, serve em `Box`/`Arc`):
  `load(path)`, `play`, `pause`, `resume`, `stop`, `seek(ms)`, `set_volume(0.0..=1.0)`,
  `set_muted`, `state() -> PlaybackState`, `events() -> Receiver<AudioEvent>`.
- `PlaybackState { status, path, position_ms, duration_ms, volume, muted }`;
  `PlaybackStatus = Stopped | Loading | Playing | Paused`.
- `AudioEvent`: `StateChanged(PlaybackState)`, `Progress { position_ms, duration_ms }` (~500 ms
  tocando), `TrackEnded`, `Error { message }`.
  `TrackEnded` só sai no **fim natural**; `stop()` e um novo `load()` nunca o emitem.
- `load` deixa o status em `Paused` (posição 0); depois chame `play`.
- `RodioBackend::new()` (desktop), `NullBackend::new()` (testes/Android), `create_default_backend()`.
- Volume: curva perceptual (ganho = volume²) em `volume_to_gain`.

## Implementações

- `RodioBackend` (`cfg(not(target_os = "android"))`): rodio 0.23 + symphonia (m4a/aac, mp3, flac,
  vorbis, wav; Opus não). Thread própria dona do stream; dispositivo aberto no primeiro `load`;
  sem dispositivo devolve `AudioError::NoOutputDevice`. Se o decoder não informa a duração, ela é
  calculada em segundo plano e chega num `StateChanged`.
- `NullBackend`: relógio manual (`advance(ms)`), `set_next_duration_ms`, `set_fail_loads`.

## Testar

```sh
cargo test -p miupy-audio                       # testes do rodio geram WAV e rodam mudos
cargo run -p miupy-audio --example play -- faixa.m4a 0.5
```

Os testes do `RodioBackend` são pulados (mensagem `PULADO:` com `--nocapture`) se não houver
dispositivo de saída.

## Android

Não usa esta implementação. A reprodução real será um plugin Tauri em Kotlin com Media3/ExoPlayer.
Ponto de extensão: implementar `AudioBackend` sobre a ponte do plugin e devolvê-lo no ramo
`cfg(target_os = "android")` de `create_default_backend` (hoje devolve o `NullBackend`).
