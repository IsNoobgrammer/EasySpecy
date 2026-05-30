//! Screen capture module — real implementation using windows-capture
//! Saves directly to MP4 via the built-in VideoEncoder

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;
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
}

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);
static OUTPUT_PATH: Mutex<String> = Mutex::new(String::new());
static START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
static RECORDING_PAUSED: AtomicBool = AtomicBool::new(false);

struct CaptureHandler {
    encoder: Option<VideoEncoder>,
    start: Instant,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = (i32, i32);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let output_path = OUTPUT_PATH.lock().unwrap().clone();
        let width = ctx.flags.0 as u32;
        let height = ctx.flags.1 as u32;

        tracing::info!("Encoder: {}x{} -> {}", width, height, output_path);

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            &output_path,
        )?;

        Ok(Self {
            encoder: Some(encoder),
            start: Instant::now(),
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
            if let Some(mut encoder) = self.encoder.take() {
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
        tracing::info!("Capture ended. Total frames: {}", count);
        RECORDING_ACTIVE.store(false, Ordering::Relaxed);
        Ok(())
    }
}

/// Start screen recording. Returns immediately, recording runs in background thread.
pub fn start_recording(output_path: String) -> Result<(), String> {
    if RECORDING_ACTIVE.load(Ordering::Relaxed) {
        return Err("Recording already in progress".to_string());
    }

    // Ensure output directory exists
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    *OUTPUT_PATH.lock().unwrap() = output_path;
    FRAME_COUNT.store(0, Ordering::Relaxed);
    SHOULD_STOP.store(false, Ordering::Relaxed);
    RECORDING_PAUSED.store(false, Ordering::Relaxed);
    *START_TIME.lock().unwrap() = Some(Instant::now());

    let monitor = Monitor::primary().map_err(|e| e.to_string())?;
    let width = monitor.width().map_err(|e| e.to_string())? as i32;
    let height = monitor.height().map_err(|e| e.to_string())? as i32;

    tracing::info!("Starting capture: {}x{}", width, height);

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (width, height),
    );

    RECORDING_ACTIVE.store(true, Ordering::Relaxed);

    // Capture::start blocks, so run in a thread
    std::thread::spawn(move || {
        if let Err(e) = CaptureHandler::start(settings) {
            tracing::error!("Capture error: {}", e);
        }
        RECORDING_ACTIVE.store(false, Ordering::Relaxed);
    });

    Ok(())
}

/// Stop the current recording. Returns the result with file info.
pub fn stop_recording() -> Result<RecordingResult, String> {
    if !RECORDING_ACTIVE.load(Ordering::Relaxed) {
        return Err("No recording in progress".to_string());
    }

    SHOULD_STOP.store(true, Ordering::Relaxed);

    // Wait for capture to finish (max 10s)
    for _ in 0..100 {
        if !RECORDING_ACTIVE.load(Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let output_path = OUTPUT_PATH.lock().unwrap().clone();
    let frame_count = FRAME_COUNT.load(Ordering::Relaxed);
    let start = START_TIME.lock().unwrap().take();
    let duration = start
        .map(|s| s.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(RecordingResult {
        output_path,
        duration_secs: duration,
        frame_count,
        file_size_bytes: file_size,
    })
}

/// Pause the current recording
pub fn pause_recording() {
    RECORDING_PAUSED.store(true, Ordering::Relaxed);
}

/// Resume the current recording
pub fn resume_recording() {
    RECORDING_PAUSED.store(false, Ordering::Relaxed);
}

/// Check if recording is active
pub fn is_recording() -> bool {
    RECORDING_ACTIVE.load(Ordering::Relaxed)
}

/// Check if recording is paused
pub fn is_paused() -> bool {
    RECORDING_PAUSED.load(Ordering::Relaxed)
}

/// Get current frame count
pub fn frame_count() -> u32 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

/// Enumerate available displays
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
