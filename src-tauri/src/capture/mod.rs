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

        tracing::info!("Video encoder init: {}x{} -> {}", width, height, video_path);

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

            // Signal frontend: we are LIVE
            CAPTURE_READY.store(true, Ordering::SeqCst);
            tracing::info!("══ CAPTURE ARMED ══ video + audio synced at t=0");
        }

        if RECORDING_PAUSED.load(Ordering::Relaxed) {
            return Ok(());
        }

        if SHOULD_STOP.load(Ordering::SeqCst) {
            if let Some(encoder) = self.encoder.take() {
                encoder.finish()?;
            }
            capture_control.stop();
            return Ok(());
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
            let mut last_x: i32 = 0;
            let mut last_y: i32 = 0;

            tracing::info!("Mouse tracking thread started");

            // Wait until capture is armed (first video frame)
            while !CAPTURE_ARMED.load(Ordering::SeqCst) {
                if !RECORDING_ACTIVE.load(Ordering::SeqCst) {
                    return; // Recording stopped before arming
                }
                std::thread::sleep(Duration::from_millis(5));
            }

            // ═══ SYNC: Reset cursor timestamp origin to NOW (= first video frame) ═══
            // This ensures cursor timestamps are perfectly aligned with video frames.
            // Without this, there's a 200-500ms offset between start_collection() and
            // first video frame, causing trail to appear "behind" cursor.
            crate::postprocess::reset_session_start();

            // Pre-compute coordinate info
            // Video is captured at MONITOR resolution (not config resolution)
            // GetCursorPos returns screen coords = same coordinate space as video
            // NO SCALING NEEDED

            while RECORDING_ACTIVE.load(Ordering::SeqCst) {
                // Get cursor position (screen coordinates = video coordinates)
                let mut point = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                let (mut vx, mut vy) = (last_x as f32, last_y as f32);

                if unsafe { GetCursorPos(&mut point).is_ok() } {
                    // Direct use — no scaling. Video is at monitor resolution.
                    vx = point.x as f32;
                    vy = point.y as f32;

                    crate::postprocess::record_cursor(vx, vy);

                    // Emit to overlay window only if position changed
                    if point.x != last_x || point.y != last_y {
                        last_x = point.x;
                        last_y = point.y;
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

                // ~120Hz polling for smoother cursor tracking
                // Higher rate = more interpolation points = smoother trails
                std::thread::sleep(Duration::from_millis(8));
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

    let video_path = VIDEO_TEMP_PATH.lock().unwrap().clone();
    let output_path = OUTPUT_PATH.lock().unwrap().clone();
    let frame_count = FRAME_COUNT.load(Ordering::Relaxed);
    let start = START_TIME.lock().unwrap().take();
    // Use the precise duration from arm-time to stop-time
    let duration = start.map(|s| (stop_instant - s).as_secs_f64()).unwrap_or(0.0);
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

    // Step 2: Merge video + audio
    set_encoding_progress(30, "Encoding video...");
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
    let final_output = if let Some(meta) = crate::postprocess::finalize() {
        // Save metadata
        if let Err(e) = crate::postprocess::save_metadata(&meta, &output_path) {
            tracing::warn!("Failed to save cursor metadata: {}", e);
        }

        // Apply trail + click effects to the video
        let effects_output = output_path.replace(".mp4", "_fx.mp4");
        match crate::postprocess::apply_effects(&output_path, &effects_output, &meta) {
            Ok(effects_path) if effects_path != output_path => {
                // Effects were applied — swap files
                let _ = std::fs::remove_file(&output_path);
                let _ = std::fs::rename(&effects_path, &output_path);
                tracing::info!("Effects baked into final video");
                output_path.clone()
            }
            _ => output_path.clone(),
        }
    } else {
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
fn merge_audio_video(video: &str, audio: &str, output: &str) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;
    let config = crate::config::AppConfig::load();

    tracing::info!("Merge: video={} + audio={} -> {}", video, audio, output);

    let video_duration = probe_video_duration(&ffmpeg, video);
    tracing::info!("Video actual duration: {:?}ms", video_duration);

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
        "-shortest".into(),
        "-progress".into(), "pipe:1".into(),
        output.into(),
    ]);

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args_ref)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let child = cmd.spawn().map_err(|e| format!("FFmpeg spawn error: {}", e))?;
    let output_result = child.wait_with_output().map_err(|e| format!("FFmpeg wait error: {}", e))?;

    // Parse progress from stdout for logging
    let stdout = String::from_utf8_lossy(&output_result.stdout);
    if let Some(speed_line) = stdout.lines().rev().find(|l| l.starts_with("speed=")) {
        tracing::info!("FFmpeg encode speed: {}", speed_line);
    }

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(format!("FFmpeg merge failed: {}", stderr));
    }

    tracing::info!("Merge complete: encoder={}, quality={:?}", encoder, config.video_quality);
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
        std::path::PathBuf::from(r"C:\Users\shaur\OneDrive\Documents\ffmpeg\bin\ffmpeg.exe"),
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

pub fn pause_recording() {
    RECORDING_PAUSED.store(true, Ordering::SeqCst);
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.pause();
    }
}

pub fn resume_recording() {
    RECORDING_PAUSED.store(false, Ordering::SeqCst);
    let lock = AUDIO_CAPTURE.lock().unwrap();
    if let Some(audio) = lock.as_ref() {
        audio.resume();
    }
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
