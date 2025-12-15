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
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let (tx, rx) = mpsc::unbounded_channel();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = tx.send(data.to_vec()); // ignore send errors
            },
            |err| eprintln!("audio error: {}", err),
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

    pub fn collect_until_stopped(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            buffer.extend(chunk);
        }
        let wav_bytes = encode_wav(&buffer, self.sample_rate, self.channels)?;
        print_wav_size(&wav_bytes);
        Ok(wav_bytes)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> anyhow::Result<Vec<u8>> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = WavWriter::new(&mut cursor, spec).context("Failed to create WavWriter.")?;
    for &sample in samples {
        writer
            .write_sample(sample)
            .context("Failed to write sample.")?;
    }
    writer.finalize().context("Failed to finalize writer.")?;
    let wav_bytes = cursor.into_inner();
    print_wav_size(&wav_bytes);
    Ok(wav_bytes)
}

fn print_wav_size(wav_bytes: &[u8]) {
    let size_kb = wav_bytes.len() as f64 / 1024.0;
    let size_mb = size_kb / 1024.0;
    println!("WAV size: {:.2} KB ({:.2} MB)", size_kb, size_mb);
}
