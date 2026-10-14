# Crate miupy-audio: motor de reprodução

- **Data:** 2026-10-13
- **Autor:** agente audio (subagente do Claude Code, orquestrado por Claude Opus 5.5 a pedido de Carlos Antunes)
- **Modelo:** Claude Sonnet, esforço medium
- **Escopo:** `crates/miupy-audio`: trait `AudioBackend`, backend rodio, backend nulo, testes, exemplo, README.

## O que foi feito
- `error.rs` (`AudioError`, thiserror), `types.rs` (trait, `PlaybackState`, `PlaybackStatus`, `AudioEvent`, `volume_to_gain`), `null.rs` (`NullBackend`, relógio manual), `rodio_backend.rs` (`RodioBackend`, só fora do Android), `lib.rs` (`create_default_backend`).
- `examples/play.rs` (progresso; comandos p, +, -, >, < via stdin) e `README.md`.
- Marcações `@req RF-01/02/03` nas implementações e `@test` nos testes.
- Dependências: rodio 0.23 (features padrão já incluem mp4/aac, mp3, flac, vorbis, wav), thiserror 2, crossbeam-channel 0.5; hound 3.5 em dev.

## Decisões
- Trait com `&self` e `Send + Sync` (estado interno sincronizado) para caber em `Arc<dyn AudioBackend>`.
- Eventos por canal crossbeam, único consumidor; cada evento `StateChanged` leva o estado completo.
- `TrackEnded` só no fim natural; `stop`/`load` nunca emitem. Emitido logo após `StateChanged(Stopped)`.
- `load` termina em `Paused`; `play`/`resume` são equivalentes.
- Duração: usa `total_duration()` do decoder; se `None`, thread de segundo plano conta amostras decodificadas e publica `StateChanged` (`duration_ms` fica 0 até lá).
- Volume: ganho = volume² (perceptual simples); mudo zera o ganho sem perder o volume.
- Dispositivo aberto no primeiro `load`; erro `NoOutputDevice`, sem panic.
- Posições em `u64` ms (alinha com o IPC).

## Verificação (comandos rodados e resultado real)
- `cargo fmt -p miupy-audio --check`: sem diferenças.
- `cargo clippy -p miupy-audio --all-targets`: `Finished`, zero warnings.
- `cargo test -p miupy-audio -- --nocapture`: `13 passed; 0 failed` (7 do NullBackend, 6 do RodioBackend).
- Os 6 testes do rodio **rodaram de verdade** nesta máquina (nenhuma mensagem `PULADO:`; havia dispositivo de saída). Rodaram mudos (`set_muted(true)`).

## Pendências e problemas conhecidos
- Não testado com arquivos reais .m4a/.mp3/.flac (só WAV gerado); o fallback de duração foi testado apenas com WAV (via `probe_duration`), não com um M4A sem duração.
- Pausa/seek dependem de `Player::get_pos` do rodio; a precisão da posição é de dezenas de ms.
- Seek em decoders sem suporte devolve `AudioError::Seek`.
- Android: sem implementação real; `create_default_backend` devolve o `NullBackend` (silencioso). Compilação para Android não foi verificada.
- Erros assíncronos (`AudioEvent::Error`) existem no contrato mas o rodio backend ainda não os emite (falha de decodificação no meio da faixa resulta em fim da faixa).
- `docs:matriz` não foi rodado (fora do escopo de escrita).

## Próximos passos
- Camada Tauri: consumir `events()` em uma thread e emitir `player://state`/`player://progress`; avançar a fila em `TrackEnded`.
- Testar manualmente com `examples/play.rs` em .m4a baixado.
- Plugin Kotlin/Media3 para Android.
