//! Fuso horário local, para a retrospectiva (`stats_wrapped`): o núcleo espera os minutos a
//! leste de UTC e nunca lê o relógio do sistema.

use chrono::{DateTime, Offset, TimeZone};

/// Minutos a leste de UTC no instante dado (Brasil, sem horário de verão: -180).
pub fn offset_minutes<Tz: TimeZone>(now: &DateTime<Tz>) -> i32 {
    now.offset().fix().local_minus_utc() / 60
}

/// Deslocamento do fuso local do sistema agora.
pub fn local_offset_minutes() -> i32 {
    offset_minutes(&chrono::Local::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Utc};

    // @test RF-15
    #[test]
    fn minutos_a_leste_de_utc() {
        let brt = FixedOffset::west_opt(3 * 3600).unwrap();
        assert_eq!(offset_minutes(&Utc::now().with_timezone(&brt)), -180);
        let ist = FixedOffset::east_opt(5 * 3600 + 1800).unwrap();
        assert_eq!(offset_minutes(&Utc::now().with_timezone(&ist)), 330);
        assert_eq!(offset_minutes(&Utc::now()), 0);
    }

    // @test RF-15
    #[test]
    fn fuso_local_esta_no_intervalo_aceito_pelo_nucleo() {
        let m = local_offset_minutes();
        assert!(m.abs() <= 24 * 60);
    }
}
