//! Cadeia: normalização, estágio de EQ, volume, limitador de pico verdadeiro.
use ebur128::{EbuR128, Mode};
use rodio::source::SeekError;
use rodio::{SampleRate, Source};
use std::num::NonZeroU16;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

pub(crate) struct DspControls {
    pub normalization: AtomicU32,
    pub volume: AtomicU32,
}
impl Default for DspControls {
    fn default() -> Self {
        Self {
            normalization: AtomicU32::new(1.0_f32.to_bits()),
            volume: AtomicU32::new(1.0_f32.to_bits()),
        }
    }
}

/// Ponto de extensão do equalizador RF-29. O estágio plano preserva as amostras.
pub(crate) trait EqStage: Send {
    fn process(&mut self, sample: f32, channel: usize) -> f32;
}
pub(crate) struct FlatEq;
impl EqStage for FlatEq {
    fn process(&mut self, sample: f32, _channel: usize) -> f32 {
        sample
    }
}

/// Limita blocos de 5 ms com um bloco de antecipação. O medidor EBU faz
/// interpolação 4× abaixo de 96 kHz e 2× até 192 kHz.
pub(crate) struct DspSource<S> {
    input: S,
    controls: Arc<DspControls>,
    eq: Box<dyn EqStage>,
    gain: f32,
    channel: usize,
    channels: usize,
    block_len: usize,
    pending: Vec<f32>,
    future: Vec<f32>,
    output: Vec<f32>,
    output_index: usize,
    meter: Option<EbuR128>,
    limiter_gain: f32,
}
impl<S: Source> DspSource<S> {
    pub fn new(input: S, controls: Arc<DspControls>) -> Self {
        let channels = input.channels().get() as usize;
        let rate = input.sample_rate().get();
        let block_len = ((rate as usize / 200).max(1)) * channels;
        Self {
            input,
            controls,
            eq: Box::new(FlatEq),
            gain: 1.0,
            channel: 0,
            channels,
            block_len,
            pending: Vec::with_capacity(block_len),
            future: Vec::with_capacity(block_len),
            output: Vec::with_capacity(block_len),
            output_index: 0,
            meter: EbuR128::new(channels as u32, rate, Mode::TRUE_PEAK).ok(),
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

        let sample_peak = self
            .pending
            .iter()
            .chain(&self.future)
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let mut peak = sample_peak;
        if let Some(meter) = &mut self.meter {
            meter.reset();
            if meter.add_frames_f32(&self.pending).is_ok()
                && meter.add_frames_f32(&self.future).is_ok()
            {
                for ch in 0..self.channels {
                    peak = peak.max(meter.true_peak(ch as u32).unwrap_or(0.0) as f32);
                }
            }
        }
        // 0,3 dB de margem para a junção de blocos com ganhos diferentes.
        let ceiling = 10.0_f32.powf(-1.3 / 20.0);
        let required = if peak > ceiling { ceiling / peak } else { 1.0 };
        let release = self.block_len as f32
            / self.channels as f32
            / (self.input.sample_rate().get() as f32 * 0.100);
        self.limiter_gain = required.min(self.limiter_gain + release);
        self.output.clear();
        self.output
            .extend(self.pending.iter().map(|s| s * self.limiter_gain));
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;
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
