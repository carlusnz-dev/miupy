//! Contabilização do tempo realmente ouvido (RF-13), a partir das posições que o motor informa.
//!
//! Regra: só conta o avanço contínuo e natural da posição enquanto toca. Pausa não gera
//! posições (e `pause()` zera a referência); seek (para frente ou para trás) e saltos grandes
//! são descontinuidades e não contam; a posição nova vira a referência.

// @req RF-13

/// Maior avanço entre duas observações consecutivas que ainda é considerado reprodução normal
/// (o motor informa a cada ~500 ms; folga para um sistema ocupado).
pub const MAX_STEP_MS: u64 = 2_000;

#[derive(Debug, Default, Clone)]
pub struct ListenTracker {
    last: Option<u64>,
    pending_ms: u64,
}

impl ListenTracker {
    /// Nova faixa: esquece a referência e o que estava pendente.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Posição observada enquanto toca (progresso ou mudança de estado em `Playing`).
    pub fn observe(&mut self, position_ms: u64) {
        if let Some(last) = self.last
            && position_ms > last
            && position_ms - last <= MAX_STEP_MS
        {
            self.pending_ms += position_ms - last;
        }
        self.last = Some(position_ms);
    }

    /// Pausa ou parada: a próxima observação reinicia a referência, sem contar o intervalo.
    pub fn pause(&mut self) {
        self.last = None;
    }

    /// Seek explícito: a posição de destino vira a referência, sem contar o salto.
    pub fn seek(&mut self, position_ms: u64) {
        self.last = Some(position_ms);
    }

    pub fn pending_ms(&self) -> u64 {
        self.pending_ms
    }

    /// Entrega e zera o que foi acumulado desde a última retirada.
    pub fn take_pending(&mut self) -> u64 {
        std::mem::take(&mut self.pending_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-13
    #[test]
    fn soma_o_avanco_continuo() {
        let mut t = ListenTracker::default();
        for p in [0, 500, 1000, 1500, 2000] {
            t.observe(p);
        }
        assert_eq!(t.take_pending(), 2000);
        assert_eq!(t.take_pending(), 0);
    }

    // @test RF-13
    #[test]
    fn pausa_nao_conta_o_intervalo() {
        let mut t = ListenTracker::default();
        t.observe(0);
        t.observe(500);
        t.pause();
        // ao retomar, a primeira observação só fixa a referência
        t.observe(500);
        t.observe(1000);
        assert_eq!(t.take_pending(), 1000);
    }

    // @test RF-13
    #[test]
    fn seek_para_frente_nao_conta() {
        let mut t = ListenTracker::default();
        t.observe(0);
        t.observe(500);
        t.seek(60_000);
        t.observe(60_500);
        assert_eq!(t.take_pending(), 1000);
    }

    // @test RF-13
    #[test]
    fn seek_para_tras_nao_conta_nem_anula() {
        let mut t = ListenTracker::default();
        t.observe(0);
        t.observe(500);
        t.observe(1000);
        t.seek(0);
        t.observe(500);
        assert_eq!(t.take_pending(), 1500);
    }

    // @test RF-13
    #[test]
    fn salto_sem_aviso_e_descontinuidade() {
        let mut t = ListenTracker::default();
        t.observe(0);
        t.observe(500);
        t.observe(90_000); // seek que não passou por `seek()`
        t.observe(90_500);
        t.observe(100); // voltou
        assert_eq!(t.take_pending(), 1000);
    }

    // @test RF-13
    #[test]
    fn posicao_repetida_nao_soma() {
        let mut t = ListenTracker::default();
        t.observe(1000);
        t.observe(1000);
        assert_eq!(t.pending_ms(), 0);
    }

    // @test RF-13
    #[test]
    fn reset_descarta_tudo() {
        let mut t = ListenTracker::default();
        t.observe(0);
        t.observe(500);
        t.reset();
        t.observe(700);
        assert_eq!(t.pending_ms(), 0);
    }
}
