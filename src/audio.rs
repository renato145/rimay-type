use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc;

pub struct AudioCapture {
    stream: cpal::Stream,
    receiver: mpsc::UnboundedReceiver<Vec<f32>>,
    sample_rate: u32,
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
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.stream.play()?;
        Ok(())
    }

    pub async fn collect_until_stopped(&mut self) -> Vec<f32> {
        let mut buffer = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            buffer.extend(chunk);
        }
        buffer
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}
