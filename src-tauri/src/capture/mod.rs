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
    VideoSettingsSubType,
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
    /// Actual output size (crop applied) — history used to show the config resolution
    pub width: i32,
    pub height: i32,
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
static AUDIO_TEMP_PATH: Mutex<String> = Mutex::new(String::new());
static START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
/// Capture thread is alive (frames can arrive).
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
/// A recording session exists: set by start, cleared only when stop/abort finishes.
/// Separate from RECORDING_ACTIVE so a capture-thread error doesn't make Stop say
/// "no recording" and throw away the segments already on disk.
static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Size of the captured frames (monitor pixels) — region crops are clamped to it.
static CAPTURE_SIZE: Mutex<(i32, i32)> = Mutex::new((0, 0));
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
/// True if the live encoder produced HEVC (else H.264). Decides whether the final pass can stream-copy.
static CAPTURED_HEVC: AtomicBool = AtomicBool::new(false);

/// Live Media Foundation encoder. H.264 unless the user picked an H.265 output, since
/// HEVC MFTs are missing on older GPUs / stock Windows 10 (0xC00D5212, issue #2).
/// Falls back to H.264 if HEVC can't be created.
fn new_live_encoder(width: u32, height: u32, path: &str, want_hevc: bool) -> Result<VideoEncoder, Box<dyn std::error::Error + Send + Sync>> {
    let config = crate::config::AppConfig::load();
    let make = |sub| VideoEncoder::new(
        VideoSettingsBuilder::new(width, height)
            .sub_type(sub)
            .bitrate(config.effective_bitrate_kbps().max(1000) * 1000)
            .frame_rate(config.fps.max(1)),
        AudioSettingsBuilder::default().disabled(true),
        ContainerSettingsBuilder::default(),
        path,
    );
    if want_hevc {
        match make(VideoSettingsSubType::HEVC) {
            Ok(e) => { CAPTURED_HEVC.store(true, Ordering::SeqCst); return Ok(e); }
            Err(e) => tracing::warn!("HEVC live encoder unavailable ({}), falling back to H.264", e),
        }
    }
    CAPTURED_HEVC.store(false, Ordering::SeqCst);
    Ok(make(VideoSettingsSubType::H264)?)
}

struct CaptureHandler {
    encoder: Option<VideoEncoder>,
    width: u32,
    height: u32,
    segment_idx: u32,
    /// Frames sent to the current segment's encoder (diagnostics for segment loss)
    seg_frames: u32,
}

impl CaptureHandler {
    /// Finalise the current encoder. Only segments that actually got frames are queued for
    /// concat — an empty one (stop while paused, double pause) would break the stitch.
    /// Never fatal: a segment that can't be finalised is logged and skipped so capture goes on.
    fn finish_segment(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(encoder) = self.encoder.take() {
            let seg = segment_path(self.segment_idx);
            if self.seg_frames == 0 {
                // MF refuses to finalise a sink that got no samples (0xC00D4A44) — just drop it
                drop(encoder);
                let _ = std::fs::remove_file(&seg);
            } else {
                match encoder.finish() {
                    Ok(()) => {
                        tracing::info!("Segment {} finalised: {} ({} frames)", self.segment_idx, seg, self.seg_frames);
                        SEGMENT_PATHS.lock().unwrap().push(seg);
                    }
                    Err(e) => tracing::error!("Segment {} could not be finalised ({} frames lost): {}", self.segment_idx, self.seg_frames, e),
                }
            }
        }
        self.seg_frames = 0;
        Ok(())
    }
}

impl Drop for CaptureHandler {
    // Normal stop already took the encoder. On an error path this finalises (and queues)
    // whatever the current segment holds, instead of losing it.
    fn drop(&mut self) {
        if self.encoder.is_some() {
            tracing::warn!("Capture handler dropped mid-segment — salvaging segment {}", self.segment_idx);
            let _ = self.finish_segment();
        }
    }
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = (i32, i32);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let width = ctx.flags.0 as u32;
        let height = ctx.flags.1 as u32;
        let video_path = segment_path(0);

        tracing::info!("Video encoder init: {}x{} -> {}", width, height, video_path);

        use crate::config::VideoEncoder as Enc;
        let want_hevc = matches!(crate::config::AppConfig::load().effective_video_encoder(), Enc::H265 | Enc::H265_NVENC);
        let encoder = new_live_encoder(width, height, &video_path, want_hevc)?;

        Ok(Self {
            encoder: Some(encoder),
            width,
            height,
            segment_idx: 0,
            seg_frames: 0,
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


            // Keyboard timestamps start at video frame 0
            crate::keyboard::reset_keyboard_start_time();

            // Signal frontend: we are LIVE
            CAPTURE_READY.store(true, Ordering::SeqCst);
            tracing::info!("══ CAPTURE ARMED ══ video + audio synced at t=0");
        }

        // ═══ STOP: finish current segment, signal done ═══
        if SHOULD_STOP.load(Ordering::SeqCst) {
            self.finish_segment()?;
            capture_control.stop();
            return Ok(());
        }

        // ═══ PAUSE: finish current segment, warm up the next one, hold until resume ═══
        if SHOULD_FINISH_SEGMENT.load(Ordering::SeqCst) {
            SHOULD_FINISH_SEGMENT.store(false, Ordering::SeqCst);
            if self.encoder.is_some() && self.seg_frames == 0 {
                // Paused again before any frame reached the warm encoder (fast pause→resume→pause):
                // keep it for the next resume instead of finalising an empty segment.
                RECORDING_PAUSED.store(true, Ordering::SeqCst);
                return Ok(());
            }
            self.finish_segment()?;
            // Create the next encoder NOW, not on resume. windows-capture's frame pool has a
            // single buffer and the encoder holds that surface uncopied; a cold encoder created
            // inside the callback could keep it pinned, starving WGC so the whole resumed
            // segment came out as 1 frame. Warm at pause = same conditions as segment 0.
            self.segment_idx += 1;
            let next = segment_path(self.segment_idx);
            self.encoder = Some(new_live_encoder(self.width, self.height, &next, CAPTURED_HEVC.load(Ordering::SeqCst))?);
            // Mark as paused AFTER the encoder is flushed — avoids race with stop_recording()
            RECORDING_PAUSED.store(true, Ordering::SeqCst);
            return Ok(());
        }

        // ═══ PAUSED: skip frames unless resume is requested ═══
        if RECORDING_PAUSED.load(Ordering::Relaxed) {
            if !SHOULD_RESUME_CAPTURE.swap(false, Ordering::SeqCst) {
                return Ok(());
            }
            tracing::info!("Resuming into segment {}", self.segment_idx);
            RECORDING_PAUSED.store(false, Ordering::SeqCst);
            // Fall through — send this frame into the (already warm) segment
        }

        let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        if self.encoder.is_none() {
            // Only reachable if the warm-up at pause failed; better a cold encoder than a panic
            self.encoder = Some(new_live_encoder(self.width, self.height, &segment_path(self.segment_idx), CAPTURED_HEVC.load(Ordering::SeqCst))?);
        }
        self.seg_frames += 1;
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
    // compare_exchange: two concurrent starts can't both pass the check
    if SESSION_ACTIVE.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("Recording already in progress".to_string());
    }
    let result = start_recording_inner(config);
    if result.is_err() {
        abort_recording();
    }
    result
}

fn start_recording_inner(config: RecordingConfig) -> Result<(), String> {
    // Warm ffmpeg while recording: the first launch of the 150 MB bundled binary pays file-cache
    // + Defender scan (~1 s); paying it now keeps stop → shareable file fast.
    std::thread::spawn(|| {
        if let Some(ffmpeg) = find_ffmpeg() {
            let mut cmd = std::process::Command::new(ffmpeg);
            cmd.arg("-version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let _ = cmd.status();
        }
    });

    // Ensure output directory exists
    if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Temp paths
    let temp_dir = std::env::temp_dir().join("easyspecy");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let audio_temp = temp_dir.join("audio_temp.wav").to_string_lossy().to_string();

    *OUTPUT_PATH.lock().unwrap() = config.output_path;
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
    *CAPTURE_SIZE.lock().unwrap() = (width, height);

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
                // Tell the UI so it stops & saves what we have. Audio is kept for stop_recording.
                if let Some(app) = crate::app_handle() {
                    let _ = app.emit("capture-error", e.to_string());
                }
            }
            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
            CAPTURE_READY.store(false, Ordering::SeqCst);
        })
        .map_err(|e| format!("Failed to spawn capture thread: {}", e))?;

    // ═══ Spawn mouse tracking thread for cursor trail + click effects ═══
    // Polls cursor position at ~120Hz for smooth overlay rendering.
    // Emits events to the live effects overlay window.
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


            while RECORDING_ACTIVE.load(Ordering::SeqCst) {
                // Get cursor position (screen coordinates = video coordinates)
                let mut point = windows::Win32::Foundation::POINT { x: 0, y: 0 };

                if unsafe { GetCursorPos(&mut point).is_ok() } {

                    // Emit and record only if position changed (prevents high-frequency lock contention)
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
                    if let Some(app) = crate::app_handle() {
                        let _ = app.emit_to(
                            "effects-overlay",
                            "cursor-click",
                            (point.x, point.y, "left"),
                        );
                    }
                }
                if right_down && !right_was_down {
                    if let Some(app) = crate::app_handle() {
                        let _ = app.emit_to(
                            "effects-overlay",
                            "cursor-click",
                            (point.x, point.y, "right"),
                        );
                    }
                }
                if middle_down && !middle_was_down {
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
    if !SESSION_ACTIVE.load(Ordering::SeqCst) {
        return Err("No recording in progress".to_string());
    }
    let result = stop_recording_inner();
    let _ = AUDIO_CAPTURE.lock().unwrap().take();
    SESSION_ACTIVE.store(false, Ordering::SeqCst);
    result
}

/// Tear down a session that never produced a usable recording (start failed / timed out).
pub fn abort_recording() {
    SHOULD_STOP.store(true, Ordering::SeqCst);
    if let Some(mut audio) = AUDIO_CAPTURE.lock().unwrap().take() {
        let _ = audio.stop();
    }
    let t = Instant::now();
    while RECORDING_ACTIVE.load(Ordering::SeqCst) && t.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("easyspecy"));
    SESSION_ACTIVE.store(false, Ordering::SeqCst);
    tracing::info!("Recording session aborted");
}

fn stop_recording_inner() -> Result<RecordingResult, String> {

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

    // Segments go straight into the final pass (concat demuxer) — no separate stitch run.
    let output_path = OUTPUT_PATH.lock().unwrap().clone();
    let segments = SEGMENT_PATHS.lock().unwrap().clone();
    if segments.is_empty() {
        return Err("No video was captured".to_string());
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

    let mode = crate::config::AppConfig::load().recording_mode;
    let region = match mode {
        crate::config::RecordingMode::FullScreen => None,
        _ => region::get_region().and_then(|r| region::sanitize(r, *CAPTURE_SIZE.lock().unwrap())),
    };
    // Crop (Region/Window mode) is folded into the single final encode.
    let crop = region.as_ref().map(|r| format!("crop={}:{}:{}:{}", r.width, r.height, r.x, r.y));

    // Step 3: Single final pass — mux audio, apply crop, and re-encode only if needed
    set_encoding_progress(35, "Encoding video...");
    // Audio enabled but nothing captured (e.g. system loopback while nothing played):
    // still write a silent track so the file has audio like the user asked for.
    let audio_file = audio_path.map(|a| {
        if std::fs::metadata(&a).map(|m| m.len() > 44).unwrap_or(false) {
            a
        } else {
            tracing::warn!("Audio file empty or missing, writing silent track");
            let _ = std::fs::remove_file(&a);
            SILENT_AUDIO.to_string()
        }
    });
    encode_final(&segments, duration * 1000.0, audio_file.as_deref(), &output_path, crop.as_deref(), crop.is_none())?;
    for seg in &segments {
        let _ = std::fs::remove_file(seg);
    }
    if let Some(a) = audio_file.as_deref().filter(|a| *a != SILENT_AUDIO) {
        let _ = std::fs::remove_file(a);
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

    tracing::info!("Recording ready: {}", output_path);
    // Read once: two `.lock()` temporaries in one struct literal deadlocked (guards live to
    // the end of the statement, std Mutex isn't re-entrant).
    let (cap_w, cap_h) = *CAPTURE_SIZE.lock().unwrap();

    Ok(RecordingResult {
        output_path,
        width: region.as_ref().map_or(cap_w, |r| r.width),
        height: region.as_ref().map_or(cap_h, |r| r.height),
        duration_secs: duration,
        frame_count,
        file_size_bytes: file_size,
        has_audio,
    })
}

/// Sentinel audio "path" for a generated silent track (lavfi source).
const SILENT_AUDIO: &str = "anullsrc=r=48000:cl=stereo";

fn encode_final(segments: &[String], duration_ms: f64, audio: Option<&str>, output: &str, crop: Option<&str>, allow_copy: bool) -> Result<(), String> {
    use crate::config::VideoEncoder as Enc;
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;
    let config = crate::config::AppConfig::load();
    let target = config.effective_video_encoder();

    let hevc = CAPTURED_HEVC.load(Ordering::SeqCst);
    let codec_matches = match target {
        Enc::H264 | Enc::H264_NVENC | Enc::MobileShareable => !hevc,
        Enc::H265 | Enc::H265_NVENC => hevc,
        _ => false,
    };
    let copy = allow_copy && codec_matches;

    tracing::info!("Final pass: {} segment(s) + audio={:?} -> {} (copy={}, crop={:?}, {:.0}ms)",
        segments.len(), audio, output, copy, crop, duration_ms);

    let mut args: Vec<String> = vec!["-y".into(), "-fflags".into(), "+genpts+igndts".into()];
    if segments.len() == 1 {
        args.extend(["-i".into(), segments[0].clone()]);
    } else {
        // Pause/resume segments: concat demuxer stitches them inside this same run
        let list = std::env::temp_dir().join("easyspecy").join("concat_list.txt");
        let body: String = segments.iter().map(|p| format!("file '{}'\n", p.replace('\\', "/"))).collect();
        std::fs::write(&list, body).map_err(|e| format!("Concat list write failed: {}", e))?;
        args.extend(["-f".into(), "concat".into(), "-safe".into(), "0".into(), "-i".into(), list.to_string_lossy().into_owned()]);
    }
    if let Some(a) = audio {
        if a == SILENT_AUDIO {
            args.extend(["-f".into(), "lavfi".into()]);
        }
        args.extend(["-i".into(), a.into(), "-map".into(), "0:v:0".into(), "-map".into(), "1:a:0".into()]);
    }

    if copy {
        args.extend(["-c:v".into(), "copy".into()]);
    } else {
        let mut vf = String::from("setpts=PTS-STARTPTS");
        if let Some(c) = crop {
            vf = format!("{},{}", c, vf);
        }
        let crf = config.ffmpeg_crf().to_string();
        args.extend(["-vf".into(), vf, "-c:v".into(), config.ffmpeg_encoder().into()]);
        // Encoder-specific args (preset, tune, svt-params, etc.)
        args.extend(config.ffmpeg_extra_args());
        if config.video_quality == crate::config::VideoQuality::Custom {
            args.extend(["-b:v".into(), format!("{}k", config.effective_bitrate_kbps())]);
        } else {
            match target {
                // NVENC uses -qp instead of -crf
                Enc::H264_NVENC | Enc::H265_NVENC | Enc::AV1_NVENC => args.extend(["-qp".into(), crf]),
                Enc::VP9 => args.extend(["-crf".into(), crf, "-b:v".into(), "0".into()]),
                _ => args.extend(["-crf".into(), crf]),
            }
        }
        // -r: without it CFR conversion guessed 25 fps from the VFR input instead of the configured rate
        args.extend(["-vsync".into(), "cfr".into(), "-r".into(), config.fps.max(1).to_string(),
            "-pix_fmt".into(), "yuv420p".into(), "-threads".into(), "0".into()]);
    }

    if audio.is_some() {
        args.extend([
            // apad + -shortest: output length = video length. Loopback audio is shorter
            // when nothing plays; without apad, -shortest chopped the end of the video.
            "-af".into(), "asetpts=PTS-STARTPTS,apad".into(),
            "-c:a".into(), "aac".into(),
            "-b:a".into(), "192k".into(),
            "-shortest".into(),
        ]);
    } else {
        args.push("-an".into());
    }

    args.extend(["-progress".into(), "pipe:1".into(), output.into()]);

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args);

    if let Err(e) = run_ffmpeg_with_progress(cmd, duration_ms, 35, 85, "Encoding video...") {
        // Never lose the recording: a single segment with no audio/crop is already a valid file.
        if audio.is_none() && crop.is_none() && segments.len() == 1 {
            tracing::warn!("Final pass failed, keeping raw capture: {}", e);
            return std::fs::rename(&segments[0], output)
                .or_else(|_| std::fs::copy(&segments[0], output).map(|_| ()))
                .map_err(|e| e.to_string());
        }
        return Err(e);
    }

    tracing::info!("Final pass complete: {}", if copy { "stream copy".to_string() } else { config.ffmpeg_encoder().to_string() });
    Ok(())
}

/// Cached: the lookup can spawn `ffmpeg -version`, and it's needed several times per stop.
fn find_ffmpeg() -> Option<String> {
    static FFMPEG: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    FFMPEG.get_or_init(locate_ffmpeg).clone()
}

fn locate_ffmpeg() -> Option<String> {
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

/// Returns the temp file path for video segment N.
fn segment_path(idx: u32) -> String {
    std::env::temp_dir()
        .join("easyspecy")
        .join(format!("video_seg_{:03}.mp4", idx))
        .to_string_lossy()
        .to_string()
}

pub fn pause_recording() {
    // Record the pause start instant for duration accounting
    *PAUSE_INSTANT.lock().unwrap() = Some(Instant::now());
    // Signal on_frame_arrived to finish the current encoder segment.
    // RECORDING_PAUSED is set by on_frame_arrived AFTER flush — not here —
    // to avoid a race where stop_recording() thinks we're done before the
    // encoder has actually written the last frames.
    SHOULD_FINISH_SEGMENT.store(true, Ordering::SeqCst);
    // A resume that no frame has processed yet must not fire after this pause
    SHOULD_RESUME_CAPTURE.store(false, Ordering::SeqCst);
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
