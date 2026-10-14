# miupy-audio

Motor de reprodução do miupy. Toca **um arquivo por vez**; fila, próxima/anterior e banco ficam na
camada de cima (`src-tauri`). Não depende de Tauri nem das outras crates.

## API

- `trait AudioBackend: Send + Sync` (métodos com `&self`, serve em `Box`/`Arc`):
  `load(path)`, `load_url(url, options)`, `play`, `pause`, `resume`, `stop`, `seek(ms)`, `set_volume(0.0..=1.0)`,
  `set_muted`, `set_normalization_gain_db(db)`, `state() -> PlaybackState`, `events() -> Receiver<AudioEvent>`.
- `StreamOptions { headers: Vec<(String, String)>, duration_ms: u64 }`. A casca fornece a URL HTTP
  já resolvida, seus cabeçalhos e a duração conhecida; `0` significa duração desconhecida.
- `PlaybackState { status, path, url, position_ms, duration_ms, volume, muted }`;
  `PlaybackStatus = Stopped | Loading | Playing | Paused`.
- `AudioEvent`: `StateChanged(PlaybackState)`, `Progress { position_ms, duration_ms }` (~500 ms
  tocando), `TrackEnded`, `Error { message }`.
  `TrackEnded` só sai no **fim natural**; `stop()` e um novo `load()` nunca o emitem.
- `load` deixa o status em `Paused` (posição 0); depois chame `play`.
- `load_url` retorna antes da rede; o status é `Loading` até o decoder ficar pronto. `play()`
  durante `Loading` agenda o início. Falhas assíncronas chegam por `AudioEvent::Error` e deixam o
  backend em `Stopped`. `state().url` identifica a faixa remota.
- `RodioBackend::new()` (desktop), `NullBackend::new()` (testes/Android), `create_default_backend()`.
- Volume: curva perceptual (ganho = volume²) em `volume_to_gain`.
- `measure_file(path)` mede loudness integrado e pico verdadeiro EBU R128 sem dispositivo de áudio;
  silêncio devolve `None`. No desktop, o rodio aplica normalização com rampa de 10 ms, EQ plano,
  volume e limitador de pico verdadeiro no fim da cadeia.

## Implementações

- `RodioBackend` (`cfg(not(target_os = "android"))`): rodio 0.23 + symphonia (m4a/aac, mp3, flac,
  vorbis, wav; Opus não). Thread própria dona do stream; dispositivo aberto no primeiro `load`;
  sem dispositivo devolve `AudioError::NoOutputDevice`. Se o decoder não informa a duração, ela é
  calculada em segundo plano e chega num `StateChanged`. Streaming usa `stream-download` com
  requisições Range e cache em memória; a abertura da rede roda em thread separada. A duração
  passada em `StreamOptions` aparece em `state()` desde `Loading`. Erros de leitura, saída e abertura
  emitem `Error`; `TrackEnded` fica reservado ao fim natural.
- `NullBackend`: relógio manual (`advance(ms)`), `set_next_duration_ms`, `set_fail_loads`;
  `load_url` simula uma faixa pronta imediatamente, com a duração recebida.

## Testar

```sh
cargo test -p miupy-audio                       # fixtures M4A, fMP4 e MP3, mais servidor HTTP local
cargo run -p miupy-audio --example play -- faixa.m4a 0.5
```

Os testes do `RodioBackend` são pulados (mensagem `PULADO:` com `--nocapture`) se não houver
dispositivo de saída.

As fixtures em `tests/fixtures/` foram geradas com `ffmpeg` (tom de 440 Hz): quatro têm dois
segundos e a usada para queda de rede tem cinco segundos.
`tone-no-duration.m4a` tem os campos de duração dos átomos MP4 zerados; a duração é obtida
contando as amostras. No streaming o decoder busca por bytes, o que torna o seek imediato; em troca, a faixa só fica
pronta depois que o arquivo inteiro chegou (o Symphonia percorre os átomos do MP4 até o fim).
Medido contra o YouTube real em 2026-10-13: primeiro áudio entre 1,5 e 3,5 s e seek para 1:00
em menos de 40 ms. A abertura tem prazo de 30 s; depois de pronta, a faixa é dada como
interrompida se o decoder esperar a rede por mais de 8 s. Há um teste de rede ignorado por
padrão (`real_stream_plays_seeks_and_keeps_playing`) que recebe a URL por `MIUPY_STREAM_URL`.

## Android

Não usa esta implementação. A reprodução real será um plugin Tauri em Kotlin com Media3/ExoPlayer.
Ponto de extensão: implementar `AudioBackend` sobre a ponte do plugin e devolvê-lo no ramo
`cfg(target_os = "android")` de `create_default_backend` (hoje devolve o `NullBackend`).
