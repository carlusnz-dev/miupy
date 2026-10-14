//! Medição EBU R128 de arquivos locais, sem abrir dispositivo de áudio.

use crate::{AudioError, Result};
use ebur128::{EbuR128, Mode};
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessMeasurement {
    pub loudness_lufs: f64,
    pub true_peak_db: f64,
}

// @req RF-28
pub fn measure_file(path: &Path) -> Result<Option<LoudnessMeasurement>> {
    let file = File::open(path).map_err(|source| AudioError::Io {
        path: path.into(),
        source,
    })?;
    let mut builder = Decoder::builder().with_data(BufReader::new(file));
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        builder = builder.with_hint(ext);
    }
    let decoder = builder.build().map_err(|e| AudioError::Decode {
        path: path.into(),
        message: e.to_string(),
    })?;
    let channels = decoder.channels().get() as usize;
    let rate = decoder.sample_rate().get();
    let mut meter = EbuR128::new(channels as u32, rate, Mode::I | Mode::TRUE_PEAK)
        .map_err(|e| AudioError::Unsupported(format!("medição EBU R128: {e}")))?;
    let mut buffer = Vec::with_capacity(4096 * channels);
    for sample in decoder {
        buffer.push(sample);
        if buffer.len() >= 4096 * channels {
            meter
                .add_frames_f32(&buffer)
                .map_err(|e| AudioError::Unsupported(e.to_string()))?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        buffer.truncate(buffer.len() / channels * channels);
        meter
            .add_frames_f32(&buffer)
            .map_err(|e| AudioError::Unsupported(e.to_string()))?;
    }
    let loudness = meter
        .loudness_global()
        .map_err(|e| AudioError::Unsupported(e.to_string()))?;
    if !loudness.is_finite() {
        return Ok(None);
    }
    let peak = (0..channels)
        .map(|ch| meter.true_peak(ch as u32).unwrap_or(0.0))
        .fold(0.0_f64, f64::max);
    if peak <= 0.0 {
        return Ok(None);
    }
    Ok(Some(LoudnessMeasurement {
        loudness_lufs: loudness,
        true_peak_db: 20.0 * peak.log10(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    // @test RF-28
    #[test]
    fn sine_and_silence() {
        let dir = tempfile::tempdir().unwrap();
        for (name, amplitude) in [("tone", 0.1_f32), ("silence", 0.0)] {
            let path = dir.path().join(format!("{name}.wav"));
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut wav = hound::WavWriter::create(&path, spec).unwrap();
            for i in 0..48000 {
                wav.write_sample(
                    amplitude * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin(),
                )
                .unwrap();
            }
            wav.finalize().unwrap();
            let measured = measure_file(&path).unwrap();
            if amplitude == 0.0 {
                assert!(measured.is_none());
            } else {
                assert!((measured.unwrap().loudness_lufs - (-23.05)).abs() < 0.5);
            }
        }
    }
}
