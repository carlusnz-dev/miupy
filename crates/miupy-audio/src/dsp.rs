//! Cadeia: normalização, estágio de EQ, volume, limitador de pico verdadeiro.
use crate::equalizer::Equalizer;
#[cfg(test)]
use ebur128::{EbuR128, Mode};
use rodio::source::SeekError;
use rodio::{SampleRate, Source};
use std::num::NonZeroU16;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering, fence};
use std::time::Duration;

fn oversample_coefficients() -> [[f32; 8]; 3] {
    std::array::from_fn(|phase| {
        let t = (phase + 1) as f32 / 4.;
        let mut weights = std::array::from_fn(|tap| {
            let x = (tap as f32 - 3.) - t;
            if x.abs() < 0.0001 {
                1.
            } else {
                let pix = std::f32::consts::PI * x;
                (pix.sin() / pix) * (pix / 4.).sin() / (pix / 4.)
            }
        });
        let sum: f32 = weights.iter().sum();
        for v in &mut weights {
            *v /= sum;
        }
        weights
    })
}

pub(crate) struct DspControls {
    pub normalization: AtomicU32,
    pub volume: AtomicU32,
    equalizer: [AtomicU32; 8],
    /// Seqlock dos ganhos do EQ: ímpar enquanto um escritor grava.
    eq_version: AtomicU32,
    eq_writer: Mutex<()>,
}
impl Default for DspControls {
    fn default() -> Self {
        Self {
            normalization: AtomicU32::new(1.0_f32.to_bits()),
            volume: AtomicU32::new(1.0_f32.to_bits()),
            equalizer: std::array::from_fn(|_| AtomicU32::new(0.0_f32.to_bits())),
            eq_version: AtomicU32::new(0),
            eq_writer: Mutex::new(()),
        }
    }
}

impl DspControls {
    // @req RF-29
    pub fn set_equalizer(&self, gains: [f32; 8]) {
        // Só serializa escritores; a thread de áudio nunca toma este lock.
        let _guard = self.eq_writer.lock().unwrap_or_else(|e| e.into_inner());
        let version = self.eq_version.load(Ordering::Relaxed);
        self.eq_version
            .store(version.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        for (atom, gain) in self.equalizer.iter().zip(gains) {
            atom.store(gain.to_bits(), Ordering::Relaxed);
        }
        self.eq_version
            .store(version.wrapping_add(2), Ordering::Release);
    }

    /// Lê os 8 ganhos de um mesmo perfil, sem lock nem alocação. Devolve `None` se um
    /// escritor não largou a tempo (o chamador mantém os ganhos anteriores).
    // @req RF-29
    pub fn read_equalizer(&self) -> Option<[f32; 8]> {
        for _ in 0..8 {
            let before = self.eq_version.load(Ordering::Acquire);
            if before & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let gains =
                std::array::from_fn(|i| f32::from_bits(self.equalizer[i].load(Ordering::Relaxed)));
            fence(Ordering::Acquire);
            if self.eq_version.load(Ordering::Relaxed) == before {
                return Some(gains);
            }
        }
        None
    }
}

/// Limita blocos de 5 ms com um bloco de antecipação e interpolação sinc 4×.
pub(crate) struct DspSource<S> {
    input: S,
    controls: Arc<DspControls>,
    eq: Equalizer,
    eq_gains: [f32; 8],
    gain: f32,
    channel: usize,
    channels: usize,
    block_len: usize,
    pending: Vec<f32>,
    future: Vec<f32>,
    output: Vec<f32>,
    output_index: usize,
    history: Vec<f32>,
    oversample: [[f32; 8]; 3],
    limiter_gain: f32,
}
impl<S: Source> DspSource<S> {
    pub fn new(input: S, controls: Arc<DspControls>) -> Self {
        let channels = input.channels().get() as usize;
        let rate = input.sample_rate().get();
        let block_len = ((rate as usize / 200).max(1)) * channels;
        // Começa no ganho já pedido para a faixa, sem 10 ms de rampa a partir de 0 dB.
        let gain = f32::from_bits(controls.normalization.load(Ordering::Relaxed));
        Self {
            input,
            controls,
            eq: Equalizer::new(channels, rate as f32),
            eq_gains: [0.; 8],
            gain,
            channel: 0,
            channels,
            block_len,
            pending: Vec::with_capacity(block_len),
            future: Vec::with_capacity(block_len),
            output: Vec::with_capacity(block_len),
            output_index: 0,
            history: vec![0.; channels * 4],
            oversample: oversample_coefficients(),
            limiter_gain: 1.0,
        }
    }

    fn fill_future(&mut self) {
        while self.future.len() < self.block_len {
            let Some(sample) = self.input.next() else {
                break;
            };
            let target = f32::from_bits(self.controls.normalization.load(Ordering::Relaxed));
            let step = 1.0 / (self.input.sample_rate().get() as f32 * 0.010);
            self.gain += (target - self.gain).clamp(-step, step);
            let normalized = sample * self.gain;
            if self.channel == 0 {
                if let Some(gains) = self.controls.read_equalizer()
                    && gains != self.eq_gains
                {
                    self.eq.set(gains);
                    self.eq_gains = gains;
                }
            }
            let equalized = self.eq.process(normalized, self.channel);
            let volume = f32::from_bits(self.controls.volume.load(Ordering::Relaxed));
            self.future.push(equalized * volume);
            self.channel = (self.channel + 1) % self.channels;
        }
    }

    fn prepare_block(&mut self) -> bool {
        if self.pending.is_empty() {
            self.fill_future();
            std::mem::swap(&mut self.pending, &mut self.future);
        }
        if self.pending.is_empty() {
            return false;
        }
        if self.future.is_empty() {
            self.fill_future();
        }

        // @req RF-28: interpolação sinc polifásica 4×, com histórico entre blocos.
        let mut peak = 0.0_f32;
        let mut current_peak = 0.0_f32;
        let count = self.pending.len() + self.future.len();
        let sample_at = |index: isize, this: &Self| -> f32 {
            if index < 0 {
                return this
                    .history
                    .get((this.history.len() as isize + index) as usize)
                    .copied()
                    .unwrap_or(0.);
            }
            let index = index as usize;
            if index < this.pending.len() {
                this.pending[index]
            } else {
                this.future
                    .get(index - this.pending.len())
                    .copied()
                    .unwrap_or(0.)
            }
        };
        for index in 0..count {
            let amplitude = sample_at(index as isize, self).abs();
            peak = peak.max(amplitude);
            if index < self.pending.len() {
                current_peak = current_peak.max(amplitude);
            }
            for phase in &self.oversample {
                let mut value = 0.;
                for (tap, weight) in phase.iter().enumerate() {
                    let offset = (tap as isize - 3) * self.channels as isize;
                    value += weight * sample_at(index as isize + offset, self);
                }
                peak = peak.max(value.abs());
                if index < self.pending.len() {
                    current_peak = current_peak.max(value.abs());
                }
            }
        }
        let ceiling = 10.0_f32.powf(-1.2 / 20.0);
        let required = if peak > ceiling { ceiling / peak } else { 1.0 };
        let release = self.block_len as f32
            / self.channels as f32
            / (self.input.sample_rate().get() as f32 * 0.100);
        let previous_gain = self.limiter_gain;
        self.limiter_gain = required.min(self.limiter_gain + release);
        // @req RF-28: rampa por amostra, limitada pelo pico do bloco atual.
        let safe_current = if current_peak > ceiling {
            ceiling / current_peak
        } else {
            1.
        };
        self.output.clear();
        for (i, &sample) in self.pending.iter().enumerate() {
            let t = (i + 1) as f32 / self.pending.len() as f32;
            let gain = (previous_gain + (self.limiter_gain - previous_gain) * t).min(safe_current);
            self.output.push(sample * gain);
        }
        for &sample in &self.pending {
            self.history.rotate_left(1);
            *self.history.last_mut().unwrap() = sample;
        }
        self.output_index = 0;
        self.pending.clear();
        std::mem::swap(&mut self.pending, &mut self.future);
        self.fill_future();
        true
    }
}
impl<S: Source> Iterator for DspSource<S> {
    type Item = f32;
    // @req RF-28
    fn next(&mut self) -> Option<f32> {
        if self.output_index >= self.output.len() && !self.prepare_block() {
            return None;
        }
        let sample = self.output[self.output_index];
        self.output_index += 1;
        Some(sample)
    }
}
impl<S: Source> Source for DspSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }
    fn channels(&self) -> NonZeroU16 {
        self.input.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)?;
        self.pending.clear();
        self.future.clear();
        self.output.clear();
        self.output_index = 0;
        self.channel = 0;
        self.limiter_gain = 1.0;
        self.history.fill(0.);
        self.eq.reset();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;
    // @test RF-28 RF-29
    #[test]
    fn changing_gain_and_profile_has_no_large_step() {
        let controls = Arc::new(DspControls::default());
        let samples: Vec<f32> = (0..48000)
            .map(|i| 0.3 * (2. * std::f32::consts::PI * 440. * i as f32 / 48000.).sin())
            .collect();
        let mut src = DspSource::new(
            SamplesBuffer::new(
                std::num::NonZeroU16::new(1).unwrap(),
                std::num::NonZeroU32::new(48000).unwrap(),
                samples,
            ),
            Arc::clone(&controls),
        );
        for _ in 0..2000 {
            src.next();
        }
        controls
            .normalization
            .store(2.0_f32.to_bits(), Ordering::Relaxed);
        controls.set_equalizer([6., 5., 3., 1., 0., 0., 0., 0.]);
        let mut prev = src.next().unwrap();
        let mut largest = 0.0_f32;
        for sample in src.take(2000) {
            largest = largest.max((sample - prev).abs());
            prev = sample;
        }
        assert!(largest < 0.15, "degrau: {largest}");
    }
    // @test RF-29
    #[test]
    fn equalizer_reads_never_mix_profiles() {
        let controls = Arc::new(DspControls::default());
        let writer = {
            let controls = Arc::clone(&controls);
            std::thread::spawn(move || {
                for i in 0..200_000u32 {
                    controls.set_equalizer([(i % 2) as f32 + 1.; 8]);
                }
            })
        };
        let mut seen = 0;
        while !writer.is_finished() {
            if let Some(g) = controls.read_equalizer() {
                assert!(g.iter().all(|v| *v == g[0]), "mistura: {g:?}");
                seen += 1;
            }
        }
        writer.join().unwrap();
        assert!(seen > 0);
        assert_eq!(controls.read_equalizer(), Some([2.; 8]));
    }
    // @test RF-28 RF-29
    #[test]
    fn seek_clears_delayed_samples() {
        let controls = Arc::new(DspControls::default());
        let samples = [vec![0.8; 5000], vec![0.; 5000]].concat();
        let mut src = DspSource::new(
            SamplesBuffer::new(
                std::num::NonZeroU16::new(1).unwrap(),
                std::num::NonZeroU32::new(48000).unwrap(),
                samples,
            ),
            controls,
        );
        src.next();
        src.try_seek(Duration::from_secs_f64(5000. / 48000.))
            .unwrap();
        assert_eq!(src.next(), Some(0.));
    }
    // @test RF-28
    #[test]
    fn boosted_signal_stays_below_true_peak_ceiling() {
        let sine = |hz: f32| {
            (0..48000)
                .map(|i| 0.6 * (2.0 * std::f32::consts::PI * hz * i as f32 / 48000.0).sin())
                .collect::<Vec<f32>>()
        };
        let impulse = (0..48000)
            .map(|i| if i % 101 == 0 { 0.9 } else { 0.0 })
            .collect::<Vec<f32>>();
        for (name, samples) in [
            ("1 kHz", sine(1000.0)),
            ("19 kHz", sine(19000.0)),
            ("impulso", impulse),
        ] {
            let controls = Arc::new(DspControls::default());
            controls
                .normalization
                .store(10.0_f32.powf(12.0 / 20.0).to_bits(), Ordering::Relaxed);
            let source = DspSource::new(
                SamplesBuffer::new(
                    std::num::NonZeroU16::new(1).unwrap(),
                    std::num::NonZeroU32::new(48000).unwrap(),
                    samples,
                ),
                controls,
            );
            let output: Vec<f32> = source.collect();
            let mut meter = EbuR128::new(1, 48000, Mode::TRUE_PEAK).unwrap();
            meter.add_frames_f32(&output).unwrap();
            let peak_db = 20.0 * meter.true_peak(0).unwrap().log10();
            assert!(peak_db <= -1.0, "{name}: pico verdadeiro {peak_db:.2} dBTP");
        }
    }
}
