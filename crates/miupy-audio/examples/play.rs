//! Toca um arquivo e mostra o progresso: `cargo run -p miupy-audio --example play -- caminho.m4a [volume 0-1]`
//! Durante a reprodução: `p` pausa/retoma, `+`/`-` ajustam volume, `>`/`<` pulam 10 s (Enter para enviar).

use miupy_audio::{AudioBackend, AudioEvent, PlaybackStatus, RodioBackend};
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next() else {
        eprintln!("uso: play <arquivo> [volume 0.0-1.0]");
        std::process::exit(2);
    };
    let backend = RodioBackend::new().expect("motor");
    if let Some(v) = args.next().and_then(|v| v.parse::<f32>().ok()) {
        backend.set_volume(v).unwrap();
    }
    if let Err(e) = backend.load(Path::new(&file)).and_then(|_| backend.play()) {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
    let events = backend.events();
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut s = String::new();
        while std::io::stdin().read_line(&mut s).is_ok_and(|n| n > 0) {
            let _ = line_tx.send(s.trim().to_string());
            s.clear();
        }
    });
    let fmt = |ms: u64| format!("{}:{:02}", ms / 60_000, ms / 1000 % 60);
    println!("tocando {file} ({})", fmt(backend.state().duration_ms));
    loop {
        if let Ok(cmd) = line_rx.try_recv() {
            let s = backend.state();
            let _ = match cmd.as_str() {
                "p" if s.status == PlaybackStatus::Playing => backend.pause(),
                "p" => backend.resume(),
                "+" => backend.set_volume(s.volume + 0.1),
                "-" => backend.set_volume(s.volume - 0.1),
                ">" => backend.seek(s.position_ms + 10_000),
                "<" => backend.seek(s.position_ms.saturating_sub(10_000)),
                _ => Ok(()),
            };
        }
        match events.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(AudioEvent::Progress {
                position_ms,
                duration_ms,
            }) => {
                println!("{} / {}", fmt(position_ms), fmt(duration_ms));
            }
            Ok(AudioEvent::StateChanged(s)) => {
                println!("estado: {:?} vol {:.2}", s.status, s.volume)
            }
            Ok(AudioEvent::TrackEnded) => {
                println!("fim da faixa");
                break;
            }
            Ok(AudioEvent::Error { message }) => eprintln!("erro: {message}"),
            Err(_) => {}
        }
    }
}
