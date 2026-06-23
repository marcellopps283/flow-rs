use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

pub struct AudioRecorder {
    stream: Option<cpal::Stream>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self { stream: None }
    }

    pub fn start_recording(&mut self, tx: mpsc::Sender<f32>) -> Result<(), anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("Failed to get default input device"))?;

        let config = device.default_input_config()?;
        let sample_format = config.sample_format();
        let config: cpal::StreamConfig = config.into();

        let err_fn = |err| eprintln!("An error occurred on the input audio stream: {}", err);

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let _ = tx.send(sample);
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let f32_sample = sample as f32 / i16::MAX as f32;
                        let _ = tx.send(f32_sample);
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let f32_sample = (sample as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0);
                        let _ = tx.send(f32_sample);
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        self.stream = Some(stream);
        
        Ok(())
    }

    pub fn stop_recording(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}
