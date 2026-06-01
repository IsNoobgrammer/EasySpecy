//! Webcam capture + compositing for PiP overlay
//!
//! Architecture:
//! 1. Webcam thread starts but waits for CAPTURE_ARMED
//! 2. When armed, captures frames at ~30fps as RGBA PNGs in temp dir
//! 3. On stop, composites webcam onto video using FFmpeg overlay with shape mask
//!
//! Sync: Webcam arms at the same instant as audio — both triggered by first video frame.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex};
use std::time::{Duration, Instant};

use crate::config::{AppConfig, WebcamShape};

// ═══ GLOBALS ═══

static WEBCAM_ARMED: AtomicBool = AtomicBool::new(false);
static WEBCAM_STOP: AtomicBool = AtomicBool::new(false);
static WEBCAM_FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static WEBCAM_ACTIVE: AtomicBool = AtomicBool::new(false);

static WEBCAM_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
static WEBCAM_DIR: Mutex<Option<String>> = Mutex::new(None);

// ═══ PUBLIC API ═══

/// Start webcam capture thread. Thread waits for CAPTURE_ARMED before collecting frames.
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

    let device_str = config.webcam_device.clone();
    let target_size = config.webcam_size;

    let handle = std::thread::Builder::new()
        .name("webcam-capture".into())
        .spawn(move || {
            if let Err(e) = webcam_capture_loop(&dir_str, &device_str, target_size) {
                tracing::error!("Webcam capture error: {}", e);
            }
        })
        .map_err(|e| e.to_string())?;

    *WEBCAM_THREAD.lock().unwrap() = Some(handle);

    tracing::info!("Webcam capture thread spawned (waiting for CAPTURE_ARMED)");
    Ok(())
}

/// Arm webcam capture — called when first video frame arrives.
/// Must be called from the same sync point as audio arming.
pub fn arm_webcam() {
    if !WEBCAM_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    WEBCAM_ARMED.store(true, Ordering::SeqCst);
    tracing::info!("Webcam ARMED — synced with video frame 0");
}

/// Stop webcam capture and wait for thread to finish.
/// Returns the directory containing webcam frames.
pub fn stop_webcam_capture() -> Option<String> {
    if !WEBCAM_ACTIVE.load(Ordering::SeqCst) {
        return None;
    }

    WEBCAM_STOP.store(true, Ordering::SeqCst);

    // Wait for thread to finish
    let handle = WEBCAM_THREAD.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = h.join();
    }

    let frame_count = WEBCAM_FRAME_COUNT.load(Ordering::Relaxed);
    tracing::info!("Webcam capture stopped: {} frames", frame_count);

    WEBCAM_ACTIVE.store(false, Ordering::SeqCst);

    if frame_count == 0 {
        return None;
    }

    WEBCAM_DIR.lock().unwrap().clone()
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

    // Create RGBA image
    let mut img = image::RgbaImage::new(size, size);

    // Parse border color
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
                    let r = 16.0_f32.min(cx); // corner radius
                    rounded_rect_contains(px, py, 0.0, 0.0, s as f32, s as f32, r)
                }
                WebcamShape::Squircle => {
                    squircle_contains(px, py, cx, cy, cx, 4.0)
                }
            };

            if inside {
                // Check if in border zone
                let border_zone = if bw > 0 {
                    let inner = match shape {
                        WebcamShape::Circle => {
                            let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                            dist > (cx - bw as f32)
                        }
                        WebcamShape::Rounded => {
                            let r = 16.0_f32.min(cx);
                            !rounded_rect_contains(px, py, bw as f32, bw as f32, (s - bw * 2) as f32, (s - bw * 2) as f32, (r - bw as f32).max(0.0))
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
                    // White = show webcam content
                    img.put_pixel(x as u32, y as u32, image::Rgba([255, 255, 255, 255]));
                }
            } else {
                // Transparent = hide
                img.put_pixel(x as u32, y as u32, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    img.save(&mask_path).map_err(|e| e.to_string())?;
    tracing::info!("Shape mask generated: {:?} ({}px)", shape, size);
    Ok(mask_path)
}

/// Composite webcam onto video using FFmpeg.
/// Runs after region crop, before audio merge.
pub fn composite_webcam_on_video(
    video_path: &str,
    webcam_dir: &str,
    mask_path: &str,
    config: &AppConfig,
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

    // FFmpeg command:
    // 1. Input video
    // 2. Input webcam frames as image sequence
    // 3. Input mask
    // 4. Scale webcam to size, apply mask as alpha, overlay on video

    let webcam_pattern = format!("{}/webcam_%06d.png", webcam_dir);

    let filter = if opacity < 1.0 {
        // With opacity: scale webcam, apply mask alpha, adjust opacity, overlay
        format!(
            "[1:v]scale={s}:{s},format=rgba[cam];\
             [2:v]scale={s}:{s},format=rgba[mask];\
             [cam][mask]alphamerge[masked];\
             [masked]colorchannelmixer=aa={op}[faded];\
             [0:v][faded]overlay={x}:{y}:shortest=1[out]",
            s = size, op = opacity, x = x, y = y
        )
    } else {
        // No opacity: scale webcam, apply mask alpha, overlay
        format!(
            "[1:v]scale={s}:{s},format=rgba[cam];\
             [2:v]scale={s}:{s},format=rgba[mask];\
             [cam][mask]alphamerge[masked];\
             [0:v][masked]overlay={x}:{y}:shortest=1[out]",
            s = size, x = x, y = y
        )
    };

    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    let args: Vec<String> = vec![
        "-y".into(),
        "-i".into(), video_path.into(),
        "-framerate".into(), "30".into(),
        "-i".into(), webcam_pattern,
        "-i".into(), mask_path.into(),
        "-filter_complex".into(), filter,
        "-map".into(), "[out]".into(),
        "-map".into(), "0:a?".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "fast".into(),
        "-crf".into(), "18".into(),
        "-c:a".into(), "copy".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        output_path.clone(),
    ];

    tracing::info!("Compositing webcam overlay: {} frames at ({}, {})", 
        WEBCAM_FRAME_COUNT.load(Ordering::Relaxed), x, y);

    let output = std::process::Command::new(&ffmpeg)
        .args(&args)
        .output()
        .map_err(|e| format!("FFmpeg webcam composite failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Try fallback without mask (just overlay, no shape)
        tracing::warn!("Masked overlay failed, trying simple overlay: {}", stderr);
        return composite_simple_overlay(video_path, webcam_dir, config);
    }

    tracing::info!("Webcam overlay composited successfully");
    Ok(output_path)
}

/// Fallback: simple overlay without shape mask
fn composite_simple_overlay(
    video_path: &str,
    webcam_dir: &str,
    config: &AppConfig,
) -> Result<String, String> {
    let output_path = std::env::temp_dir()
        .join("easyspecy")
        .join("video_webcam.mp4")
        .to_string_lossy()
        .to_string();

    let x = config.webcam_x.max(0).to_string();
    let y = config.webcam_y.max(0).to_string();
    let size = config.webcam_size;
    let opacity = config.webcam_opacity.clamp(0.0, 1.0);

    let webcam_pattern = format!("{}/webcam_%06d.png", webcam_dir);
    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    let filter = format!(
        "[1:v]scale={s}:{s},format=rgba[cam];\
         [0:v][cam]overlay={x}:{y}:shortest=1[out]",
        s = size, x = x, y = y
    );

    let args: Vec<String> = vec![
        "-y".into(),
        "-i".into(), video_path.into(),
        "-framerate".into(), "30".into(),
        "-i".into(), webcam_pattern,
        "-filter_complex".into(), filter,
        "-map".into(), "[out]".into(),
        "-map".into(), "0:a?".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "fast".into(),
        "-crf".into(), "18".into(),
        "-c:a".into(), "copy".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        output_path.clone(),
    ];

    let output = std::process::Command::new(&ffmpeg)
        .args(&args)
        .output()
        .map_err(|e| format!("FFmpeg simple overlay failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Webcam overlay failed: {}", stderr));
    }

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

fn webcam_capture_loop(
    output_dir: &str,
    device_str: &str,
    target_size: u32,
) -> Result<(), String> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
    use nokhwa::Camera;

    // Wait for CAPTURE_ARMED
    tracing::info!("Webcam thread waiting for CAPTURE_ARMED...");
    while !WEBCAM_ARMED.load(Ordering::SeqCst) {
        if WEBCAM_STOP.load(Ordering::SeqCst) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    // Initialize camera
    let index = if device_str == "default" || device_str.is_empty() {
        CameraIndex::Index(0)
    } else {
        device_str
            .parse::<u32>()
            .map(CameraIndex::Index)
            .unwrap_or(CameraIndex::Index(0))
    };

    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

    let mut camera =
        Camera::new(index, format).map_err(|e| format!("Failed to open webcam: {}", e))?;

    camera
        .open_stream()
        .map_err(|e| format!("Failed to start webcam stream: {}", e))?;

    tracing::info!("Webcam stream started (target_size={}px)", target_size);

    // Get camera resolution
    let resolution = camera.resolution();
    let cam_w = resolution.width();
    let cam_h = resolution.height();
    tracing::info!("Webcam resolution: {}x{}", cam_w, cam_h);

    if cam_w == 0 || cam_h == 0 {
        tracing::error!("Webcam returned zero resolution!");
        camera.stop_stream().ok();
        return Ok(());
    }

    let frame_interval = Duration::from_millis(33); // ~30fps
    let mut last_frame_time = Instant::now();
    let mut frame_idx: u32 = 0;

    loop {
        if WEBCAM_STOP.load(Ordering::SeqCst) {
            break;
        }

        let now = Instant::now();
        if now.duration_since(last_frame_time) < frame_interval {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        last_frame_time = now;

        // Capture frame
        match camera.frame() {
            Ok(frame) => {
                let frame_path = format!("{}/webcam_{:06}.png", output_dir, frame_idx);
                let raw = frame.buffer();
                let raw_len = raw.len();

                // Use frame's actual resolution (may differ from camera.resolution())
                let frame_res = frame.resolution();
                let fw = frame_res.width();
                let fh = frame_res.height();

                if frame_idx == 0 {
                    tracing::info!(
                        "First webcam frame: buffer_len={}, frame_res={}x{}, camera_res={}x{}, target_size={}",
                        raw_len, fw, fh, cam_w, cam_h, target_size
                    );
                }

                // Validate buffer size — raw should be fw*fh*3 (RGB)
                let expected_rgb = (fw * fh * 3) as usize;
                let expected_rgba = (fw * fh * 4) as usize;

                let rgba: Vec<u8> = if raw_len == expected_rgb {
                    // Standard RGB → RGBA conversion
                    let mut rgba = Vec::with_capacity(expected_rgba);
                    for chunk in raw.chunks(3) {
                        if chunk.len() >= 3 {
                            rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                        }
                    }
                    rgba
                } else if raw_len == expected_rgba {
                    // Already RGBA
                    raw.to_vec()
                } else {
                    if frame_idx == 0 {
                        tracing::error!(
                            "Webcam buffer size mismatch: got {} bytes, expected {} (RGB) or {} (RGBA) for {}x{}",
                            raw_len, expected_rgb, expected_rgba, fw, fh
                        );
                    }
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };

                // Build image from the actual frame dimensions
                if let Some(img) = image::RgbaImage::from_raw(fw, fh, rgba) {
                    let resized = image::imageops::resize(
                        &img,
                        target_size,
                        target_size,
                        image::imageops::FilterType::Triangle,
                    );
                    if let Err(e) = resized.save(&frame_path) {
                        tracing::warn!("Failed to save webcam frame {}: {}", frame_idx, e);
                    } else {
                        frame_idx += 1;
                        WEBCAM_FRAME_COUNT.store(frame_idx, Ordering::Relaxed);
                    }
                } else {
                    tracing::warn!(
                        "RgbaImage::from_raw failed: {} bytes for {}x{}", raw_len, fw, fh
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Webcam frame capture error: {}", e);
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        if frame_idx > 0 && frame_idx % 30 == 0 {
            tracing::info!("Webcam: {} frames captured", frame_idx);
        }
    }

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
        [0, 232, 138] // default emerald
    }
}

fn rounded_rect_contains(px: f32, py: f32, rx: f32, ry: f32, w: f32, h: f32, r: f32) -> bool {
    // Check if point is inside rounded rectangle
    if px < rx || px > rx + w || py < ry || py > ry + h {
        return false;
    }
    // Check corners
    let corners = [
        (rx + r, ry + r),
        (rx + w - r, ry + r),
        (rx + r, ry + h - r),
        (rx + w - r, ry + h - r),
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
