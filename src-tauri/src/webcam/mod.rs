//! Webcam capture + compositing for PiP overlay
//!
//! Architecture (sync-manager pattern + producer-consumer):
//! 1. start_webcam_capture() spawns thread that opens camera IMMEDIATELY
//! 2. Thread verifies camera works (test frame), signals WEBCAM_READY
//! 3. Thread waits for CAPTURE_ARMED (sync point with video + audio)
//! 4. When armed, captures frames as fast as camera delivers (native FPS)
//! 5. PNG encoding/saving offloaded to a writer thread (producer-consumer)
//! 6. On stop, computes actual FPS from frame_count / elapsed_time
//! 7. Composites webcam onto video using FFmpeg with the computed FPS
//!
//! Sync flow: camera opens → WEBCAM_READY → wait for CAPTURE_ARMED → capture frames
//! If camera fails to open, WEBCAM_ERROR is set and reported to user.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::{AppConfig, WebcamShape};

// ═══ GLOBALS ═══

static WEBCAM_ARMED: AtomicBool = AtomicBool::new(false);
static WEBCAM_STOP: AtomicBool = AtomicBool::new(false);
static WEBCAM_FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static WEBCAM_ACTIVE: AtomicBool = AtomicBool::new(false);
static WEBCAM_READY: AtomicBool = AtomicBool::new(false);
static WEBCAM_ERROR: Mutex<Option<String>> = Mutex::new(None);

static WEBCAM_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
static WEBCAM_DIR: Mutex<Option<String>> = Mutex::new(None);
/// Instant when webcam was armed (set in arm_webcam)
static WEBCAM_ARMED_TIME: Mutex<Option<Instant>> = Mutex::new(None);

// ═══ PUBLIC API ═══

/// Start webcam capture thread. Camera opens IMMEDIATELY and signals WEBCAM_READY.
/// If camera fails, returns Err with the reason (shown to user).
/// Called from start_recording() BEFORE video capture starts.
pub fn start_webcam_capture(config: &AppConfig) -> Result<(), String> {
    if !config.webcam_enabled {
        return Ok(());
    }

    let temp_dir = std::env::temp_dir().join("easyspecy").join("webcam");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let dir_str = temp_dir.to_string_lossy().to_string();

    *WEBCAM_DIR.lock().unwrap() = Some(dir_str.clone());

    // Clean old frames
    for entry in std::fs::read_dir(&temp_dir).unwrap_or_else(|_| std::fs::read_dir(".").unwrap()) {
        if let Ok(e) = entry {
            let _ = std::fs::remove_file(e.path());
        }
    }

    WEBCAM_ARMED.store(false, Ordering::SeqCst);
    WEBCAM_STOP.store(false, Ordering::SeqCst);
    WEBCAM_FRAME_COUNT.store(0, Ordering::SeqCst);
    WEBCAM_ACTIVE.store(true, Ordering::SeqCst);
    WEBCAM_READY.store(false, Ordering::SeqCst);
    *WEBCAM_ERROR.lock().unwrap() = None;
    *WEBCAM_ARMED_TIME.lock().unwrap() = None;

    let device_str = config.webcam_device.clone();
    let target_size = config.webcam_size;

    let handle = std::thread::Builder::new()
        .name("webcam-capture".into())
        .spawn(move || {
            if let Err(e) = webcam_capture_loop(&dir_str, &device_str, target_size) {
                tracing::error!("Webcam capture error: {}", e);
                *WEBCAM_ERROR.lock().unwrap() = Some(e.clone());
                WEBCAM_READY.store(false, Ordering::SeqCst);
            }
        })
        .map_err(|e| e.to_string())?;

    *WEBCAM_THREAD.lock().unwrap() = Some(handle);

    // Wait for camera to either become READY or fail (max 5 seconds)
    let start = Instant::now();
    while !WEBCAM_READY.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(5) {
        // Check if thread already errored out
        if let Some(err) = WEBCAM_ERROR.lock().unwrap().as_ref() {
            WEBCAM_ACTIVE.store(false, Ordering::SeqCst);
            return Err(format!("Webcam failed: {}", err));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if WEBCAM_READY.load(Ordering::SeqCst) {
        tracing::info!("Webcam READY — waiting for CAPTURE_ARMED");
        Ok(())
    } else {
        WEBCAM_ACTIVE.store(false, Ordering::SeqCst);
        Err("Webcam failed to initialize within 5 seconds".into())
    }
}

/// Arm webcam capture — called when first video frame arrives.
/// Must be called from the same sync point as audio arming.
pub fn arm_webcam() {
    if !WEBCAM_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    *WEBCAM_ARMED_TIME.lock().unwrap() = Some(Instant::now());
    WEBCAM_ARMED.store(true, Ordering::SeqCst);
    tracing::info!("Webcam ARMED — synced with video frame 0");
}

/// Get webcam error message if any. Called from frontend to show error to user.
pub fn get_webcam_error() -> Option<String> {
    WEBCAM_ERROR.lock().unwrap().clone()
}

/// Stop webcam capture and wait for thread to finish.
/// Returns (Option<webcam_dir>, elapsed_seconds_since_armed).
/// The elapsed time is used to compute actual webcam FPS for compositing.
pub fn stop_webcam_capture() -> (Option<String>, f64) {
    if !WEBCAM_ACTIVE.load(Ordering::SeqCst) {
        return (None, 0.0);
    }

    // Compute elapsed time BEFORE signaling stop (so we don't include post-stop work)
    let elapsed = WEBCAM_ARMED_TIME
        .lock()
        .unwrap()
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    WEBCAM_STOP.store(true, Ordering::SeqCst);

    // Wait for thread to finish
    let handle = WEBCAM_THREAD.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = h.join();
    }

    let frame_count = WEBCAM_FRAME_COUNT.load(Ordering::Relaxed);
    let actual_fps = if elapsed > 0.1 { frame_count as f64 / elapsed } else { 30.0 };
    tracing::info!(
        "Webcam capture stopped: {} frames in {:.2}s = {:.1} fps",
        frame_count, elapsed, actual_fps
    );

    WEBCAM_ACTIVE.store(false, Ordering::SeqCst);

    if frame_count == 0 {
        return (None, elapsed);
    }

    (WEBCAM_DIR.lock().unwrap().clone(), elapsed)
}

/// Generate shape mask PNG for FFmpeg compositing.
/// Returns path to the mask PNG (white shape on transparent background).
pub fn generate_shape_mask(
    shape: &WebcamShape,
    size: u32,
    border_width: u32,
    border_color: &str,
) -> Result<PathBuf, String> {
    let mask_dir = std::env::temp_dir().join("easyspecy");
    std::fs::create_dir_all(&mask_dir).map_err(|e| e.to_string())?;
    let mask_path = mask_dir.join("webcam_mask.png");

    let s = size as i32;
    let bw = border_width as i32;

    let mut img = image::RgbaImage::new(size, size);
    let bc = parse_hex_color(border_color);

    for y in 0..s {
        for x in 0..s {
            let cx = s as f32 / 2.0;
            let cy = s as f32 / 2.0;
            let px = x as f32;
            let py = y as f32;

            let inside = match shape {
                WebcamShape::Circle => {
                    let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    dist <= cx
                }
                WebcamShape::Rounded => {
                    let r = 16.0_f32.min(cx);
                    rounded_rect_contains(px, py, 0.0, 0.0, s as f32, s as f32, r)
                }
                WebcamShape::Squircle => squircle_contains(px, py, cx, cy, cx, 4.0),
            };

            if inside {
                let border_zone = if bw > 0 {
                    let inner = match shape {
                        WebcamShape::Circle => {
                            let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                            dist > (cx - bw as f32)
                        }
                        WebcamShape::Rounded => {
                            let r = 16.0_f32.min(cx);
                            !rounded_rect_contains(
                                px, py, bw as f32, bw as f32, (s - bw * 2) as f32,
                                (s - bw * 2) as f32, (r - bw as f32).max(0.0),
                            )
                        }
                        WebcamShape::Squircle => {
                            !squircle_contains(px, py, cx, cy, cx - bw as f32, 4.0)
                        }
                    };
                    inner
                } else {
                    false
                };

                if border_zone {
                    img.put_pixel(x as u32, y as u32, image::Rgba([bc[0], bc[1], bc[2], 255]));
                } else {
                    img.put_pixel(x as u32, y as u32, image::Rgba([255, 255, 255, 255]));
                }
            } else {
                img.put_pixel(x as u32, y as u32, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    img.save(&mask_path).map_err(|e| e.to_string())?;
    tracing::info!("Shape mask generated: {:?} ({}px)", shape, size);
    Ok(mask_path)
}

/// Composite webcam onto video using FFmpeg.
/// `duration_secs` is the wall-clock recording duration (from armed to stop),
/// used to compute the actual webcam FPS so playback matches real-time.
pub fn composite_webcam_on_video(
    video_path: &str,
    webcam_dir: &str,
    mask_path: &str,
    config: &AppConfig,
    duration_secs: f64,
) -> Result<String, String> {
    let output_path = std::env::temp_dir()
        .join("easyspecy")
        .join("video_webcam.mp4")
        .to_string_lossy()
        .to_string();

    let x = config.webcam_x.max(0).to_string();
    let y = config.webcam_y.max(0).to_string();
    let opacity = config.webcam_opacity.clamp(0.0, 1.0);
    let size = config.webcam_size;

    let webcam_pattern = format!("{}/webcam_%06d.png", webcam_dir);

    // ═══ Compute actual webcam FPS from frame count / elapsed time ═══
    let frame_count = WEBCAM_FRAME_COUNT.load(Ordering::Relaxed);
    let webcam_fps = if duration_secs > 0.1 {
        frame_count as f64 / duration_secs
    } else {
        30.0
    };
    tracing::info!(
        "Webcam composite: {} frames / {:.2}s = {:.2} fps (video fps)",
        frame_count, duration_secs, webcam_fps
    );

    let filter = if opacity < 1.0 {
        format!(
            "[1:v]scale={s}:{s},format=rgba[cam];\
             [2:v]scale={s}:{s},format=rgba[mask];\
             [cam][mask]alphamerge[masked];\
             [masked]colorchannelmixer=aa={op}[faded];\
             [0:v][faded]overlay={x}:{y}[out]",
            s = size, op = opacity, x = x, y = y
        )
    } else {
        format!(
            "[1:v]scale={s}:{s},format=rgba[cam];\
             [2:v]scale={s}:{s},format=rgba[mask];\
             [cam][mask]alphamerge[masked];\
             [0:v][masked]overlay={x}:{y}[out]",
            s = size, x = x, y = y
        )
    };

    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    let fps_str = format!("{:.2}", webcam_fps);

    let args: Vec<String> = vec![
        "-y".into(), "-i".into(), video_path.into(),
        "-framerate".into(), fps_str,
        "-i".into(), webcam_pattern,
        "-i".into(), mask_path.into(),
        "-filter_complex".into(), filter,
        "-map".into(), "[out]".into(),
        "-map".into(), "0:a?".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "ultrafast".into(),
        "-crf".into(), "18".into(),
        "-threads".into(), "0".into(),
        "-c:a".into(), "copy".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        "-progress".into(), "pipe:1".into(),
        output_path.clone(),
    ];

    tracing::info!(
        "Compositing webcam overlay: {} frames at ({}, {}), fps={:.2}",
        frame_count, x, y, webcam_fps
    );

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args);

    let webcam_duration_ms = duration_secs * 1000.0;
    if let Err(e) = crate::capture::run_ffmpeg_with_progress(
        cmd, webcam_duration_ms, 25, 35, "Webcam overlay..."
    ) {
        tracing::warn!("Masked overlay failed, trying simple overlay: {}", e);
        return composite_simple_overlay(video_path, webcam_dir, config, duration_secs);
    }

    tracing::info!("Webcam overlay composited successfully");
    Ok(output_path)
}

/// Fallback: simple overlay without shape mask
fn composite_simple_overlay(
    video_path: &str,
    webcam_dir: &str,
    config: &AppConfig,
    duration_secs: f64,
) -> Result<String, String> {
    let output_path = std::env::temp_dir()
        .join("easyspecy")
        .join("video_webcam.mp4")
        .to_string_lossy()
        .to_string();

    let x = config.webcam_x.max(0).to_string();
    let y = config.webcam_y.max(0).to_string();
    let size = config.webcam_size;

    let webcam_pattern = format!("{}/webcam_%06d.png", webcam_dir);
    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    let filter = format!(
        "[1:v]scale={s}:{s},format=rgba[cam];\
         [0:v][cam]overlay={x}:{y}[out]",
        s = size, x = x, y = y
    );

    // Compute actual FPS
    let frame_count = WEBCAM_FRAME_COUNT.load(Ordering::Relaxed);
    let webcam_fps = if duration_secs > 0.1 {
        frame_count as f64 / duration_secs
    } else {
        30.0
    };
    let fps_str = format!("{:.2}", webcam_fps);

    let args: Vec<String> = vec![
        "-y".into(), "-i".into(), video_path.into(),
        "-framerate".into(), fps_str,
        "-i".into(), webcam_pattern,
        "-filter_complex".into(), filter,
        "-map".into(), "[out]".into(),
        "-map".into(), "0:a?".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "ultrafast".into(),
        "-crf".into(), "18".into(),
        "-threads".into(), "0".into(),
        "-c:a".into(), "copy".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        "-progress".into(), "pipe:1".into(),
        output_path.clone(),
    ];

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args(&args);

    let webcam_duration_ms = duration_secs * 1000.0;
    crate::capture::run_ffmpeg_with_progress(
        cmd, webcam_duration_ms, 25, 35, "Webcam overlay..."
    )?;

    Ok(output_path)
}

/// Clean up webcam temp files
pub fn cleanup_webcam() {
    if let Some(dir) = WEBCAM_DIR.lock().unwrap().take() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let mask = std::env::temp_dir().join("easyspecy").join("webcam_mask.png");
    let _ = std::fs::remove_file(mask);
}

// ═══ INTERNAL: CAPTURE LOOP ═══
// Opens camera IMMEDIATELY, signals READY, then waits for ARMED.
// Uses producer-consumer: capture thread grabs frames as fast as possible,
// writer thread handles PNG encoding + disk I/O in the background.

fn webcam_capture_loop(
    output_dir: &str,
    device_str: &str,
    target_size: u32,
) -> Result<(), String> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
    use nokhwa::Camera;

    // ═══ PHASE 1: Open camera immediately (before ARMED) ═══
    let index = if device_str == "default" || device_str.is_empty() {
        CameraIndex::Index(0)
    } else {
        device_str
            .parse::<u32>()
            .map(CameraIndex::Index)
            .unwrap_or(CameraIndex::Index(0))
    };

    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

    tracing::info!("Webcam: opening camera device...");
    let mut camera = Camera::new(index, format)
        .map_err(|e| format!("Failed to open webcam: {}", e))?;

    camera.open_stream()
        .map_err(|e| format!("Failed to start webcam stream: {}", e))?;

    // Camera warmup — auto-exposure needs time
    std::thread::sleep(Duration::from_millis(500));

    // Get camera resolution
    let resolution = camera.resolution();
    let cam_w = resolution.width();
    let cam_h = resolution.height();
    tracing::info!("Webcam resolution: {}x{}", cam_w, cam_h);

    if cam_w == 0 || cam_h == 0 {
        camera.stop_stream().ok();
        return Err("Webcam returned zero resolution".into());
    }

    // Verify camera works with a test frame (must DECODE, not just read raw bytes)
    match camera.frame() {
        Ok(frame) => match frame.decode_image::<RgbFormat>() {
            Ok(img) => {
                tracing::info!("Webcam test frame OK: decoded {}x{}", img.width(), img.height());
            }
            Err(e) => {
                camera.stop_stream().ok();
                return Err(format!("Webcam test frame decode failed: {}", e));
            }
        },
        Err(e) => {
            camera.stop_stream().ok();
            return Err(format!("Webcam test frame failed: {}", e));
        }
    }

    // ═══ PHASE 2: Signal READY — camera is open and working ═══
    WEBCAM_READY.store(true, Ordering::SeqCst);
    tracing::info!("Webcam READY — camera open, waiting for CAPTURE_ARMED...");

    // ═══ PHASE 3: Wait for CAPTURE_ARMED (sync with video + audio) ═══
    while !WEBCAM_ARMED.load(Ordering::SeqCst) {
        if WEBCAM_STOP.load(Ordering::SeqCst) {
            camera.stop_stream().ok();
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    tracing::info!("Webcam CAPTURE_ARMED — recording frames");

    // ═══ PHASE 4: Producer-consumer capture ═══
    // Capture thread grabs frames as fast as the camera delivers.
    // Writer thread handles PNG encoding + disk I/O asynchronously.

    let (tx, rx) = std::sync::mpsc::sync_channel::<(image::RgbImage, u32)>(6);
    let writer_dir = output_dir.to_string();

    // Spawn writer thread
    let writer_handle = std::thread::Builder::new()
        .name("webcam-writer".into())
        .spawn(move || {
            while let Ok((img, idx)) = rx.recv() {
                let frame_path = format!("{}/webcam_{:06}.png", writer_dir, idx);
                let _ = img.save(&frame_path);
            }
        })
        .map_err(|e| format!("Failed to spawn webcam writer: {}", e))?;

    // Capture loop — grab frames as fast as camera delivers
    let mut frame_idx: u32 = 0;

    loop {
        if WEBCAM_STOP.load(Ordering::SeqCst) {
            break;
        }

        match camera.frame() {
            Ok(frame) => {
                // Decode the frame to RGB
                let decoded = match frame.decode_image::<RgbFormat>() {
                    Ok(img) => img,
                    Err(e) => {
                        if frame_idx == 0 {
                            tracing::warn!("Webcam frame decode failed: {}", e);
                        }
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                };

                // Resize (square) to the configured overlay size
                let resized = image::imageops::resize(
                    &decoded, target_size, target_size, image::imageops::FilterType::Triangle,
                );

                // Send to writer thread (non-blocking if buffer has space)
                match tx.try_send((resized, frame_idx)) {
                    Ok(()) => {
                        frame_idx += 1;
                        WEBCAM_FRAME_COUNT.store(frame_idx, Ordering::Relaxed);
                    }
                    Err(std::sync::mpsc::TrySendError::Full(_)) => {
                        // Writer is busy — drop this frame to keep capture running
                        // This prevents capture from stalling on disk I/O
                    }
                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                        break;
                    }
                }
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        if frame_idx > 0 && frame_idx % 30 == 0 {
            tracing::info!("Webcam: {} frames captured", frame_idx);
        }
    }

    // Signal writer thread to finish
    drop(tx);
    let _ = writer_handle.join();

    camera.stop_stream().ok();
    tracing::info!("Webcam capture loop ended ({} frames)", frame_idx);
    Ok(())
}

// ═══ HELPERS ═══

fn parse_hex_color(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        [r, g, b]
    } else {
        [0, 232, 138]
    }
}

fn rounded_rect_contains(px: f32, py: f32, rx: f32, ry: f32, w: f32, h: f32, r: f32) -> bool {
    if px < rx || px > rx + w || py < ry || py > ry + h {
        return false;
    }
    let corners = [
        (rx + r, ry + r), (rx + w - r, ry + r),
        (rx + r, ry + h - r), (rx + w - r, ry + h - r),
    ];
    for (cx, cy) in &corners {
        if (px < rx + r || px > rx + w - r) && (py < ry + r || py > ry + h - r) {
            let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if dist > r {
                return false;
            }
        }
    }
    true
}

fn squircle_contains(px: f32, py: f32, cx: f32, cy: f32, radius: f32, n: f32) -> bool {
    let dx = ((px - cx) / radius).abs();
    let dy = ((py - cy) / radius).abs();
    dx.powf(n) + dy.powf(n) <= 1.0
}
