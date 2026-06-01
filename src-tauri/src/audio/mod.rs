//! Audio capture module — records mic AND/OR system audio using cpal
//!
//! LEVEL METERING (Cap-inspired architecture):
//! - AudioLevels uses AtomicU32 (f32 transmuted) — ZERO locks in callbacks
//! - Recording callbacks update levels directly (no separate monitor stream during recording)
//! - Monitor streams only run when IDLE (pre-recording level check)
//! - Frontend polls levels at ~20Hz via get_audio_levels()
//!
//! SYNC ARCHITECTURE:
//! - Streams are created and playing immediately on start()
//! - Samples are DISCARDED until set_armed(true) is called (first video frame)
//! - Both mic and system streams start buffering at the EXACT same instant
//! - On stop(), samples are mixed (overlapped, NOT concatenated) and written to WAV
//!
//! NOISE SUPPRESSION:
//! - nnnoiseless (RNNoise port) replaces hand-rolled spectral subtraction
//! - Works on 480-sample frames at 48kHz — pure Rust, no GPU needed
//! - Noise gate + nnnoiseless + gain normalization pipeline

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// ═══ LOCK-FREE AUDIO LEVEL METERING ═══
// Uses AtomicU32 with f32 transmute — no Mutex, no lock contention in real-time callbacks.
// Inspired by Cap's approach: compute levels inline in every callback.

#[derive(Debug)]
pub struct AudioLevels {
    pub mic_rms: AtomicU32,
    pub mic_peak: AtomicU32,
    pub mic_db: AtomicU32,
    pub sys_rms: AtomicU32,
    pub sys_peak: AtomicU32,
    pub sys_db: AtomicU32,
}

// Serializable version for Tauri IPC
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioLevelsSnapshot {
    pub mic_rms: f32,
    pub mic_peak: f32,
    pub mic_db: f32,
    pub sys_rms: f32,
    pub sys_peak: f32,
    pub sys_db: f32,
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            mic_rms: AtomicU32::new(0.0f32.to_bits()),
            mic_peak: AtomicU32::new(0.0f32.to_bits()),
            mic_db: AtomicU32::new((-60.0f32).to_bits()),
            sys_rms: AtomicU32::new(0.0f32.to_bits()),
            sys_peak: AtomicU32::new(0.0f32.to_bits()),
            sys_db: AtomicU32::new((-60.0f32).to_bits()),
        }
    }
}

impl AudioLevels {
    fn store_mic(&self, peak: f32, rms: f32, db: f32) {
        self.mic_peak.store(peak.to_bits(), Ordering::Relaxed);
        self.mic_rms.store(rms.to_bits(), Ordering::Relaxed);
        self.mic_db.store(db.to_bits(), Ordering::Relaxed);
    }

    fn store_sys(&self, peak: f32, rms: f32, db: f32) {
        self.sys_peak.store(peak.to_bits(), Ordering::Relaxed);
        self.sys_rms.store(rms.to_bits(), Ordering::Relaxed);
        self.sys_db.store(db.to_bits(), Ordering::Relaxed);
    }

    fn load_mic(&self) -> (f32, f32, f32) {
        (
            f32::from_bits(self.mic_peak.load(Ordering::Relaxed)),
            f32::from_bits(self.mic_rms.load(Ordering::Relaxed)),
            f32::from_bits(self.mic_db.load(Ordering::Relaxed)),
        )
    }

    fn load_sys(&self) -> (f32, f32, f32) {
        (
            f32::from_bits(self.sys_peak.load(Ordering::Relaxed)),
            f32::from_bits(self.sys_rms.load(Ordering::Relaxed)),
            f32::from_bits(self.sys_db.load(Ordering::Relaxed)),
        )
    }

    pub fn snapshot(&self) -> AudioLevelsSnapshot {
        let (mic_peak, mic_rms, mic_db) = self.load_mic();
        let (sys_peak, sys_rms, sys_db) = self.load_sys();
        AudioLevelsSnapshot { mic_rms, mic_peak, mic_db, sys_rms, sys_peak, sys_db }
    }

    pub fn reset(&self) {
        self.mic_peak.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.mic_rms.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.mic_db.store(f32::NEG_INFINITY.to_bits(), Ordering::Relaxed);
        self.sys_peak.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.sys_rms.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.sys_db.store(f32::NEG_INFINITY.to_bits(), Ordering::Relaxed);
    }
}

// Global atomic levels — zero contention, updated from any thread
static AUDIO_LEVELS: std::sync::OnceLock<Arc<AudioLevels>> = std::sync::OnceLock::new();

fn global_levels() -> &'static AudioLevels {
    AUDIO_LEVELS.get_or_init(|| Arc::new(AudioLevels::default()))
}

/// Compute peak amplitude and RMS from interleaved samples
#[inline]
fn compute_levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut peak: f32 = 0.0;
    let mut sum_sq: f64 = 0.0;
    for &s in samples {
        let a = s.abs();
        if a > peak {
            peak = a;
        }
        sum_sq += (s as f64) * (s as f64);
    }
    let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
    (peak, rms)
}

/// Convert RMS to dB (full-scale). Returns -60.0 for silence.
#[inline]
fn rms_to_db(rms: f32) -> f32 {
    if rms < 0.001 {
        -60.0
    } else {
        20.0 * (rms as f64).log10() as f32
    }
}

// ═══ AUDIO MONITOR (pre-recording level check) ═══
/// Lightweight streams that only compute levels — no sample buffering.
/// These run ONLY when idle (before recording starts).
/// During recording, the AudioCapture callbacks update levels directly.
struct AudioMonitor {
    _mic_stream: Option<cpal::Stream>,
    _sys_stream: Option<cpal::Stream>,
}

unsafe impl Send for AudioMonitor {}

static AUDIO_MONITOR: std::sync::OnceLock<Mutex<AudioMonitor>> = std::sync::OnceLock::new();

/// Start monitoring audio levels without recording.
/// Only runs when idle — during recording, AudioCapture handles levels.
pub fn start_audio_monitor(source: &AudioSource) -> Result<(), String> {
    let host = cpal::default_host();
    let levels = global_levels();
    let monitor_mutex =
        AUDIO_MONITOR.get_or_init(|| Mutex::new(AudioMonitor { _mic_stream: None, _sys_stream: None }));
    let mut monitor = monitor_mutex.lock().unwrap();

    // Mic monitor stream
    if *source == AudioSource::Mic || *source == AudioSource::Both {
        if monitor._mic_stream.is_none() {
            if let Some(device) = host.default_input_device() {
                if let Ok(supported) = device.default_input_config() {
                    let config: cpal::StreamConfig = supported.clone().into();
                    let stream_result = match supported.sample_format() {
                        cpal::SampleFormat::F32 => device.build_input_stream(
                            &config,
                            move |data: &[f32], _| {
                                let (peak, rms) = compute_levels(data);
                                global_levels().store_mic(peak, rms, rms_to_db(rms));
                            },
                            |e| tracing::warn!("Mic monitor error: {}", e),
                            None,
                        ),
                        cpal::SampleFormat::I16 => device.build_input_stream(
                            &config,
                            move |data: &[i16], _| {
                                let f32_data: Vec<f32> =
                                    data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                                let (peak, rms) = compute_levels(&f32_data);
                                global_levels().store_mic(peak, rms, rms_to_db(rms));
                            },
                            |e| tracing::warn!("Mic monitor error: {}", e),
                            None,
                        ),
                        _ => {
                            tracing::warn!("Unsupported mic format for monitor");
                            return Ok(());
                        }
                    };
                    if let Ok(stream) = stream_result {
                        let _ = stream.play();
                        monitor._mic_stream = Some(stream);
                        tracing::info!("Mic monitor started");
                    }
                }
            }
        }
    }

    // System audio monitor stream (WASAPI loopback)
    if *source == AudioSource::System || *source == AudioSource::Both {
        if monitor._sys_stream.is_none() {
            if let Some(device) = host.default_output_device() {
                if let Ok(supported) = device.default_output_config() {
                    let config: cpal::StreamConfig = supported.clone().into();
                    let stream_result = match supported.sample_format() {
                        cpal::SampleFormat::F32 => device.build_input_stream(
                            &config,
                            move |data: &[f32], _| {
                                let (peak, rms) = compute_levels(data);
                                global_levels().store_sys(peak, rms, rms_to_db(rms));
                            },
                            |e| tracing::warn!("System monitor error: {}", e),
                            None,
                        ),
                        cpal::SampleFormat::I16 => device.build_input_stream(
                            &config,
                            move |data: &[i16], _| {
                                let f32_data: Vec<f32> =
                                    data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                                let (peak, rms) = compute_levels(&f32_data);
                                global_levels().store_sys(peak, rms, rms_to_db(rms));
                            },
                            |e| tracing::warn!("System monitor error: {}", e),
                            None,
                        ),
                        _ => {
                            tracing::warn!("Unsupported system format for monitor");
                            return Ok(());
                        }
                    };
                    if let Ok(stream) = stream_result {
                        let _ = stream.play();
                        monitor._sys_stream = Some(stream);
                        tracing::info!("System audio monitor started");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Stop monitoring streams (called before recording starts to free devices).
pub fn stop_audio_monitor() {
    if let Some(mutex) = AUDIO_MONITOR.get() {
        if let Ok(mut monitor) = mutex.lock() {
            monitor._mic_stream = None;
            monitor._sys_stream = None;
            tracing::info!("Audio monitor stopped");
        }
    }
    global_levels().reset();
}

/// Get current audio levels (called by frontend polling).
/// Returns snapshot — no locks, just atomic loads.
pub fn get_audio_levels() -> AudioLevelsSnapshot {
    global_levels().snapshot()
}

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
    _mic_stream: Option<cpal::Stream>,
    _sys_stream: Option<cpal::Stream>,
}

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

    /// Create and start audio streams.
    /// Recording callbacks ALSO update global levels (lock-free via atomics).
    /// This means visualization works during recording without a separate monitor stream.
    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        self.recording.store(true, Ordering::SeqCst);

        // ── Mic stream ──
        if self.source == AudioSource::Mic || self.source == AudioSource::Both {
            let device = host
                .default_input_device()
                .ok_or("No microphone device found")?;
            let supported = device
                .default_input_config()
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
                        // Always update levels for visualization (lock-free)
                        let (peak, rms) = compute_levels(data);
                        global_levels().store_mic(peak, rms, rms_to_db(rms));
                        // Buffer samples only when armed
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
                        let f32_data: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        // Always update levels (lock-free)
                        let (peak, rms) = compute_levels(&f32_data);
                        global_levels().store_mic(peak, rms, rms_to_db(rms));
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            samples.lock().unwrap().extend_from_slice(&f32_data);
                        }
                    },
                    |err| tracing::error!("Mic stream error: {}", err),
                    None,
                ),
                fmt => return Err(format!("Unsupported mic format: {:?}", fmt)),
            }
            .map_err(|e| format!("Mic stream build failed: {}", e))?;

            stream
                .play()
                .map_err(|e| format!("Mic play failed: {}", e))?;
            self._mic_stream = Some(stream);
            tracing::info!(
                "Mic stream: {}Hz {}ch (armed=false)",
                self.mic_rate,
                self.mic_channels
            );
        }

        // ── System audio stream (WASAPI loopback) ──
        if self.source == AudioSource::System || self.source == AudioSource::Both {
            let device = host
                .default_output_device()
                .ok_or("No output device for system audio loopback")?;
            let supported = device
                .default_output_config()
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
                        // Always update levels (lock-free)
                        let (peak, rms) = compute_levels(data);
                        global_levels().store_sys(peak, rms, rms_to_db(rms));
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
                        let f32_data: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        // Always update levels (lock-free)
                        let (peak, rms) = compute_levels(&f32_data);
                        global_levels().store_sys(peak, rms, rms_to_db(rms));
                        if recording.load(Ordering::Relaxed)
                            && armed.load(Ordering::Relaxed)
                            && !paused.load(Ordering::Relaxed)
                        {
                            samples.lock().unwrap().extend_from_slice(&f32_data);
                        }
                    },
                    |err| tracing::error!("System audio error: {}", err),
                    None,
                ),
                fmt => return Err(format!("Unsupported system format: {:?}", fmt)),
            }
            .map_err(|e| format!("System audio stream failed: {}", e))?;

            stream
                .play()
                .map_err(|e| format!("System audio play failed: {}", e))?;
            self._sys_stream = Some(stream);
            tracing::info!(
                "System audio: {}Hz {}ch (armed=false)",
                self.sys_rate,
                self.sys_channels
            );
        }

        Ok(())
    }

    pub fn set_armed(&self, armed: bool) {
        self.armed.store(armed, Ordering::SeqCst);
        if armed {
            tracing::info!("Audio ARMED — recording synced with video frame 0");
        }
    }

    /// Stop recording, drop streams, process samples, write WAV.
    pub fn stop(&mut self) -> Result<String, String> {
        self.recording.store(false, Ordering::SeqCst);

        // Drop streams immediately — releases WASAPI handles
        self._mic_stream = None;
        self._sys_stream = None;

        // Brief sleep for final callbacks
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mic_raw: Vec<f32> = self.mic_samples.lock().unwrap().drain(..).collect();
        let sys_raw: Vec<f32> = self.sys_samples.lock().unwrap().drain(..).collect();

        tracing::info!(
            "Audio stop: mic={} samples ({}Hz {}ch), sys={} samples ({}Hz {}ch)",
            mic_raw.len(),
            self.mic_rate,
            self.mic_channels,
            sys_raw.len(),
            self.sys_rate,
            self.sys_channels
        );

        if mic_raw.is_empty() && sys_raw.is_empty() {
            tracing::warn!("Audio: no samples captured");
            return Ok(String::new());
        }

        let out_rate = self.mic_rate.max(self.sys_rate).max(44100);
        let out_channels: u32 = 2;

        // Step 1: Resample
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

        // Step 3: MIX
        let mixed: Vec<f32> = match self.source {
            AudioSource::Mic => mic_stereo,
            AudioSource::System => sys_stereo,
            AudioSource::Both => {
                let mic_processed = process_mic_audio(&mic_stereo, &sys_stereo, out_rate, out_channels);

                let config = crate::config::AppConfig::load();
                let sys_vol = config.system_volume.clamp(0.0, 1.0);

                let len = mic_processed.len().max(sys_stereo.len());
                let mut out = Vec::with_capacity(len);
                for i in 0..len {
                    let m = if i < mic_processed.len() { mic_processed[i] } else { 0.0 };
                    let s = if i < sys_stereo.len() { sys_stereo[i] } else { 0.0 };
                    out.push((m + s * sys_vol).clamp(-1.0, 1.0));
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
            writer
                .write_sample((s * i16::MAX as f32) as i16)
                .map_err(|e| format!("WAV write failed: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("WAV finalize failed: {}", e))?;

        let duration_secs = mixed.len() as f64 / (out_rate as f64 * out_channels as f64);
        tracing::info!(
            "Audio saved: {:.2}s, {:?}, {}Hz {}ch",
            duration_secs,
            self.source,
            out_rate,
            out_channels
        );

        // Reset levels
        global_levels().reset();

        Ok(self.output_path.to_string_lossy().to_string())
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESAMPLING & CHANNEL CONVERSION
// ═══════════════════════════════════════════════════════════════════════════

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

fn convert_channels(input: &[f32], from_ch: u32, to_ch: u32) -> Vec<f32> {
    if input.is_empty() || from_ch == to_ch || from_ch == 0 {
        return input.to_vec();
    }

    let from = from_ch as usize;
    let to = to_ch as usize;

    if from == 1 && to == 2 {
        let mut out = Vec::with_capacity(input.len() * 2);
        for &s in input {
            out.push(s);
            out.push(s);
        }
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
            for c in 0..to {
                out.push(frame[c]);
            }
        }
        out
    } else {
        let mut out = Vec::with_capacity(input.len() / from * to);
        for frame in input.chunks_exact(from) {
            for c in 0..to {
                if c < from {
                    out.push(frame[c]);
                } else {
                    out.push(frame[from - 1]);
                }
            }
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MIC POST-PROCESSING: nnnoiseless + noise gate + gain normalization
// ═══════════════════════════════════════════════════════════════════════════

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

    // Step 1: Estimate noise floor from first 200ms
    let noise_samples = (200 * frames_per_ms * ch).min(mic.len());
    let noise_rms = rms(&mic[..noise_samples]);

    let gate_multiplier = 1.0 + config.noise_gate_threshold * 4.0;
    let noise_threshold = noise_rms * gate_multiplier;

    tracing::info!(
        "Mic processing: noise_rms={:.6}, gate_threshold={:.6} (sensitivity={:.1})",
        noise_rms,
        noise_threshold,
        config.noise_gate_threshold
    );

    // Step 2: Noise gate
    let gated = if config.noise_gate_threshold > 0.01 {
        noise_gate(mic, noise_threshold, sample_rate, channels)
    } else {
        mic.to_vec()
    };

    // Step 3: nnnoiseless denoising (replaces hand-rolled spectral subtraction)
    let denoised = if config.noise_reduction > 0.01 {
        nnnoiseless_denoise(&gated, sample_rate, channels, config.noise_reduction)
    } else {
        gated
    };

    // Step 4: Apply mic gain
    let gained: Vec<f32> = if (config.mic_gain - 1.0).abs() > 0.01 {
        denoised
            .iter()
            .map(|&s| (s * config.mic_gain).clamp(-1.0, 1.0))
            .collect()
    } else {
        denoised
    };

    // Step 5: Normalize mic loudness to match system audio
    normalize_to_target(&gained, sys, sample_rate, channels)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

/// nnnoiseless denoising — pure Rust RNNoise port.
/// Works on 480-sample frames at 48kHz mono.
/// For stereo, processes each channel independently.
fn nnnoiseless_denoise(input: &[f32], sample_rate: u32, channels: u32, strength: f32) -> Vec<f32> {
    let ch = channels as usize;

    // nnnoiseless expects mono 480-sample frames at 48kHz
    // If stereo, process L and R independently
    if ch == 2 {
        // Deinterleave
        let mut left = Vec::with_capacity(input.len() / 2);
        let mut right = Vec::with_capacity(input.len() / 2);
        for chunk in input.chunks_exact(2) {
            left.push(chunk[0]);
            right.push(chunk[1]);
        }
        let denoised_l = nnnoiseless_mono(&left, sample_rate, strength);
        let denoised_r = nnnoiseless_mono(&right, sample_rate, strength);
        // Reinterleave
        let mut out = Vec::with_capacity(input.len());
        for (l, r) in denoised_l.iter().zip(denoised_r.iter()) {
            out.push(*l);
            out.push(*r);
        }
        out
    } else {
        nnnoiseless_mono(input, sample_rate, strength)
    }
}

fn nnnoiseless_mono(input: &[f32], sample_rate: u32, strength: f32) -> Vec<f32> {
    let mut denoiser = nnnoiseless::DenoiseState::new();
    let frame_size = nnnoiseless::DenoiseState::FRAME_SIZE; // 480
    let mut output = vec![0.0f32; input.len()];

    // nnnoiseless works at 48kHz. If different rate, we note it but still process
    // (it'll still reduce noise, just less optimally)
    let gain = 1.0 - strength.clamp(0.0, 1.0) * 0.8; // strength maps to gain reduction

    for (i, chunk) in input.chunks(frame_size).enumerate() {
        let mut in_frame = [0.0f32; 480];
        let mut out_frame = [0.0f32; 480];

        // Copy input (pad with zeros if short frame)
        for (j, &s) in chunk.iter().enumerate() {
            in_frame[j] = s;
        }

        denoiser.process_frame(&mut out_frame, &in_frame);

        // Blend denoised with original based on strength
        let start = i * frame_size;
        for j in 0..chunk.len().min(frame_size) {
            if start + j < output.len() {
                // Mix: strength=1.0 → fully denoised, strength=0.0 → original
                output[start + j] = out_frame[j] * (1.0 - gain) + input[start + j] * gain;
            }
        }
    }

    output
}

fn noise_gate(
    input: &[f32],
    threshold: f32,
    sample_rate: u32,
    channels: u32,
) -> Vec<f32> {
    let ch = channels as usize;
    let frame_count = input.len() / ch;
    let mut output = vec![0.0f32; input.len()];

    let window_frames = (sample_rate as usize / 100).max(1);
    let attack_frames = (sample_rate as usize * 5 / 1000).max(1);
    let release_frames = (sample_rate as usize * 50 / 1000).max(1);

    let mut envelope: f32 = 0.0;

    for frame_idx in 0..frame_count {
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
        let local_rms = if count > 0 {
            (sum_sq / count as f64).sqrt() as f32
        } else {
            0.0
        };

        let target = if local_rms > threshold { 1.0f32 } else { 0.0f32 };

        if target > envelope {
            envelope += (target - envelope) / attack_frames as f32;
        } else {
            envelope += (target - envelope) / release_frames as f32;
        }
        envelope = envelope.clamp(0.0, 1.0);

        for c in 0..ch {
            output[frame_idx * ch + c] = input[frame_idx * ch + c] * envelope;
        }
    }

    output
}

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

    let target_rms = sys_rms * 1.5;

    let gain = if mic_rms > 0.0001 {
        (target_rms / mic_rms).clamp(1.0, 12.0)
    } else {
        1.0
    };

    tracing::info!(
        "Mic normalize: mic_rms={:.4}, sys_rms={:.4}, target={:.4}, gain={:.2}x",
        mic_rms,
        sys_rms,
        target_rms,
        gain
    );

    let mut output = Vec::with_capacity(mic.len());
    for &sample in mic {
        let amplified = sample * gain;
        let limited = soft_limit(amplified);
        output.push(limited);
    }

    output
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    if x.abs() < 0.8 {
        x
    } else {
        let sign = x.signum();
        let abs_x = x.abs();
        let compressed = 0.8 + (1.0 - 0.8) * (1.0 - (-((abs_x - 0.8) * 4.0)).exp());
        sign * compressed.min(0.99)
    }
}
