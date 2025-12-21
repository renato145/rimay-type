use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;
use tokio::sync::mpsc;

pub struct AudioCapture {
    stream: cpal::Stream,
    receiver: mpsc::UnboundedReceiver<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
}

impl AudioCapture {
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device found.")?;
        let config = device
            .default_input_config()
            .context("No default config found.")?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        let (tx, rx) = mpsc::unbounded_channel();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = tx.send(data.to_vec()); // ignore send errors
            },
            |e| {
                tracing::error!(error.cause_chain=?e, error.message=%e, "Audio stream error.");
            },
            None,
        )?;

        Ok(Self {
            stream,
            receiver: rx,
            sample_rate,
            channels,
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.stream.play()?;
        Ok(())
    }

    pub fn collect_until_stopped(&mut self) -> anyhow::Result<Option<Vec<u8>>> {
        let mut buffer = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            buffer.extend(chunk);
        }
        let wav_bytes = encode_wav(&buffer, self.sample_rate, self.channels)?;
        Ok(wav_bytes)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

const MIN_DURATION_SECS: f32 = 0.5;

#[tracing::instrument(skip(samples), fields(wav_size))]
pub fn encode_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> anyhow::Result<Option<Vec<u8>>> {
    let min_samples = (sample_rate as f32 * MIN_DURATION_SECS) as usize;
    if samples.len() < min_samples {
        tracing::info!("Skipping short audio.");
        return Ok(None);
    }

    let samples = convert_to_mono(samples, channels);
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = WavWriter::new(&mut cursor, spec).context("Failed to create WavWriter.")?;
    for sample in samples {
        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(sample_i16)
            .context("Failed to write sample.")?;
    }
    writer.finalize().context("Failed to finalize writer.")?;
    let wav_bytes = cursor.into_inner();
    let wav_size_mb = wav_bytes.len() as f64 / 1024f64.powi(2);
    tracing::Span::current().record("wav_size", format!("{wav_size_mb:.2} MB"));
    Ok(Some(wav_bytes))
}

pub fn convert_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec(); // already mono
    }
    samples
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
