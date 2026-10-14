//! Datas civis em UTC sem dependência externa (algoritmos de Howard Hinnant).

/// Dias desde 1970-01-01 para uma data do calendário gregoriano.
pub(crate) fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Intervalo `[início, fim)` em epoch ms (UTC) do ano civil `year` no fuso `tz_offset_minutes`
/// (minutos a leste de UTC; São Paulo = -180).
pub(crate) fn year_bounds_ms(year: i32, tz_offset_minutes: i32) -> (i64, i64) {
    let off = i64::from(tz_offset_minutes) * 60_000;
    let start = days_from_civil(year, 1, 1) * 86_400_000 - off;
    let end = days_from_civil(year + 1, 1, 1) * 86_400_000 - off;
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-15
    #[test]
    fn datas_conhecidas() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        // 2026-01-01T00:00:00Z
        assert_eq!(days_from_civil(2026, 1, 1) * 86_400_000, 1_767_225_600_000);
    }

    // @test RF-15
    #[test]
    fn ano_local_desloca_pelo_fuso() {
        let (s, e) = year_bounds_ms(2026, -180);
        assert_eq!(s, 1_767_225_600_000 + 3 * 3_600_000);
        assert_eq!(e - s, 365 * 86_400_000);
        let (_, e_bissexto) = year_bounds_ms(2024, 0);
        let (s_bissexto, _) = year_bounds_ms(2024, 0);
        assert_eq!(e_bissexto - s_bissexto, 366 * 86_400_000);
    }
}
