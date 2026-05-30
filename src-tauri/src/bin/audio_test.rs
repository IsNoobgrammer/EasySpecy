use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let duration: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(5);
    let output = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "test_audio.wav".to_string());

    println!("EasySpecy Audio Capture Test");
    println!("Duration: {}s", duration);
    println!("Output: {}", output);

    let host = cpal::default_host();

    // List devices
    println!("\nAvailable input devices:");
    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                println!("  - {}", name);
                devices.push(device);
            }
        }
    }

    if devices.is_empty() {
        println!("No input devices found!");
        return Ok(());
    }

    let device = &devices[0];
    let device_name = device.name().unwrap_or_default();
    println!("\nUsing: {}", device_name);

    // Get supported config
    let config = device
        .default_input_config()
        .map_err(|e| anyhow::anyhow!("No input config: {}", e))?;

    println!("Format: {:?}", config.sample_format());
    println!("Channels: {}", config.channels());
    println!("Sample rate: {} Hz", config.sample_rate().0);

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as u32;
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let samples_clone = samples.clone();
    let recording = Arc::new(AtomicBool::new(true));
    let recording_clone = recording.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if recording_clone.load(Ordering::Relaxed) {
                        let mut buf = samples_clone.lock().unwrap();
                        buf.extend_from_slice(data);
                    }
                },
                |err| eprintln!("Audio error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let samples_clone = samples.clone();
            let recording_clone = recording.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if recording_clone.load(Ordering::Relaxed) {
                        let mut buf = samples_clone.lock().unwrap();
                        for &s in data {
                            buf.push(s as f32 / i16::MAX as f32);
                        }
                    }
                },
                |err| eprintln!("Audio error: {}", err),
                None,
            )?
        }
        fmt => {
            println!("Unsupported format: {:?}", fmt);
            return Ok(());
        }
    };

    println!("Recording...");
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_secs(duration));

    recording.store(false, Ordering::Relaxed);
    drop(stream);

    let samples = samples.lock().unwrap();
    println!("Captured {} samples ({:.1}s)", samples.len(), samples.len() as f64 / (sample_rate as f64 * channels as f64));

    // Write WAV
    let spec = hound::WavSpec {
        channels: channels as u16,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&output, spec)?;
    for &sample in samples.iter() {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }
    writer.finalize()?;

    let metadata = std::fs::metadata(&output)?;
    println!("Saved: {} ({} bytes, {:.1} KB)", output, metadata.len(), metadata.len() as f64 / 1024.0);

    Ok(())
}
