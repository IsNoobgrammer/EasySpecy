//! Screen capture module — real implementation using windows-capture
//! Saves directly to MP4 via the built-in VideoEncoder, then merges audio via FFmpeg
//! Supports region capture by cropping via FFmpeg post-processing
//!
//! SYNC ARCHITECTURE:
//! 1. start_recording() initializes everything (monitor, audio streams) on the calling thread
//! 2. Video capture thread is spawned — blocks until first frame arrives
//! 3. On first frame: arms audio, records wall-clock start time
//! 4. Frontend polls `is_capture_ready()` to know when to show timer
//! 5. Audio uses the same armed gate — zero samples before first video frame
//! This guarantees video and audio start at the EXACT same instant (< 1ms drift).

use crate::audio::AudioCapture;
use crate::region;
use tauri::Emitter;
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
    pub audio_source: String, // "Mic", "System", "Both"
    pub audio_sample_rate: u32,
    pub fps: u32,
    // Webcam
    pub webcam_enabled: bool,
    pub webcam_device: String,
    pub webcam_size: u32,
    pub webcam_x: i32,
    pub webcam_y: i32,
    pub webcam_shape: String,
    pub webcam_border_color: String,
    pub webcam_border_width: u32,
    pub webcam_opacity: f32,
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
/// Sync gate: audio records only when this is true. Set on first video frame.
static CAPTURE_ARMED: AtomicBool = AtomicBool::new(false);
/// Signal to frontend: capture is ready and actively recording
static CAPTURE_READY: AtomicBool = AtomicBool::new(false);
/// Encoding progress: 0-100 (polled by frontend during encoding phase)
static ENCODING_PROGRESS: AtomicU32 = AtomicU32::new(0);
/// Encoding stage description
static ENCODING_STAGE: Mutex<String> = Mutex::new(String::new());

// ═══ Segment-based pause/resume ═══
/// Finalized segment paths (one per active recording run between pauses)
static SEGMENT_PATHS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Signal on_frame_arrived to finish the current encoder segment (pause)
static SHOULD_FINISH_SEGMENT: AtomicBool = AtomicBool::new(false);
/// Signal on_frame_arrived to start a new encoder segment (resume)
static SHOULD_RESUME_CAPTURE: AtomicBool = AtomicBool::new(false);
/// Accumulated time spent in paused state — subtracted from wallclock duration
/// before passing to sync_verifier so checks remain valid with pauses.
static TOTAL_PAUSED_DURATION: Mutex<Duration> = Mutex::new(Duration::ZERO);
/// Wall-clock instant when the current pause began
static PAUSE_INSTANT: Mutex<Option<Instant>> = Mutex::new(None);

struct CaptureHandler {
    encoder: Option<VideoEncoder>,
    width: u32,
    height: u32,
    segment_idx: u32,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = (i32, i32);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let width = ctx.flags.0 as u32;
        let height = ctx.flags.1 as u32;
        let video_path = segment_path(0);

        tracing::info!("Video encoder init: {}x{} -> {}", width, height, video_path);

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            &video_path,
        )?;

        Ok(Self {
            encoder: Some(encoder),
            width,
            height,
            segment_idx: 0,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // ═══ SYNC POINT: First frame = arm everything simultaneously ═══
        if !CAPTURE_ARMED.load(Ordering::SeqCst) {
            CAPTURE_ARMED.store(true, Ordering::SeqCst);
            *START_TIME.lock().unwrap() = Some(Instant::now());

            // Arm audio capture to start recording NOW (synced with video)
            if ENABLE_AUDIO.load(Ordering::SeqCst) {
                let lock = AUDIO_CAPTURE.lock().unwrap();
                if let Some(audio) = lock.as_ref() {
                    audio.set_armed(true);
                }
            }

            // Arm webcam capture (synced with video + audio)
            crate::webcam::arm_webcam();

            // Signal frontend: we are LIVE
            CAPTURE_READY.store(true, Ordering::SeqCst);
            tracing::info!("══ CAPTURE ARMED ══ video + audio synced at t=0");
        }

        // ═══ STOP: finish current segment, signal done ═══
        if SHOULD_STOP.load(Ordering::SeqCst) {
            if let Some(encoder) = self.encoder.take() {
                encoder.finish()?;
                let seg = segment_path(self.segment_idx);
                tracing::info!("Final segment {} finalised: {}", self.segment_idx, seg);
                SEGMENT_PATHS.lock().unwrap().push(seg);
            }
            capture_control.stop();
            return Ok(());
        }

        // ═══ PAUSE: finish current segment, hold until resume ═══
        if SHOULD_FINISH_SEGMENT.load(Ordering::SeqCst) {
            if let Some(encoder) = self.encoder.take() {
                encoder.finish()?;
            }
            let seg = segment_path(self.segment_idx);
            tracing::info!("Segment {} finalised: {}", self.segment_idx, seg);
            SEGMENT_PATHS.lock().unwrap().push(seg);
            self.segment_idx += 1;
            SHOULD_FINISH_SEGMENT.store(false, Ordering::SeqCst);
            // Mark as paused AFTER the encoder is flushed — avoids race with stop_recording()
            RECORDING_PAUSED.store(true, Ordering::SeqCst);
            return Ok(());
        }

        // ═══ PAUSED: skip frames unless resume is requested ═══
        if RECORDING_PAUSED.load(Ordering::Relaxed) {
            if SHOULD_RESUME_CAPTURE.load(Ordering::SeqCst) {
                // Start a fresh encoder for the new segment
                let new_path = segment_path(self.segment_idx);
                tracing::info!("Starting segment {} at: {}", self.segment_idx, new_path);
                self.encoder = Some(VideoEncoder::new(
                    VideoSettingsBuilder::new(self.width, self.height),
                    AudioSettingsBuilder::default().disabled(true),
                    ContainerSettingsBuilder::default(),
                    &new_path,
                )?);
                SHOULD_RESUME_CAPTURE.store(false, Ordering::SeqCst);
                RECORDING_PAUSED.store(false, Ordering::SeqCst);
                // Fall through — send this frame into the new segment
            } else {
                return Ok(());
            }
        }

        let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        self.encoder.as_mut().unwrap().send_frame(frame)?;

        if count % 120 == 0 {
            tracing::debug!("Frame {}", count);
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let count = FRAME_COUNT.load(Ordering::Relaxed);
        tracing::info!("Capture session closed. Total frames: {}", count);
        Ok(())
    }
}

/// Start screen + audio recording.
/// Initializes capture pipeline and spawns threads. Returns immediately.
/// Frontend must poll `is_capture_ready()` before showing timer.
pub fn start_recording(config: RecordingConfig) -> Result<(), String> {
    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
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

    // Reset all state atomics
    FRAME_COUNT.store(0, Ordering::SeqCst);
    SHOULD_STOP.store(false, Ordering::SeqCst);
    RECORDING_PAUSED.store(false, Ordering::SeqCst);
    CAPTURE_ARMED.store(false, Ordering::SeqCst);
    CAPTURE_READY.store(false, Ordering::SeqCst);
    ENABLE_AUDIO.store(config.enable_audio, Ordering::SeqCst);
    // Reset segment state
    SEGMENT_PATHS.lock().unwrap().clear();
    SHOULD_FINISH_SEGMENT.store(false, Ordering::SeqCst);
    SHOULD_RESUME_CAPTURE.store(false, Ordering::SeqCst);
    *TOTAL_PAUSED_DURATION.lock().unwrap() = Duration::ZERO;
    *PAUSE_INSTANT.lock().unwrap() = None;

    // Initialize monitor and settings BEFORE spawning threads
    let monitor = Monitor::primary().map_err(|e| e.to_string())?;
    let width = monitor.width().map_err(|e| e.to_string())? as i32;
    let height = monitor.height().map_err(|e| e.to_string())? as i32;

    let min_interval = if config.fps > 0 {
        MinimumUpdateIntervalSettings::Custom(Duration::from_millis(1000 / config.fps as u64))
    } else {
        MinimumUpdateIntervalSettings::Default
    };

    tracing::info!(
        "Starting capture: {}x{} @ {}fps, audio={} ({}), sample_rate={}",
        width, height, config.fps, config.enable_audio, config.audio_source, config.audio_sample_rate
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

    // ═══ Create audio streams FIRST (ready but NOT armed) ═══
    // Streams are live and buffering, but samples are discarded until armed.
    // This eliminates audio device initialization latency from the sync equation.
    if config.enable_audio {
        let source = match config.audio_source.as_str() {
            "System" => crate::audio::AudioSource::System,
            "Both" => crate::audio::AudioSource::Both,
            _ => crate::audio::AudioSource::Mic,
        };
        let mut audio = AudioCapture::new(audio_temp, source).map_err(|e| e.to_string())?;
        audio.start().map_err(|e| e.to_string())?;
        *AUDIO_CAPTURE.lock().unwrap() = Some(audio);
    }

    // ═══ Start webcam capture (waits for CAPTURE_ARMED) ═══
    if config.webcam_enabled {
        // Tell frontend to release any browser webcam streams (device contention)
        if let Some(app) = crate::app_handle() {
            let _ = app.emit("release-webcam", ());
        }
        // Give browser time to release the device
        std::thread::sleep(Duration::from_millis(300));

        let webcam_config = crate::config::AppConfig {
            webcam_enabled: config.webcam_enabled,
            webcam_device: config.webcam_device.clone(),
            webcam_size: config.webcam_size,
            webcam_x: config.webcam_x,
            webcam_y: config.webcam_y,
            webcam_shape: match config.webcam_shape.as_str() {
                "Circle" => crate::config::WebcamShape::Circle,
                "Rounded" => crate::config::WebcamShape::Rounded,
                "Squircle" => crate::config::WebcamShape::Squircle,
                _ => crate::config::WebcamShape::Circle,
            },
            webcam_border_color: config.webcam_border_color.clone(),
            webcam_border_width: config.webcam_border_width,
            webcam_opacity: config.webcam_opacity,
            ..Default::default()
        };
        if let Err(e) = crate::webcam::start_webcam_capture(&webcam_config) {
            tracing::warn!("Webcam capture failed to start: {}", e);
            // Emit error to frontend so user knows webcam won't be in the video
            if let Some(app) = crate::app_handle() {
                let _ = app.emit("webcam-error", e.to_string());
            }
        }
    }

    RECORDING_ACTIVE.store(true, Ordering::SeqCst);

    // ═══ Spawn video capture thread ═══
    std::thread::Builder::new()
        .name("easyspecy-capture".to_string())
        .spawn(move || {
            if let Err(e) = CaptureHandler::start(settings) {
                tracing::error!("Capture thread error: {}", e);
            }
            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
            CAPTURE_READY.store(false, Ordering::SeqCst);
            let _ = AUDIO_CAPTURE.lock().unwrap().take();
        })
        .map_err(|e| format!("Failed to spawn capture thread: {}", e))?;

    // ═══ Spawn mouse tracking thread for cursor trail + click effects ═══
    // Polls cursor position at ~120Hz for smooth overlay rendering.
    // Emits events to the overlay window AND feeds postprocess for FFmpeg bake-in.
    std::thread::Builder::new()
        .name("easyspecy-mouse".to_string())
        .spawn(move || {
            use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON, VK_MBUTTON};
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            use tauri::Emitter;

            let mut left_was_down = false;
            let mut right_was_down = false;
            let mut middle_was_down = false;
            let mut start_point = windows::Win32::Foundation::POINT { x: 0, y: 0 };
            let (mut last_x, mut last_y) = unsafe {
                if GetCursorPos(&mut start_point).is_ok() {
                    (start_point.x, start_point.y)
                } else {
                    (0, 0)
                }
            };

            tracing::info!("Mouse tracking thread started");

            // Wait until capture is armed (first video frame)
            while !CAPTURE_ARMED.load(Ordering::SeqCst) {
                if !RECORDING_ACTIVE.load(Ordering::SeqCst) {
                    return; // Recording stopped before arming
                }
                std::thread::sleep(Duration::from_millis(5));
            }

            // ═══ SYNC: Reset cursor timestamp origin to NOW (= first video frame) ═══
            crate::postprocess::reset_session_start();

            while RECORDING_ACTIVE.load(Ordering::SeqCst) {
                // Get cursor position (screen coordinates = video coordinates)
                let mut point = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                let (mut vx, mut vy) = (last_x as f32, last_y as f32);

                if unsafe { GetCursorPos(&mut point).is_ok() } {
                    vx = point.x as f32;
                    vy = point.y as f32;

                    // Emit and record only if position changed (prevents high-frequency lock contention)
                    if point.x != last_x || point.y != last_y {
                        last_x = point.x;
                        last_y = point.y;

                        crate::postprocess::record_cursor(vx, vy);

                        if let Some(app) = crate::app_handle() {
                            let _ = app.emit_to(
                                "effects-overlay",
                                "cursor-move",
                                (point.x, point.y),
                            );
                        }
                    }
                }

                // Detect mouse button state changes (press = new click)
                let left_down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } & 0x8000u16 as i16 != 0;
                let right_down = unsafe { GetAsyncKeyState(VK_RBUTTON.0 as i32) } & 0x8000u16 as i16 != 0;
                let middle_down = unsafe { GetAsyncKeyState(VK_MBUTTON.0 as i32) } & 0x8000u16 as i16 != 0;

                if left_down && !left_was_down {
                    crate::postprocess::record_click(vx, vy, "left");
                    // ═══ AUTO-ZOOM: Capture window bounds on click asynchronously ═══
                    std::thread::spawn(|| {
                        capture_window_bounds_on_click();
                    });
                    if let Some(app) = crate::app_handle() {
                        let _ = app.emit_to(
                            "effects-overlay",
                            "cursor-click",
                            (point.x, point.y, "left"),
                        );
                    }
                }
                if right_down && !right_was_down {
                    crate::postprocess::record_click(vx, vy, "right");
                    std::thread::spawn(|| {
                        capture_window_bounds_on_click();
                    });
                    if let Some(app) = crate::app_handle() {
                        let _ = app.emit_to(
                            "effects-overlay",
                            "cursor-click",
                            (point.x, point.y, "right"),
                        );
                    }
                }
                if middle_down && !middle_was_down {
                    crate::postprocess::record_click(vx, vy, "middle");
                    std::thread::spawn(|| {
                        capture_window_bounds_on_click();
                    });
                    if let Some(app) = crate::app_handle() {
                        let _ = app.emit_to(
                            "effects-overlay",
                            "cursor-click",
                            (point.x, point.y, "middle"),
                        );
                    }
                }

                left_was_down = left_down;
                right_was_down = right_down;
                middle_was_down = middle_down;

                // ~60Hz polling for cursor tracking (balances GPU/CPU and updates smooth 60fps)
                std::thread::sleep(Duration::from_millis(16));
            }

            tracing::info!("Mouse tracking thread stopped");
        })
        .map_err(|e| format!("Failed to spawn mouse tracking thread: {}", e))?;

    Ok(())
}

/// Returns true once the first video frame has been captured and audio is armed.
/// Frontend should poll this before showing the recording timer.
pub fn is_capture_ready() -> bool {
    CAPTURE_READY.load(Ordering::SeqCst)
}

/// Get encoding progress (0-100) and current stage description
pub fn get_encoding_progress() -> (u32, String) {
    let progress = ENCODING_PROGRESS.load(Ordering::Relaxed);
    let stage = ENCODING_STAGE.lock().unwrap().clone();
    (progress, stage)
}

fn set_encoding_progress(progress: u32, stage: &str) {
    ENCODING_PROGRESS.store(progress, Ordering::Relaxed);
    *ENCODING_STAGE.lock().unwrap() = stage.to_string();
}

/// Run an FFmpeg command that has `-progress pipe:1`, parsing real-time progress
/// and mapping it to an encoding progress range [range_start..range_end].
/// Returns Ok(()) on success, Err on FFmpeg failure.
pub(crate) fn run_ffmpeg_with_progress(
    mut cmd: std::process::Command,
    total_duration_ms: f64,
    range_start: u32,
    range_end: u32,
    stage: &str,
) -> Result<(), String> {
    cmd.stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| format!("FFmpeg spawn error: {}", e))?;

    // Drain stderr on a background thread to prevent pipe buffer deadlock
    let stderr = child.stderr.take();
    let stderr_handle = stderr.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = String::new();
            use std::io::Read;
            let _ = s.read_to_string(&mut buf);
            buf
        })
    });

    // Parse stdout (-progress pipe:1) for real-time progress
    let stdout = child.stdout.take().ok_or("FFmpeg no stdout")?;
    let reader = std::io::BufReader::new(stdout);
    use std::io::BufRead;

    let range_size = range_end.saturating_sub(range_start) as f64;
    let mut last_pct = 0u32;

    for line in reader.lines() {
        let line = line.unwrap_or_default();
        let line = line.trim();

        if let Some(val) = line.strip_prefix("out_time_ms=") {
            if total_duration_ms > 0.0 {
                if let Ok(us) = val.parse::<f64>() {
                    let ms = us / 1000.0;
                    let frac = (ms / total_duration_ms).clamp(0.0, 1.0);
                    let pct = range_start + (frac * range_size) as u32;
                    if pct > last_pct && pct <= 100 {
                        last_pct = pct;
                        set_encoding_progress(pct, stage);
                    }
                }
            }
        } else if line.starts_with("progress=end") {
            set_encoding_progress(range_end, stage);
        }
    }

    let status = child.wait().map_err(|e| format!("FFmpeg wait error: {}", e))?;
    let stderr_out = stderr_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    if !status.success() {
        return Err(format!("FFmpeg failed: {}", stderr_out));
    }

    // Log encode speed if available
    if let Some(speed_line) = stderr_out.lines().rev().find(|l| l.starts_with("speed=")) {
        tracing::info!("FFmpeg encode speed: {}", speed_line);
    }

    Ok(())
}

/// Stop recording, merge audio+video if needed, crop if region is set.
/// This is a blocking call — should be run on a background thread from the command layer.
pub fn stop_recording() -> Result<RecordingResult, String> {
    if !RECORDING_ACTIVE.load(Ordering::SeqCst) {
        return Err("No recording in progress".to_string());
    }

    // Record the stop time IMMEDIATELY for precise duration
    let stop_instant = Instant::now();

    // Stop audio FIRST (instant) — ensures audio doesn't record extra samples
    let audio_path = if ENABLE_AUDIO.load(Ordering::SeqCst) {
        let mut lock = AUDIO_CAPTURE.lock().unwrap();
        if let Some(ref mut audio) = *lock {
            audio.stop().ok()
        } else {
            None
        }
    } else {
        None
    };

    // Signal video to stop on next frame
    SHOULD_STOP.store(true, Ordering::SeqCst);

    // Wait for video capture thread to finish flushing encoder
    let wait_start = Instant::now();
    while RECORDING_ACTIVE.load(Ordering::SeqCst) {
        if wait_start.elapsed() > Duration::from_secs(15) {
            tracing::error!("Timeout waiting for capture thread to stop");
            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // ─── Stitch segments into video_temp.mp4 ───────────────────────────────
    let video_path = VIDEO_TEMP_PATH.lock().unwrap().clone();
    let output_path = OUTPUT_PATH.lock().unwrap().clone();
    let segments = SEGMENT_PATHS.lock().unwrap().clone();
    set_encoding_progress(8, "Stitching segments...");
    // Verify each segment before stitching
    for seg in &segments {
        match crate::sync_verifier::verify_segment_duration(seg) {
            Ok(ms) => tracing::info!("Segment pre-check: {} — {:.1}ms", seg, ms),
            Err(e) => tracing::warn!("Segment pre-check warn: {}", e),
        }
    }
    if let Err(e) = concat_segments(&segments, &video_path) {
        tracing::error!("Segment concat failed: {} — falling back to first segment", e);
        if let Some(first) = segments.first() {
            let _ = std::fs::copy(first, &video_path);
        }
    }
    let frame_count = FRAME_COUNT.load(Ordering::Relaxed);
    let start = START_TIME.lock().unwrap().take();
    // Compute active recording duration: wall-clock elapsed MINUS total paused time.
    // This is what we pass to sync_verifier so it compares against actual content.
    let paused_total = *TOTAL_PAUSED_DURATION.lock().unwrap();
    let duration = start
        .map(|s| ((stop_instant - s).as_secs_f64() - paused_total.as_secs_f64()).max(0.0))
        .unwrap_or(0.0);
    let has_audio = audio_path.is_some();

    tracing::info!(
        "Recording stopped: {} frames, {:.2}s, audio={}",
        frame_count, duration, has_audio
    );

    set_encoding_progress(10, "Processing audio...");

    // Check if region capture is set
    let region = region::get_region();

    // Step 1: If region is set, crop video to region
    set_encoding_progress(20, "Cropping region...");
    let processed_video = if let Some(ref r) = region {
        let cropped_path = std::env::temp_dir()
            .join("easyspecy")
            .join("video_cropped.mp4")
            .to_string_lossy()
            .to_string();
        crop_video(&video_path, &cropped_path, r)?;
        let _ = std::fs::remove_file(&video_path);
        cropped_path
    } else {
        video_path
    };

    // Step 2: Webcam overlay compositing (before audio merge)
    set_encoding_progress(25, "Webcam overlay...");
    let (webcam_dir, webcam_elapsed) = crate::webcam::stop_webcam_capture();
    // Use webcam's own elapsed time for FPS calculation (more accurate than video duration)
    let webcam_duration = if webcam_elapsed > 0.1 { webcam_elapsed } else { duration };
    let processed_video = if let Some(ref wdir) = webcam_dir {
        let config_loaded = crate::config::AppConfig::load();
        let shape = match config_loaded.webcam_shape {
            crate::config::WebcamShape::Circle => crate::config::WebcamShape::Circle,
            crate::config::WebcamShape::Rounded => crate::config::WebcamShape::Rounded,
            crate::config::WebcamShape::Squircle => crate::config::WebcamShape::Squircle,
        };
        let mask_path = crate::webcam::generate_shape_mask(
            &shape,
            config_loaded.webcam_size,
            config_loaded.webcam_border_width,
            &config_loaded.webcam_border_color,
        );
        match mask_path {
            Ok(mask) => {
                match crate::webcam::composite_webcam_on_video(&processed_video, wdir, &mask.to_string_lossy(), &config_loaded, webcam_duration) {
                    Ok(webcam_video) => {
                        let _ = std::fs::remove_file(&processed_video);
                        crate::webcam::cleanup_webcam();
                        webcam_video
                    }
                    Err(e) => {
                        tracing::warn!("Webcam overlay failed: {}", e);
                        crate::webcam::cleanup_webcam();
                        processed_video
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Webcam mask generation failed: {}", e);
                crate::webcam::cleanup_webcam();
                processed_video
            }
        }
    } else {
        processed_video
    };

    // Step 3: Merge video + audio (real-time progress 35-85%)
    set_encoding_progress(35, "Encoding video...");
    if has_audio {
        let audio_file = audio_path.unwrap();
        if std::path::Path::new(&audio_file).exists()
            && std::fs::metadata(&audio_file).map(|m| m.len() > 44).unwrap_or(false)
        {
            merge_audio_video(&processed_video, &audio_file, &output_path)?;
            let _ = std::fs::remove_file(&processed_video);
            let _ = std::fs::remove_file(&audio_file);
        } else {
            tracing::warn!("Audio file empty or missing, saving video only");
            fix_video_timestamps(&processed_video, &output_path)?;
            let _ = std::fs::remove_file(&processed_video);
        }
    } else {
        if processed_video != output_path {
            fix_video_timestamps(&processed_video, &output_path)?;
            let _ = std::fs::remove_file(&processed_video);
        }
    }

    // Cleanup temp dir
    set_encoding_progress(90, "Verifying sync...");
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("easyspecy"));

    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // ═══ SYNC VERIFICATION ═══
    let config_loaded = crate::config::AppConfig::load();
    let report = crate::sync_verifier::verify_recording_sync(
        &output_path,
        Duration::from_secs_f64(duration),
        frame_count,
        has_audio,
        config_loaded.fps,
    );
    match report {
        Ok(r) if !r.passed => {
            let error_summary = r.errors.join("; ");
            tracing::error!("SYNC VERIFICATION FAILED — recording may be desynced: {}", error_summary);
        }
        Ok(_) => {
            tracing::info!("SYNC VERIFICATION PASSED ✓");
        }
        Err(e) => {
            tracing::warn!("Sync verification could not run: {}", e);
        }
    }

    set_encoding_progress(100, "Complete");

    // ═══ Save cursor metadata and apply effects ═══
    tracing::info!("══ stop_recording: calling finalize() ══");
    let final_output = if let Some(meta) = crate::postprocess::finalize() {
        tracing::info!(
            "══ finalize() returned Some: {} trail, {} clicks, trail='{}', click='{}' ══",
            meta.cursor_trail.len(), meta.click_events.len(),
            meta.trail_style, meta.click_effect
        );
        // Save metadata to disk — required for post-processing (effects, autozoom, etc.)
        if let Err(e) = crate::postprocess::save_metadata(&meta, &output_path) {
            tracing::warn!("Failed to save cursor metadata: {}", e);
        }

        // Apply trail + click effects to the video
        let effects_output = output_path.replace(".mp4", "_fx.mp4");
        tracing::info!("══ calling apply_effects({}) ══", output_path);
        let res_path = match crate::postprocess::apply_effects(&output_path, &effects_output, &meta) {
            Ok(ref effects_path) if effects_path != &output_path => {
                tracing::info!("══ apply_effects SUCCESS: {} → {} ══", effects_path, output_path);
                // Effects were applied — swap files
                let _ = std::fs::remove_file(&output_path);
                let _ = std::fs::rename(effects_path, &output_path);
                tracing::info!("Effects baked into final video");
                output_path.clone()
            }
            Ok(ref same_path) => {
                tracing::info!("══ apply_effects RETURNED SAME PATH (no effects): {} ══", same_path);
                output_path.clone()
            }
            Err(e) => {
                tracing::error!("══ apply_effects ERROR: {} ══", e);
                output_path.clone()
            }
        };

        // All post-processing done — delete .meta.json, user only needs the .mp4
        let meta_path = output_path.replace(".mp4", ".meta.json");
        if std::path::Path::new(&meta_path).exists() {
            if let Err(e) = std::fs::remove_file(&meta_path) {
                tracing::warn!("Failed to delete .meta.json after post-processing: {}", e);
            } else {
                tracing::info!("Cleaned up .meta.json after post-processing: {}", meta_path);
            }
        }

        res_path
    } else {
        tracing::info!("══ finalize() returned None — no metadata! ══");
        output_path.clone()
    };

    Ok(RecordingResult {
        output_path: final_output,
        duration_secs: duration,
        frame_count,
        file_size_bytes: file_size,
        has_audio,
    })
}

/// Crop video to region using FFmpeg crop filter
fn crop_video(input: &str, output: &str, region: &region::CaptureRegion) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;

    tracing::info!(
        "Crop: {}x{}+{},{} -> {}",
        region.width, region.height, region.x, region.y, output
    );

    let crop_filter = format!(
        "crop={}:{}:{}:{}",
        region.width, region.height, region.x, region.y
    );

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args([
            "-y",
            "-i",
            input,
            "-vf",
            &crop_filter,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "23",
            output,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let result = cmd.output().map_err(|e| format!("FFmpeg crop error: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("FFmpeg crop failed: {}", stderr));
    }
    Ok(())
}

/// Merge video + audio using FFmpeg.
/// Forces both streams to start at exactly t=0 with no gap.
/// Re-encodes video with setpts=PTS-STARTPTS to reset timestamps.
/// Uses configured encoder (H264/H265/VP9) and quality settings.
/// Reports real-time progress via -progress pipe:1.
fn merge_audio_video(video: &str, audio: &str, output: &str) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;
    let config = crate::config::AppConfig::load();

    tracing::info!("Merge: video={} + audio={} -> {}", video, audio, output);

    let video_duration_ms = probe_video_duration(&ffmpeg, video).unwrap_or(0.0);
    tracing::info!("Video actual duration: {:?}ms", video_duration_ms);

    let encoder = config.ffmpeg_encoder().to_string();
    let crf = config.ffmpeg_crf().to_string();
    let bitrate = format!("{}k", config.effective_bitrate_kbps());

    // Build FFmpeg args based on encoder
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-fflags".into(), "+genpts+igndts".into(),
        "-i".into(), video.into(),
        "-i".into(), audio.into(),
        "-vf".into(), "setpts=PTS-STARTPTS".into(),
        "-af".into(), "asetpts=PTS-STARTPTS".into(),
        "-c:v".into(), encoder.clone(),
    ];

    // Add encoder-specific args (preset, tune, svt-params, etc.)
    args.extend(config.ffmpeg_extra_args());

    // Quality: use CRF for quality-based encoding
    if config.video_quality == crate::config::VideoQuality::Custom {
        args.extend(["-b:v".into(), bitrate]);
    } else {
        // NVENC uses -qp instead of -crf
        match config.video_encoder {
            crate::config::VideoEncoder::H264_NVENC
            | crate::config::VideoEncoder::H265_NVENC
            | crate::config::VideoEncoder::AV1_NVENC => {
                args.extend(["-qp".into(), crf.clone()]);
            }
            crate::config::VideoEncoder::VP9 => {
                args.extend(["-crf".into(), crf.clone(), "-b:v".into(), "0".into()]);
            }
            _ => {
                args.extend(["-crf".into(), crf.clone()]);
            }
        }
    }

    args.extend([
        "-vsync".into(), "cfr".into(),
        "-c:a".into(), "aac".into(),
        "-b:a".into(), "192k".into(),
        "-threads".into(), "0".into(),
        "-shortest".into(),
        "-progress".into(), "pipe:1".into(),
        output.into(),
    ]);

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args_ref);

    run_ffmpeg_with_progress(cmd, video_duration_ms, 35, 85, "Encoding video...")?;

    tracing::info!(
        "Merge complete: encoder={}, quality={:?}",
        encoder,
        config.video_quality
    );
    Ok(())
}

/// Probe video duration in milliseconds using ffmpeg
fn probe_video_duration(ffmpeg: &str, video_path: &str) -> Option<f64> {
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(["-i", video_path])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    for line in stderr.lines() {
        if line.contains("Duration:") {
            // Parse "Duration: HH:MM:SS.mm"
            let start = line.find("Duration:")? + "Duration:".len();
            let rest = line[start..].trim();
            let end = rest.find(',')?;
            let time_str = rest[..end].trim();
            let parts: Vec<&str> = time_str.split(':').collect();
            if parts.len() == 3 {
                let h: f64 = parts[0].parse().ok()?;
                let m: f64 = parts[1].parse().ok()?;
                let s: f64 = parts[2].parse().ok()?;
                return Some((h * 3600.0 + m * 60.0 + s) * 1000.0);
            }
        }
    }
    None
}

/// Fix video timestamps — re-encode to ensure video starts at t=0 with no gap.
fn fix_video_timestamps(input: &str, output: &str) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;
    let config = crate::config::AppConfig::load();

    tracing::info!("Fixing video timestamps: {} -> {}", input, output);

    let encoder = config.ffmpeg_encoder().to_string();
    let crf = config.ffmpeg_crf().to_string();

    let preset = match config.video_encoder {
        crate::config::VideoEncoder::VP9 => "good",
        _ => "fast",
    };

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-fflags".into(), "+genpts+igndts".into(),
        "-i".into(), input.into(),
        "-vf".into(), "setpts=PTS-STARTPTS".into(),
        "-c:v".into(), encoder,
        "-preset".into(), preset.into(),
        "-crf".into(), crf,
        "-vsync".into(), "cfr".into(),
        "-an".into(),
        output.into(),
    ];

    if config.video_encoder == crate::config::VideoEncoder::VP9 {
        // VP9 needs -b:v 0 for CRF mode
        args.insert(args.len() - 1, "-b:v".into());
        args.insert(args.len() - 1, "0".into());
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args_ref)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let result = cmd.output().map_err(|e| format!("FFmpeg fix timestamps error: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        tracing::warn!("FFmpeg timestamp fix failed, copying raw: {}", stderr);
        std::fs::rename(input, output)
            .or_else(|_| std::fs::copy(input, output).map(|_| ()))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn find_ffmpeg() -> Option<String> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

    let candidates = [
        exe_dir.join("resources").join("ffmpeg.exe"),
        exe_dir.join("ffmpeg.exe"),
        exe_dir.parent().unwrap_or(&exe_dir).join("resources").join("ffmpeg.exe"),
        std::path::PathBuf::from("resources").join("ffmpeg.exe"),
        std::path::PathBuf::from("src-tauri").join("resources").join("ffmpeg.exe"),
    ];

    for path in &candidates {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    if cmd.status().is_ok() {
        return Some("ffmpeg".to_string());
    }

    None
}

/// Public wrapper for find_ffmpeg — used by commands module for GPU detection
pub fn find_ffmpeg_pub() -> Option<String> {
    find_ffmpeg()
}

/// Capture the foreground window bounds and record them for auto-zoom
fn capture_window_bounds_on_click() {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowRect};
    use windows::Win32::Foundation::RECT;

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return;
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let x = rect.left;
            let y = rect.top;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            // Get window title
            let mut title_buf: [u16; 256] = [0; 256];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);

            // Only record if window has reasonable dimensions
            if width > 50 && height > 50 {
                crate::postprocess::record_window_bounds(x, y, width, height, &title);
            }
        }
    }
}

/// Returns the temp file path for video segment N.
fn segment_path(idx: u32) -> String {
    std::env::temp_dir()
        .join("easyspecy")
        .join(format!("video_seg_{:03}.mp4", idx))
        .to_string_lossy()
        .to_string()
}

/// Concatenate video segments into a single MP4 using FFmpeg's concat demuxer.
/// Uses `-c copy` — no re-encode, only container header stitching (~instant).
/// If there is only one segment, renames it directly (zero FFmpeg overhead).
fn concat_segments(segments: &[String], output: &str) -> Result<(), String> {
    if segments.is_empty() {
        return Err("No segments to concatenate".to_string());
    }
    if segments.len() == 1 {
        // Fast path — single segment, just move it
        std::fs::rename(&segments[0], output)
            .or_else(|_| std::fs::copy(&segments[0], output).map(|_| ()))
            .map_err(|e| format!("Single segment rename failed: {}", e))?;
        tracing::info!("Single segment fast-path: {} -> {}", segments[0], output);
        return Ok(());
    }

    // Write the concat list file
    let list_path = std::env::temp_dir()
        .join("easyspecy")
        .join("concat_list.txt");
    let list_content: String = segments
        .iter()
        .map(|p| format!("file '{}'\n", p.replace('\\', "/")))
        .collect();
    std::fs::write(&list_path, &list_content)
        .map_err(|e| format!("Concat list write failed: {}", e))?;

    tracing::info!(
        "Concatenating {} segments -> {} via FFmpeg concat demuxer",
        segments.len(), output
    );

    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found for concat")?;
    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args([
        "-f", "concat",
        "-safe", "0",
        "-i", &list_path.to_string_lossy(),
        "-c", "copy",
        "-y",
        output,
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let result = cmd.output().map_err(|e| format!("FFmpeg concat exec failed: {}", e))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("FFmpeg concat error: {}", stderr));
    }

    // Clean up segment files and concat list
    let _ = std::fs::remove_file(&list_path);
    for seg in segments {
        let _ = std::fs::remove_file(seg);
    }

    tracing::info!("Concat complete: {}", output);
    Ok(())
}

pub fn pause_recording() {
    // Record the pause start instant for duration accounting
    *PAUSE_INSTANT.lock().unwrap() = Some(Instant::now());
    // Signal on_frame_arrived to finish the current encoder segment.
    // RECORDING_PAUSED is set by on_frame_arrived AFTER flush — not here —
    // to avoid a race where stop_recording() thinks we're done before the
    // encoder has actually written the last frames.
    SHOULD_FINISH_SEGMENT.store(true, Ordering::SeqCst);
    // Pause audio immediately (sample collection stops now)
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.pause();
    }
    tracing::info!("pause_recording: SHOULD_FINISH_SEGMENT set, audio paused");
}

pub fn resume_recording() {
    // Accumulate paused duration for sync_verifier wallclock correction
    if let Some(pause_start) = PAUSE_INSTANT.lock().unwrap().take() {
        let paused = pause_start.elapsed();
        *TOTAL_PAUSED_DURATION.lock().unwrap() += paused;
        tracing::info!("resume_recording: paused for {:.2}s, total paused: {:.2}s",
            paused.as_secs_f64(),
            TOTAL_PAUSED_DURATION.lock().unwrap().as_secs_f64()
        );
    }
    // Signal on_frame_arrived to start a fresh encoder segment.
    // on_frame_arrived clears RECORDING_PAUSED after the new encoder is ready.
    SHOULD_RESUME_CAPTURE.store(true, Ordering::SeqCst);
    // Resume audio immediately
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.resume();
    }
    tracing::info!("resume_recording: SHOULD_RESUME_CAPTURE set, audio resumed");
}

pub fn is_recording() -> bool {
    RECORDING_ACTIVE.load(Ordering::SeqCst)
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
