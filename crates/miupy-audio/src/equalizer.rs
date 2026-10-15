//! Filtros biquad RBJ, com estado independente por canal.
use std::f32::consts::PI;
const FREQ: [f32; 8] = [60., 170., 310., 600., 1000., 3000., 6000., 12000.];
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}
impl Biquad {
    fn new(band: usize, gain: f32, rate: f32) -> Self {
        if gain.abs() < 0.001 || FREQ[band] >= rate * 0.49 {
            return Self {
                b0: 1.,
                ..Self::default()
            };
        }
        let a = 10.0_f32.powf(gain / 40.);
        let w = 2. * PI * FREQ[band] / rate;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2. * 0.70710677);
        let (b0, b1, b2, a0, a1, a2) = match band {
            0 => {
                let k = 2. * a.sqrt() * alpha;
                (
                    a * ((a + 1.) - (a - 1.) * cos + k),
                    2. * a * ((a - 1.) - (a + 1.) * cos),
                    a * ((a + 1.) - (a - 1.) * cos - k),
                    (a + 1.) + (a - 1.) * cos + k,
                    -2. * ((a - 1.) + (a + 1.) * cos),
                    (a + 1.) + (a - 1.) * cos - k,
                )
            }
            7 => {
                let k = 2. * a.sqrt() * alpha;
                (
                    a * ((a + 1.) + (a - 1.) * cos + k),
                    -2. * a * ((a - 1.) + (a + 1.) * cos),
                    a * ((a + 1.) + (a - 1.) * cos - k),
                    (a + 1.) - (a - 1.) * cos + k,
                    2. * ((a - 1.) - (a + 1.) * cos),
                    (a + 1.) - (a - 1.) * cos - k,
                )
            }
            _ => (
                1. + alpha * a,
                -2. * cos,
                1. - alpha * a,
                1. + alpha / a,
                -2. * cos,
                1. - alpha / a,
            ),
        };
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.,
            z2: 0.,
        }
    }
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}
fn response_db(filter: Biquad, hz: f32, rate: f32) -> f32 {
    let w = 2. * PI * hz / rate;
    let (sn, cs) = w.sin_cos();
    let c2 = 2. * cs * cs - 1.;
    let s2 = 2. * sn * cs;
    let nr = filter.b0 + filter.b1 * cs + filter.b2 * c2;
    let ni = -filter.b1 * sn - filter.b2 * s2;
    let dr = 1. + filter.a1 * cs + filter.a2 * c2;
    let di = -filter.a1 * sn - filter.a2 * s2;
    10. * ((nr * nr + ni * ni) / (dr * dr + di * di)).log10()
}
// @req RF-29: ajusta os ganhos dos filtros para a resposta somada nas frequências centrais.
fn calibrated(target: [f32; 8], rate: f32) -> [f32; 8] {
    let mut actual = target;
    for _ in 0..6 {
        let mut error = [0.; 8];
        let mut jac = [[0.; 9]; 8];
        for i in 0..8 {
            for j in 0..8 {
                let f = Biquad::new(j, actual[j], rate);
                error[i] += response_db(f, FREQ[i], rate);
                let plus = Biquad::new(j, actual[j] + 0.05, rate);
                jac[i][j] =
                    (response_db(plus, FREQ[i], rate) - response_db(f, FREQ[i], rate)) / 0.05;
            }
            jac[i][8] = target[i] - error[i];
        }
        for j in 0..8 {
            let mut pivot = j;
            for i in j + 1..8 {
                if jac[i][j].abs() > jac[pivot][j].abs() {
                    pivot = i;
                }
            }
            jac.swap(j, pivot);
            let d = jac[j][j];
            if d.abs() < 0.0001 {
                return actual;
            }
            for value in &mut jac[j][j..] {
                *value /= d;
            }
            for i in 0..8 {
                if i != j {
                    let factor = jac[i][j];
                    let row = jac[j];
                    for (value, base) in jac[i][j..].iter_mut().zip(&row[j..]) {
                        *value -= factor * base;
                    }
                }
            }
        }
        for j in 0..8 {
            actual[j] = (actual[j] + jac[j][8]).clamp(-36., 36.);
        }
        if error.iter().zip(target).all(|(a, b)| (*a - b).abs() < 0.01) {
            break;
        }
    }
    actual
}
// @req RF-29
pub(crate) struct Equalizer {
    current: Vec<[Biquad; 8]>,
    next: Vec<[Biquad; 8]>,
    pre: f32,
    next_pre: f32,
    fade: usize,
    fade_len: usize,
    gains: [f32; 8],
    rate: f32,
}
impl Equalizer {
    pub fn new(channels: usize, rate: f32) -> Self {
        let flat = [Biquad {
            b0: 1.,
            ..Biquad::default()
        }; 8];
        Self {
            current: vec![flat; channels],
            next: vec![flat; channels],
            pre: 1.,
            next_pre: 1.,
            fade: 0,
            fade_len: (rate * 0.02) as usize,
            gains: [0.; 8],
            rate,
        }
    }
    pub fn set(&mut self, gains: [f32; 8]) {
        if gains == self.gains {
            return;
        }
        self.gains = gains;
        let calibrated = calibrated(gains, self.rate);
        let filters = std::array::from_fn(|i| Biquad::new(i, calibrated[i], self.rate));
        self.next.fill(filters);
        self.next_pre = 10.0_f32.powf(-gains.iter().copied().fold(0., f32::max) / 20.);
        self.fade = self.fade_len.max(1);
    }
    pub fn process(&mut self, x: f32, ch: usize) -> f32 {
        let mut a = x * self.pre;
        for f in &mut self.current[ch] {
            a = f.process(a);
        }
        if self.fade == 0 {
            return a;
        }
        let mut b = x * self.next_pre;
        for f in &mut self.next[ch] {
            b = f.process(b);
        }
        let t = 1. - self.fade as f32 / self.fade_len.max(1) as f32;
        if ch + 1 == self.current.len() {
            self.fade -= 1;
            if self.fade == 0 {
                std::mem::swap(&mut self.current, &mut self.next);
                self.pre = self.next_pre;
            }
        }
        a * (1. - t) + b * t
    }
    pub fn reset(&mut self) {
        let gains = self.gains;
        *self = Self::new(self.current.len(), self.rate);
        self.set(gains);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // @test RF-29
    #[test]
    fn flat_is_transparent() {
        let mut eq = Equalizer::new(2, 48000.);
        for i in 0..1000 {
            let x = (i as f32 * 0.3).sin();
            assert!((eq.process(x, 0) - x).abs() < 1e-5);
            assert!((eq.process(x, 1) - x).abs() < 1e-5);
        }
    }
    // @test RF-29
    #[test]
    fn pre_gain_prevents_a_boosted_sine_from_clipping() {
        let mut eq = Equalizer::new(1, 48000.);
        eq.set([0., 0., 0., 0., 6., 0., 0., 0.]);
        let max = (0..48000)
            .map(|i| {
                eq.process((2. * PI * 1000. * i as f32 / 48000.).sin(), 0)
                    .abs()
            })
            .fold(0., f32::max);
        assert!(max <= 1.0 + 1e-5, "{max}");
    }
    // @test RF-29
    #[test]
    fn builtin_frequency_response() {
        let profiles: [[f32; 8]; 8] = [
            [0., 0., 0., 0., 0., 0., 0., 0.],
            [6., 5., 3., 1., 0., 0., 0., 0.],
            [0., 0., 0., 0., 1., 3., 5., 6.],
            [-2., -1., 0., 2., 4., 4., 2., 0.],
            [4., 3., -1., -2., -1., 2., 4., 5.],
            [-1., 1., 3., 4., 3., 0., -1., -1.],
            [5., 4., 1., 0., -1., 2., 3., 4.],
            [3., 2., 1., 1., 2., 2., 3., 2.],
        ];
        for gains in profiles {
            for (band, hz) in FREQ.iter().copied().enumerate() {
                let mut eq = Equalizer::new(1, 48000.);
                eq.set(gains);
                let mut input = 0.;
                let mut output = 0.;
                for i in 0..48000 {
                    let x = (2. * PI * hz * i as f32 / 48000.).sin();
                    let y = eq.process(x, 0);
                    if i > 24000 {
                        input += x * x;
                        output += y * y;
                    }
                }
                let db = 10. * (output / input).log10();
                let pre = gains.iter().copied().fold(0., f32::max);
                assert!(
                    (db - (gains[band] - pre)).abs() < 0.5,
                    "perfil {gains:?}, banda {hz}, real {db:.2}, esperado {:.2}",
                    gains[band] - pre
                );
            }
        }
    }
}
