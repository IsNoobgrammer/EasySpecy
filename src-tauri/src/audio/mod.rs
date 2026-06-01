//! Audio capture module — records mic AND/OR system audio using cpal
//!
//! SYNC ARCHITECTURE:
//! - Streams are created and playing immediately on start()
//! - Samples are DISCARDED until set_armed(true) is called (first video frame)
//! - Both mic and system streams start buffering at the EXACT same instant
//! - On stop(), samples are mixed (overlapped, NOT concatenated) and written to WAV
//!
//! MIXING (Both mode):
//! - Both streams record simultaneously from the same armed instant
//! - Resampling is frame-aware (respects channel interleaving)
//! - Final mix: out[i] = clamp(mic[i] + system[i], -1.0, 1.0)
//! - Result: mic and system audio play at the same time in the output
//!
//! STREAM LIFETIME:
//! - Streams are stored in the struct (not forgotten) so they can be properly dropped
//! - This prevents WASAPI exclusive mode from blocking system audio playback

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum AudioSource {
    Mic,
    System,
    Both,
}

pub struct AudioCapture {
    mic_samples: Arc<Mutex<Vec<f32>>>,
    sys_samples: Arc<Mutex<Vec<f32>>>,
    mic_rate: u32,
    mic_channels: u32,
    sys_rate: u32,
    sys_channels: u32,
    output_path: PathBuf,
    source: AudioSource,
    recording: Arc<AtomicBool>,
    armed: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    // Store streams so they are properly dropped (not leaked via mem::forget)
    _mic_stream: Option<cpal::Stream>,
    _sys_stream: Option<cpal::Stream>,
}

// AudioCapture is Send because cpal::Stream is Send
// We need this for the static Mutex<Option<AudioCapture>>
unsafe impl Send for AudioCapture {}

impl AudioCapture {
    pub fn new(output_path: String, source: AudioSource) -> Result<Self, String> {
        tracing::info!("Audio init: source={:?}", source);
        Ok(Self {
            mic_samples: Arc::new(Mutex::new(Vec::with_capacity(48000 * 2 * 60))),
            sys_samples: Arc::new(Mutex::new(Vec::with_capacity(48000 * 2 * 60))),
            mic_rate: 0,
            mic_channels: 0,
            sys_rate: 0,
            sys_channels: 0,
            output_path: PathBuf::from(output_path),
            source,
            recording: Arc::new(AtomicBool::new(false)),
            armed: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            _mic_stream: None,
            _sys_stream: None,
        })
    }

    /// Create and start audio streams. Samples are buffered but NOT recorded
    /// until set_armed(true) is called (synced with first video frame).
    /// Streams are stored in self and properly dropped on stop().
    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        self.recording.store(true, Ordering::SeqCst);

        // ── Mic stream ──
        if self.source == AudioSource::Mic || self.source == AudioSource::Both {
            let device = host.default_input_device()
                .ok_or("No microphone device found")?;
            let supported = device.default_input_config()
                .map_err(|e| format!("Mic config error: {}", e))?;

            self.mic_rate = supported.sample_rate().0;
            self.mic_channels = supported.channels() as u32;

            let samples = self.mic_samples.clone();
            let recording = self.recording.clone();
            let armed = self.armed.clone();
            let paused = self.paused.clone();
            let stream_config: cpal::StreamConfig = supported.clone().into();

            let stream = match supported.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            samples.lock().unwrap().extend_from_slice(data);
                        }
                    },
                    |err| tracing::error!("Mic stream error: {}", err),
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            let mut buf = samples.lock().unwrap();
                            for &s in data {
                                buf.push(s as f32 / i16::MAX as f32);
                            }
                        }
                    },
                    |err| tracing::error!("Mic stream error: {}", err),
                    None,
                ),
                fmt => return Err(format!("Unsupported mic format: {:?}", fmt)),
            }.map_err(|e| format!("Mic stream build failed: {}", e))?;

            stream.play().map_err(|e| format!("Mic play failed: {}", e))?;
            self._mic_stream = Some(stream);
            tracing::info!("Mic stream: {}Hz {}ch (armed=false)", self.mic_rate, self.mic_channels);
        }

        // ── System audio stream (WASAPI loopback) ──
        if self.source == AudioSource::System || self.source == AudioSource::Both {
            let device = host.default_output_device()
                .ok_or("No output device for system audio loopback")?;
            let supported = device.default_output_config()
                .map_err(|e| format!("System audio config error: {}", e))?;

            self.sys_rate = supported.sample_rate().0;
            self.sys_channels = supported.channels() as u32;

            let samples = self.sys_samples.clone();
            let recording = self.recording.clone();
            let armed = self.armed.clone();
            let paused = self.paused.clone();
            let stream_config: cpal::StreamConfig = supported.clone().into();

            let stream = match supported.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            samples.lock().unwrap().extend_from_slice(data);
                        }
                    },
                    |err| tracing::error!("System audio error: {}", err),
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            let mut buf = samples.lock().unwrap();
                            for &s in data {
                                buf.push(s as f32 / i16::MAX as f32);
                            }
                        }
                    },
                    |err| tracing::error!("System audio error: {}", err),
                    None,
                ),
                fmt => return Err(format!("Unsupported system format: {:?}", fmt)),
            }.map_err(|e| format!("System audio stream failed: {}", e))?;

            stream.play().map_err(|e| format!("System audio play failed: {}", e))?;
            self._sys_stream = Some(stream);
            tracing::info!("System audio: {}Hz {}ch (armed=false)", self.sys_rate, self.sys_channels);
        }

        Ok(())
    }

    /// Called by capture module when first video frame arrives.
    pub fn set_armed(&self, armed: bool) {
        self.armed.store(armed, Ordering::SeqCst);
        if armed {
            tracing::info!("Audio ARMED — recording synced with video frame 0");
        }
    }

    /// Stop recording, drop streams, process samples, write WAV.
    /// For "Both" mode: mic and system are MIXED (overlapped), not concatenated.
    pub fn stop(&mut self) -> Result<String, String> {
        self.recording.store(false, Ordering::SeqCst);

        // Drop streams IMMEDIATELY — this releases WASAPI handles
        // and restores normal audio playback on the system
        self._mic_stream = None;
        self._sys_stream = None;

        // Brief sleep to let final audio callbacks complete before we drain
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mic_raw: Vec<f32> = self.mic_samples.lock().unwrap().drain(..).collect();
        let sys_raw: Vec<f32> = self.sys_samples.lock().unwrap().drain(..).collect();

        tracing::info!(
            "Audio stop: mic={} samples ({}Hz {}ch), sys={} samples ({}Hz {}ch)",
            mic_raw.len(), self.mic_rate, self.mic_channels,
            sys_raw.len(), self.sys_rate, self.sys_channels
        );

        if mic_raw.is_empty() && sys_raw.is_empty() {
            tracing::warn!("Audio: no samples captured");
            return Ok(String::new());
        }

        // Target format: highest available rate, stereo
        let out_rate = self.mic_rate.max(self.sys_rate).max(44100);
        let out_channels: u32 = 2;

        // Step 1: Resample each source to output rate (frame-aware)
        let mic_resampled = if !mic_raw.is_empty() {
            resample_frames(&mic_raw, self.mic_channels, self.mic_rate, out_rate)
        } else {
            Vec::new()
        };
        let sys_resampled = if !sys_raw.is_empty() {
            resample_frames(&sys_raw, self.sys_channels, self.sys_rate, out_rate)
        } else {
            Vec::new()
        };

        // Step 2: Convert to stereo
        let mic_stereo = convert_channels(&mic_resampled, self.mic_channels, out_channels);
        let sys_stereo = convert_channels(&sys_resampled, self.sys_channels, out_channels);

        // Step 3: MIX (overlap, NOT concatenate)
        let mixed: Vec<f32> = match self.source {
            AudioSource::Mic => mic_stereo,
            AudioSource::System => sys_stereo,
            AudioSource::Both => {
                // ═══ MIC POST-PROCESSING PIPELINE ═══
                // 1. Noise gate (kill low-level background noise)
                // 2. Spectral noise reduction (remove constant hiss/hum)
                // 3. Normalize mic loudness to match system audio level
                let mic_processed = process_mic_audio(&mic_stereo, &sys_stereo, out_rate, out_channels);

                let config = crate::config::AppConfig::load();
                let sys_vol = config.system_volume.clamp(0.0, 1.0);

                // OVERLAP mixing with configurable system volume
                let len = mic_processed.len().max(sys_stereo.len());
                let mut out = Vec::with_capacity(len);
                for i in 0..len {
                    let m = if i < mic_processed.len() { mic_processed[i] } else { 0.0 };
                    let s = if i < sys_stereo.len() { sys_stereo[i] } else { 0.0 };
                    // Mic at full volume, system at user-configured level
                    out.push((m * 1.0 + s * sys_vol).clamp(-1.0, 1.0));
                }
                out
            }
        };

        if mixed.is_empty() {
            return Ok(String::new());
        }

        // Step 4: Write WAV
        let spec = hound::WavSpec {
            channels: out_channels as u16,
            sample_rate: out_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        if let Some(parent) = self.output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut writer = hound::WavWriter::create(&self.output_path, spec)
            .map_err(|e| format!("WAV create failed: {}", e))?;
        for &s in mixed.iter() {
            writer.write_sample((s * i16::MAX as f32) as i16)
                .map_err(|e| format!("WAV write failed: {}", e))?;
        }
        writer.finalize().map_err(|e| format!("WAV finalize failed: {}", e))?;

        let duration_secs = mixed.len() as f64 / (out_rate as f64 * out_channels as f64);
        tracing::info!("Audio saved: {:.2}s, {:?}, {}Hz {}ch", duration_secs, self.source, out_rate, out_channels);

        Ok(self.output_path.to_string_lossy().to_string())
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
}

/// Frame-aware resampling using linear interpolation between frames.
fn resample_frames(input: &[f32], channels: u32, from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == to_rate || from_rate == 0 || channels == 0 {
        return input.to_vec();
    }

    let ch = channels as usize;
    let num_input_frames = input.len() / ch;
    if num_input_frames == 0 {
        return Vec::new();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let num_output_frames = (num_input_frames as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(num_output_frames * ch);

    for frame_idx in 0..num_output_frames {
        let src_pos = frame_idx as f64 / ratio;
        let src_frame = src_pos as usize;
        let frac = (src_pos - src_frame as f64) as f32;

        let frame0 = src_frame.min(num_input_frames - 1);
        let frame1 = (src_frame + 1).min(num_input_frames - 1);

        for c in 0..ch {
            let s0 = input[frame0 * ch + c];
            let s1 = input[frame1 * ch + c];
            output.push(s0 + (s1 - s0) * frac);
        }
    }

    output
}

/// Convert between channel counts (interleaved).
fn convert_channels(input: &[f32], from_ch: u32, to_ch: u32) -> Vec<f32> {
    if input.is_empty() || from_ch == to_ch || from_ch == 0 {
        return input.to_vec();
    }

    let from = from_ch as usize;
    let to = to_ch as usize;

    if from == 1 && to == 2 {
        let mut out = Vec::with_capacity(input.len() * 2);
        for &s in input { out.push(s); out.push(s); }
        out
    } else if from == 2 && to == 1 {
        let mut out = Vec::with_capacity(input.len() / 2);
        for chunk in input.chunks_exact(2) {
            out.push((chunk[0] + chunk[1]) * 0.5);
        }
        out
    } else if from > to {
        let mut out = Vec::with_capacity(input.len() / from * to);
        for frame in input.chunks_exact(from) {
            for c in 0..to { out.push(frame[c]); }
        }
        out
    } else {
        let mut out = Vec::with_capacity(input.len() / from * to);
        for frame in input.chunks_exact(from) {
            for c in 0..to {
                if c < from { out.push(frame[c]); }
                else { out.push(frame[from - 1]); }
            }
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MIC POST-PROCESSING: Noise reduction + loudness normalization
// ═══════════════════════════════════════════════════════════════════════════

/// Full mic processing pipeline for "Both" mode:
/// 1. Estimate noise floor from first 200ms (assumed to be silence/ambient)
/// 2. Apply noise gate (kill anything below threshold)
/// 3. Apply spectral subtraction (remove constant noise profile)
/// 4. Normalize loudness to match system audio RMS
fn process_mic_audio(
    mic: &[f32],
    sys: &[f32],
    sample_rate: u32,
    channels: u32,
) -> Vec<f32> {
    if mic.is_empty() {
        return Vec::new();
    }

    let config = crate::config::AppConfig::load();
    let ch = channels as usize;
    let frames_per_ms = sample_rate as usize / 1000;

    // ═══ Step 1: Estimate noise floor from first 200ms ═══
    let noise_samples = (200 * frames_per_ms * ch).min(mic.len());
    let noise_rms = rms(&mic[..noise_samples]);

    // Use config noise_gate_threshold to scale sensitivity (0.0 = off, 1.0 = aggressive)
    let gate_multiplier = 1.0 + config.noise_gate_threshold * 4.0; // maps 0..1 to 1x..5x
    let noise_threshold = noise_rms * gate_multiplier;

    tracing::info!(
        "Mic processing: noise_rms={:.6}, gate_threshold={:.6} (sensitivity={:.1})",
        noise_rms, noise_threshold, config.noise_gate_threshold
    );

    // ═══ Step 2: Noise gate (skip if threshold is 0) ═══
    let gated = if config.noise_gate_threshold > 0.01 {
        noise_gate(mic, noise_threshold, sample_rate, channels)
    } else {
        mic.to_vec()
    };

    // ═══ Step 3: Spectral noise reduction (skip if reduction is 0) ═══
    let denoised = if config.noise_reduction > 0.01 {
        spectral_subtract(&gated, noise_rms, sample_rate, channels, config.noise_reduction)
    } else {
        gated
    };

    // ═══ Step 4: Apply mic gain from config ═══
    let gained: Vec<f32> = if (config.mic_gain - 1.0).abs() > 0.01 {
        denoised.iter().map(|&s| (s * config.mic_gain).clamp(-1.0, 1.0)).collect()
    } else {
        denoised
    };

    // ═══ Step 5: Normalize mic loudness to match system audio ═══
    let normalized = normalize_to_target(&gained, sys, sample_rate, channels);

    normalized
}

/// Calculate RMS (root mean square) of a signal — measures loudness
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

/// Noise gate with smooth attack/release envelope.
/// Kills audio below threshold, with 5ms attack and 50ms release to avoid clicks.
fn noise_gate(
    input: &[f32],
    threshold: f32,
    sample_rate: u32,
    channels: u32,
) -> Vec<f32> {
    let ch = channels as usize;
    let frame_count = input.len() / ch;
    let mut output = vec![0.0f32; input.len()];

    // Window size for level detection: 10ms
    let window_frames = (sample_rate as usize / 100).max(1);
    // Attack: 5ms, Release: 50ms (in frames)
    let attack_frames = (sample_rate as usize * 5 / 1000).max(1);
    let release_frames = (sample_rate as usize * 50 / 1000).max(1);

    let mut envelope: f32 = 0.0; // 0 = gate closed, 1 = gate open

    for frame_idx in 0..frame_count {
        // Compute local RMS over a window centered on current frame
        let win_start = frame_idx.saturating_sub(window_frames / 2);
        let win_end = (frame_idx + window_frames / 2).min(frame_count);
        let mut sum_sq: f64 = 0.0;
        let mut count = 0;
        for f in win_start..win_end {
            for c in 0..ch {
                let s = input[f * ch + c] as f64;
                sum_sq += s * s;
                count += 1;
            }
        }
        let local_rms = if count > 0 { (sum_sq / count as f64).sqrt() as f32 } else { 0.0 };

        // Gate logic
        let target = if local_rms > threshold { 1.0f32 } else { 0.0f32 };

        // Smooth envelope
        if target > envelope {
            // Attack (fast open)
            envelope += (target - envelope) / attack_frames as f32;
        } else {
            // Release (slow close)
            envelope += (target - envelope) / release_frames as f32;
        }
        envelope = envelope.clamp(0.0, 1.0);

        // Apply envelope
        for c in 0..ch {
            output[frame_idx * ch + c] = input[frame_idx * ch + c] * envelope;
        }
    }

    output
}

/// Spectral subtraction — removes constant background noise.
/// Works by subtracting the estimated noise magnitude from each sample's envelope.
/// This is a simplified time-domain approach (not full FFT) that's fast and effective
/// for constant noise like fan hum, AC, or mic hiss.
/// `strength` controls aggressiveness: 0.0 = none, 1.0 = maximum
fn spectral_subtract(
    input: &[f32],
    noise_rms: f32,
    _sample_rate: u32,
    _channels: u32,
    strength: f32,
) -> Vec<f32> {
    if noise_rms < 0.0001 {
        // Noise floor is negligible, skip processing
        return input.to_vec();
    }

    // Subtraction factor scales with strength: 0.0 → 0x, 0.5 → 1.5x, 1.0 → 3.0x
    let subtract_factor: f32 = strength * 3.0;
    let noise_level = noise_rms * subtract_factor;

    let mut output = Vec::with_capacity(input.len());
    for &sample in input {
        let magnitude = sample.abs();
        if magnitude <= noise_level {
            // Below noise floor — suppress completely
            output.push(0.0);
        } else {
            // Above noise floor — subtract noise and preserve sign
            let cleaned_magnitude = magnitude - noise_level;
            output.push(cleaned_magnitude.copysign(sample));
        }
    }

    output
}

/// Normalize mic audio loudness to match system audio level.
/// Measures RMS of both, computes gain to bring mic up to system level.
/// Applies a limiter to prevent clipping.
fn normalize_to_target(
    mic: &[f32],
    sys: &[f32],
    _sample_rate: u32,
    _channels: u32,
) -> Vec<f32> {
    if mic.is_empty() {
        return Vec::new();
    }

    let mic_rms = rms(mic);
    let sys_rms = if !sys.is_empty() { rms(sys) } else { 0.1 };

    // Target: mic should be at least as loud as system, ideally 1.5x louder
    // so voice cuts through clearly
    let target_rms = sys_rms * 1.5;

    // Calculate required gain
    let gain = if mic_rms > 0.0001 {
        (target_rms / mic_rms).clamp(1.0, 12.0) // Max 12x boost (prevents insane amplification)
    } else {
        1.0 // Mic is silent, don't amplify noise
    };

    tracing::info!(
        "Mic normalize: mic_rms={:.4}, sys_rms={:.4}, target={:.4}, gain={:.2}x",
        mic_rms, sys_rms, target_rms, gain
    );

    // Apply gain with soft limiter
    let mut output = Vec::with_capacity(mic.len());
    for &sample in mic {
        let amplified = sample * gain;
        // Soft limiter (tanh-style) — prevents hard clipping
        let limited = soft_limit(amplified);
        output.push(limited);
    }

    output
}

/// Soft limiter using tanh-like curve.
/// Keeps signal in [-1, 1] without hard clipping artifacts.
#[inline]
fn soft_limit(x: f32) -> f32 {
    if x.abs() < 0.8 {
        x // Linear region — no distortion
    } else {
        // Soft knee: smoothly compress towards ±1
        let sign = x.signum();
        let abs_x = x.abs();
        // Maps [0.8, inf) -> [0.8, 1.0) smoothly
        let compressed = 0.8 + (1.0 - 0.8) * (1.0 - (-((abs_x - 0.8) * 4.0)).exp());
        sign * compressed.min(0.99)
    }
}
