//! Audio capture module — records mic input to WAV file using cpal

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    sample_rate: u32,
    channels: u32,
    output_path: PathBuf,
}

impl AudioCapture {
    pub fn new(output_path: String, sample_rate: Option<u32>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No audio input device found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {}", e))?;

        let sr = sample_rate.unwrap_or(config.sample_rate().0);
        let channels = config.channels() as u32;

        tracing::info!(
            "Audio: {} {}Hz {}ch",
            device.name().unwrap_or_default(),
            sr,
            channels
        );

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            recording: Arc::new(AtomicBool::new(false)),
            sample_rate: sr,
            channels,
            output_path: PathBuf::from(output_path),
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No audio input device found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {}", e))?;

        let samples = self.samples.clone();
        let recording = self.recording.clone();
        self.recording.store(true, Ordering::Relaxed);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if recording.load(Ordering::Relaxed) {
                        samples.lock().unwrap().extend_from_slice(data);
                    }
                },
                |err| tracing::error!("Audio error: {}", err),
                None,
            ),
            cpal::SampleFormat::I16 => {
                let samples = self.samples.clone();
                let recording = self.recording.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if recording.load(Ordering::Relaxed) {
                            let mut buf = samples.lock().unwrap();
                            for &s in data {
                                buf.push(s as f32 / i16::MAX as f32);
                            }
                        }
                    },
                    |err| tracing::error!("Audio error: {}", err),
                    None,
                )
            }
            fmt => return Err(format!("Unsupported format: {:?}", fmt)),
        }
        .map_err(|e| format!("Stream build failed: {}", e))?;

        stream.play().map_err(|e| format!("Stream play failed: {}", e))?;
        // Leak stream — stopped via AtomicBool flag in callback
        std::mem::forget(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<String, String> {
        self.recording.store(false, Ordering::Relaxed);

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Ok(String::new());
        }

        let spec = hound::WavSpec {
            channels: self.channels as u16,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        if let Some(parent) = self.output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut writer =
            hound::WavWriter::create(&self.output_path, spec).map_err(|e| e.to_string())?;
        for &s in samples.iter() {
            writer
                .write_sample((s * i16::MAX as f32) as i16)
                .map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;

        let dur = samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64);
        tracing::info!("Audio saved: {:.1}s -> {}", dur, self.output_path.display());
        Ok(self.output_path.to_string_lossy().to_string())
    }

    pub fn pause(&self) {
        self.recording.store(false, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.recording.store(true, Ordering::Relaxed);
    }
}
