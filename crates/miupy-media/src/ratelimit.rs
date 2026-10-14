//! Limitador de taxa: no máximo uma permissão por intervalo (MusicBrainz: 1 req/s).
//!
//! Usa `tokio::time`, então os testes controlam o tempo com `start_paused`.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

/// Reserva "vagas" em sequência: cada chamada a [`RateLimiter::acquire`] só retorna depois do
/// intervalo mínimo desde a vaga anterior. Seguro para uso concorrente (as chamadas formam fila).
#[derive(Debug)]
pub struct RateLimiter {
    interval: Duration,
    next_slot: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_slot: Mutex::new(Instant::now()),
        }
    }

    // @req RF-12
    /// Espera a vez. A primeira chamada é imediata.
    pub async fn acquire(&self) {
        let wait_until = {
            let mut next = self.next_slot.lock().await;
            let now = Instant::now();
            let slot = (*next).max(now);
            *next = slot + self.interval;
            slot
        };
        tokio::time::sleep_until(wait_until).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn primeira_imediata_e_seguintes_espaçadas() {
        let rl = RateLimiter::new(Duration::from_secs(1));
        let t0 = Instant::now();
        rl.acquire().await;
        assert_eq!(t0.elapsed(), Duration::ZERO);
        rl.acquire().await;
        assert_eq!(t0.elapsed(), Duration::from_secs(1));
        rl.acquire().await;
        assert_eq!(t0.elapsed(), Duration::from_secs(2));
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn nao_acumula_credito_apos_ociosidade() {
        let rl = RateLimiter::new(Duration::from_secs(1));
        rl.acquire().await;
        tokio::time::advance(Duration::from_secs(10)).await;
        let t = Instant::now();
        rl.acquire().await; // imediata
        rl.acquire().await; // deve esperar 1s, sem rajada
        assert_eq!(t.elapsed(), Duration::from_secs(1));
    }

    // @test RF-12
    #[tokio::test(start_paused = true)]
    async fn chamadas_concorrentes_viram_fila() {
        let rl = std::sync::Arc::new(RateLimiter::new(Duration::from_secs(1)));
        let t0 = Instant::now();
        let mut hs = Vec::new();
        for _ in 0..4 {
            let rl = rl.clone();
            hs.push(tokio::spawn(async move {
                rl.acquire().await;
                t0.elapsed()
            }));
        }
        let mut times = Vec::new();
        for h in hs {
            times.push(h.await.unwrap());
        }
        times.sort();
        let secs: Vec<u64> = times.iter().map(|d| d.as_secs()).collect();
        assert_eq!(secs, vec![0, 1, 2, 3]);
    }
}
