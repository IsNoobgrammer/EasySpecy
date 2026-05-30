//! Screen capture module — real implementation using windows-capture
//! Saves directly to MP4 via the built-in VideoEncoder, then merges audio via FFmpeg

use crate::audio::AudioCapture;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingResult {
    pub output_path: String,
    pub duration_secs: f64,
    pub frame_count: u32,
    pub file_size_bytes: u64,
    pub has_audio: bool,
}

#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub output_path: String,
    pub enable_audio: bool,
    pub audio_sample_rate: u32,
    pub fps: u32,
}

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);
static OUTPUT_PATH: Mutex<String> = Mutex::new(String::new());
static VIDEO_TEMP_PATH: Mutex<String> = Mutex::new(String::new());
static AUDIO_TEMP_PATH: Mutex<String> = Mutex::new(String::new());
static START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
static RECORDING_PAUSED: AtomicBool = AtomicBool::new(false);
static AUDIO_CAPTURE: Mutex<Option<AudioCapture>> = Mutex::new(None);
static ENABLE_AUDIO: AtomicBool = AtomicBool::new(false);

struct CaptureHandler {
    encoder: Option<VideoEncoder>,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = (i32, i32);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let video_path = VIDEO_TEMP_PATH.lock().unwrap().clone();
        let width = ctx.flags.0 as u32;
        let height = ctx.flags.1 as u32;

        tracing::info!("Video: {}x{} -> {}", width, height, video_path);

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            &video_path,
        )?;

        Ok(Self {
            encoder: Some(encoder),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if RECORDING_PAUSED.load(Ordering::Relaxed) {
            return Ok(());
        }

        let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);

        if SHOULD_STOP.load(Ordering::Relaxed) {
            if let Some(encoder) = self.encoder.take() {
                encoder.finish()?;
            }
            capture_control.stop();
            return Ok(());
        }

        self.encoder.as_mut().unwrap().send_frame(frame)?;

        if count % 60 == 0 {
            tracing::info!("Frame {}", count);
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let count = FRAME_COUNT.load(Ordering::Relaxed);
        tracing::info!("Capture ended. Frames: {}", count);
        Ok(())
    }
}

/// Start screen + audio recording
pub fn start_recording(config: RecordingConfig) -> Result<(), String> {
    if RECORDING_ACTIVE.load(Ordering::Relaxed) {
        return Err("Recording already in progress".to_string());
    }

    // Ensure output directory exists
    if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Temp paths
    let temp_dir = std::env::temp_dir().join("easyspecy");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let video_temp = temp_dir.join("video_temp.mp4").to_string_lossy().to_string();
    let audio_temp = temp_dir.join("audio_temp.wav").to_string_lossy().to_string();

    *OUTPUT_PATH.lock().unwrap() = config.output_path;
    *VIDEO_TEMP_PATH.lock().unwrap() = video_temp.clone();
    *AUDIO_TEMP_PATH.lock().unwrap() = audio_temp.clone();

    FRAME_COUNT.store(0, Ordering::Relaxed);
    SHOULD_STOP.store(false, Ordering::Relaxed);
    RECORDING_PAUSED.store(false, Ordering::Relaxed);
    ENABLE_AUDIO.store(config.enable_audio, Ordering::Relaxed);
    *START_TIME.lock().unwrap() = Some(Instant::now());

    // Start audio capture with configured sample rate
    if config.enable_audio {
        let mut audio =
            AudioCapture::new(audio_temp, Some(config.audio_sample_rate)).map_err(|e| e.to_string())?;
        audio.start().map_err(|e| e.to_string())?;
        *AUDIO_CAPTURE.lock().unwrap() = Some(audio);
    }

    // Start video capture
    let monitor = Monitor::primary().map_err(|e| e.to_string())?;
    let width = monitor.width().map_err(|e| e.to_string())? as i32;
    let height = monitor.height().map_err(|e| e.to_string())? as i32;

    // FPS via MinimumUpdateIntervalSettings
    let min_interval = if config.fps > 0 {
        MinimumUpdateIntervalSettings::Custom(Duration::from_millis(1000 / config.fps as u64))
    } else {
        MinimumUpdateIntervalSettings::Default
    };

    tracing::info!(
        "Capture: {}x{} @ {}fps, audio={}, sample_rate={}",
        width,
        height,
        config.fps,
        config.enable_audio,
        config.audio_sample_rate
    );

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        min_interval,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (width, height),
    );

    RECORDING_ACTIVE.store(true, Ordering::Relaxed);

    std::thread::spawn(move || {
        if let Err(e) = CaptureHandler::start(settings) {
            tracing::error!("Capture error: {}", e);
        }
        RECORDING_ACTIVE.store(false, Ordering::Relaxed);
    });

    Ok(())
}

/// Stop recording, merge audio+video if needed
pub fn stop_recording() -> Result<RecordingResult, String> {
    if !RECORDING_ACTIVE.load(Ordering::Relaxed) {
        return Err("No recording in progress".to_string());
    }

    SHOULD_STOP.store(true, Ordering::Relaxed);

    // Wait for video to finish
    for _ in 0..150 {
        if !RECORDING_ACTIVE.load(Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Stop audio
    let audio_path = if ENABLE_AUDIO.load(Ordering::Relaxed) {
        let mut lock = AUDIO_CAPTURE.lock().unwrap();
        if let Some(audio) = lock.as_mut() {
            audio.stop().ok()
        } else {
            None
        }
    } else {
        None
    };

    let video_path = VIDEO_TEMP_PATH.lock().unwrap().clone();
    let output_path = OUTPUT_PATH.lock().unwrap().clone();
    let frame_count = FRAME_COUNT.load(Ordering::Relaxed);
    let start = START_TIME.lock().unwrap().take();
    let duration = start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    let has_audio = audio_path.is_some();

    // Merge video + audio if we have audio
    if has_audio {
        let audio_file = audio_path.unwrap();
        if std::path::Path::new(&audio_file).exists() {
            merge_audio_video(&video_path, &audio_file, &output_path)?;
            let _ = std::fs::remove_file(&video_path);
            let _ = std::fs::remove_file(&audio_file);
        } else {
            // No audio recorded, just use video
            std::fs::rename(&video_path, &output_path)
                .or_else(|_| std::fs::copy(&video_path, &output_path).map(|_| ()))
                .map_err(|e| e.to_string())?;
        }
    } else {
        // No audio, just move video
        if video_path != output_path {
            std::fs::rename(&video_path, &output_path)
                .or_else(|_| std::fs::copy(&video_path, &output_path).map(|_| ()))
                .map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&video_path);
        }
    }

    // Cleanup temp dir
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("easyspecy"));

    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(RecordingResult {
        output_path,
        duration_secs: duration,
        frame_count,
        file_size_bytes: file_size,
        has_audio,
    })
}

fn merge_audio_video(video: &str, audio: &str, output: &str) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;

    tracing::info!("Merge: video + audio -> {}", output);

    let status = std::process::Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            video,
            "-i",
            audio,
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-shortest",
            output,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("FFmpeg error: {}", e))?;

    if !status.success() {
        return Err("FFmpeg merge failed".to_string());
    }
    Ok(())
}

fn find_ffmpeg() -> Option<String> {
    for p in &["ffmpeg", "C:\\Users\\shaur\\OneDrive\\Documents\\ffmpeg\\bin\\ffmpeg.exe"] {
        if std::process::Command::new(p)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            return Some(p.to_string());
        }
    }
    None
}

pub fn pause_recording() {
    RECORDING_PAUSED.store(true, Ordering::Relaxed);
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.pause();
    }
}

pub fn resume_recording() {
    RECORDING_PAUSED.store(false, Ordering::Relaxed);
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.resume();
    }
}

pub fn is_recording() -> bool {
    RECORDING_ACTIVE.load(Ordering::Relaxed)
}

pub fn is_paused() -> bool {
    RECORDING_PAUSED.load(Ordering::Relaxed)
}

pub fn frame_count() -> u32 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

pub fn get_displays() -> Vec<DisplayInfo> {
    match Monitor::enumerate() {
        Ok(monitors) => monitors
            .into_iter()
            .enumerate()
            .map(|(i, m)| DisplayInfo {
                id: i as u32,
                name: m.name().unwrap_or_else(|_| format!("Monitor {}", i + 1)),
                width: m.width().unwrap_or(1920),
                height: m.height().unwrap_or(1080),
                is_primary: i == 0,
            })
            .collect(),
        Err(_) => vec![DisplayInfo {
            id: 0,
            name: "Primary Display".to_string(),
            width: 1920,
            height: 1080,
            is_primary: true,
        }],
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}
