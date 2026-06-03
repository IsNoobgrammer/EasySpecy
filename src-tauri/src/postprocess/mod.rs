#![allow(dead_code)]
//! Post-processing module — FFmpeg encoding, auto-zoom, cursor trail filters
//!
//! Phase 2: Cursor trail + click metadata collection during recording.
//! On stop, metadata is fed to FFmpeg filter chains for baked-in effects.

use serde::{Deserialize, Serialize};
use rayon::prelude::*;
use std::sync::Mutex;
use std::time::Instant;

/// A single cursor position sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSample {
    pub timestamp_ms: u64,
    pub x: f32,
    pub y: f32,
}

/// A single click event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEvent {
    pub timestamp_ms: u64,
    pub x: f32,
    pub y: f32,
    pub button: String,
}

/// Pre-computed confetti particle state at a specific frame (internal)
#[derive(Debug, Clone)]
struct ConfettiState {
    x: f32,
    y: f32,
    size: f32,
    color: (u8, u8, u8),
    rotation: f32,
    shape: u8,  // 0=rect, 1=circle, 2=triangle
    alpha: f32,
}

/// Window bounds captured at click time (for auto-zoom)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBoundsEvent {
    pub timestamp_ms: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub title: String,
}

/// Keyboard event with key name and timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    pub timestamp_ms: u64,
    pub key: String,
}


/// Collected cursor + click metadata for a recording session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingMetadata {
    pub cursor_trail: Vec<CursorSample>,
    pub click_events: Vec<ClickEvent>,
    pub window_events: Vec<WindowBoundsEvent>,
    pub keyboard_events: Vec<KeyboardEvent>,
    pub trail_style: String,
    pub click_effect: String,
    pub trail_color: String,
    pub secondary_color: String,
}

static METADATA: Mutex<Option<RecordingMetadata>> = Mutex::new(None);
static SESSION_START: Mutex<Option<Instant>> = Mutex::new(None);

/// Start collecting cursor metadata for a new recording session
pub fn start_collection() {
    let config = crate::config::AppConfig::load();
    tracing::info!(
        "start_collection: trail='{}', click='{}', color='{}'",
        config.trail_style, config.click_effect, config.cursor_trail_color
    );
    *METADATA.lock().unwrap() = Some(RecordingMetadata {
        trail_style: config.trail_style.clone(),
        click_effect: config.click_effect.clone(),
        trail_color: config.cursor_trail_color.clone(),
        secondary_color: config.cursor_secondary_color.clone(),
        ..Default::default()
    });
    *SESSION_START.lock().unwrap() = Some(Instant::now());
    tracing::info!("Cursor metadata collection started");
}

/// Reset session start time to NOW — called when CAPTURE_ARMED fires
/// to sync cursor timestamps with video frame 0.
pub fn reset_session_start() {
    *SESSION_START.lock().unwrap() = Some(Instant::now());
    // Clear any samples recorded before arming (they have wrong timestamps)
    if let Some(ref mut m) = *METADATA.lock().unwrap() {
        m.cursor_trail.clear();
        m.click_events.clear();
        m.keyboard_events.clear();
    }
    // Also reset keyboard capture start time
    crate::keyboard::reset_keyboard_start_time();

    tracing::info!("Cursor and keyboard session start reset to CAPTURE_ARMED instant");
}

/// Record a cursor position sample
pub fn record_cursor(x: f32, y: f32) {
    let start = SESSION_START.lock().unwrap();
    if let Some(t) = *start {
        let ms = t.elapsed().as_millis() as u64;
        let mut meta = METADATA.lock().unwrap();
        if let Some(ref mut m) = *meta {
            // Only record if cursor actually MOVED (skip stationary duplicates)
            let moved = m.cursor_trail.last().map_or(true, |s| {
                let dx = (x - s.x).abs();
                let dy = (y - s.y).abs();
                dx > 0.5 || dy > 0.5 // Must move at least 0.5px
            });
            if moved {
                m.cursor_trail.push(CursorSample { timestamp_ms: ms, x, y });
            }
        }
    }
}

/// Record a click event
pub fn record_click(x: f32, y: f32, button: &str) {
    let start = SESSION_START.lock().unwrap();
    if let Some(t) = *start {
        let ms = t.elapsed().as_millis() as u64;
        let mut meta = METADATA.lock().unwrap();
        if let Some(ref mut m) = *meta {
            m.click_events.push(ClickEvent { timestamp_ms: ms, x, y, button: button.to_string() });
        }
    }
}

/// Record window bounds at click time (for auto-zoom)
pub fn record_window_bounds(x: i32, y: i32, width: i32, height: i32, title: &str) {
    let start = SESSION_START.lock().unwrap();
    if let Some(t) = *start {
        let ms = t.elapsed().as_millis() as u64;
        let mut meta = METADATA.lock().unwrap();
        if let Some(ref mut m) = *meta {
            m.window_events.push(WindowBoundsEvent {
                timestamp_ms: ms,
                x,
                y,
                width,
                height,
                title: title.chars().take(256).collect(),
            });
        }
    }
}

/// Record a keyboard event timestamp + key
pub fn record_keyboard_event(key: &str) {
    let start = SESSION_START.lock().unwrap();
    if let Some(t) = *start {
        let ms = t.elapsed().as_millis() as u64;
        let mut meta = METADATA.lock().unwrap();
        if let Some(ref mut m) = *meta {
            m.keyboard_events.push(KeyboardEvent { timestamp_ms: ms, key: key.to_string() });
        }
    }
}

/// Finalize and return the collected metadata
pub fn finalize() -> Option<RecordingMetadata> {
    let meta = METADATA.lock().unwrap().take();
    *SESSION_START.lock().unwrap() = None;
    if let Some(ref m) = meta {
        tracing::info!(
            "Cursor metadata finalized: {} trail samples, {} clicks",
            m.cursor_trail.len(),
            m.click_events.len()
        );
    }
    meta
}

/// Save metadata to a JSON file alongside the recording
pub fn save_metadata(meta: &RecordingMetadata, output_path: &str) -> Result<(), String> {
    let meta_path = output_path.replace(".mp4", ".meta.json");
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(&meta_path, json).map_err(|e| e.to_string())?;
    tracing::info!("Metadata saved to {}", meta_path);
    Ok(())
}

/// Apply cursor trail and click effects to the recorded video using FFmpeg.
/// Renders smooth trail as transparent PNG frames, composites via FFmpeg overlay.
/// Returns the path to the effects-applied video (or input if no effects).
#[allow(unreachable_code, unused_variables)]
pub fn apply_effects(input: &str, _output: &str, meta: &RecordingMetadata) -> Result<String, String> {
    tracing::info!("apply_effects: bypassing post-processing overlays (recorded live)");
    return Ok(input.to_string());

    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    // ═══ ORIGINAL PIPELINE (overlay only — no zoom) ═══

    tracing::info!(
        "apply_effects (overlay): trail='{}', click='{}', samples={}, clicks={}",
        meta.trail_style, meta.click_effect,
        meta.cursor_trail.len(), meta.click_events.len()
    );

    // Skip if no effects enabled
    if meta.trail_style == "none" && meta.click_effect == "none" {
        return Ok(input.to_string());
    }
    if meta.cursor_trail.is_empty() && meta.click_events.is_empty() {
        tracing::info!("apply_effects skipped: no trail/click data");
        return Ok(input.to_string());
    }

    tracing::info!("apply_effects proceeding: {} trail samples, {} clicks",
        meta.cursor_trail.len(), meta.click_events.len()
    );

    let config = crate::config::AppConfig::load();
    let fps = config.fps;

    // Video is captured at MONITOR resolution, not config resolution
    // Get actual monitor size for the trail frame buffer
    let region = crate::region::get_region();
    let (width, height) = if let Some(ref r) = region {
        (r.width as u32, r.height as u32)
    } else {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            let w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
            let h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
            (w, h)
        }
        #[cfg(not(target_os = "windows"))]
        {
            (config.resolution_width, config.resolution_height)
        }
    };

    // Get actual video duration by probing with FFmpeg
    let video_duration_ms = probe_video_duration(&ffmpeg, input).unwrap_or_else(|| {
        // Fallback: use last cursor sample timestamp
        meta.cursor_trail.last().map(|s| s.timestamp_ms).unwrap_or(0)
    });
    let total_frames = ((video_duration_ms as f64 / 1000.0) * fps as f64).ceil() as u32;

    tracing::info!("apply_effects: video_dur={}ms, fps={}, total_frames={}",
        video_duration_ms, fps, total_frames
    );
    if total_frames == 0 {
        tracing::info!("apply_effects skipped: total_frames=0");
        return Ok(input.to_string());
    }

    // Create temp dir for trail frames
    let trail_dir = std::env::temp_dir().join("easyspecy_trail");
    let _ = std::fs::remove_dir_all(&trail_dir);
    std::fs::create_dir_all(&trail_dir).map_err(|e| e.to_string())?;

    tracing::info!("Rendering {} trail frames at {}x{} {}fps", total_frames, width, height, fps);

    // Parse trail color + secondary color (for right-click, gradients)
    let color = parse_hex_color(&meta.trail_color);
    let secondary_color = parse_hex_color(&meta.secondary_color);

    // ═══ PRE-SMOOTH THE ENTIRE CURSOR PATH ═══
    // This is the key: build one continuous smooth curve from all samples FIRST,
    // then sample it per-frame. No per-frame interpolation jitter.
    let smooth_path = build_smooth_path(&meta.cursor_trail);
    tracing::info!("Smooth path: {} points from {} raw samples", smooth_path.len(), meta.cursor_trail.len());

    // ═══ Pipe raw RGBA frames to FFmpeg ═══
    let effects_path = input.replace(".mp4", "_effects.mp4");

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args([
        "-y",
        "-i", input,
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-s", &format!("{}x{}", width, height),
        "-r", &fps.to_string(),
        "-i", "pipe:0",
        "-filter_complex", "[0:v][1:v]overlay=0:0:format=auto[out]",
        "-map", "[out]",
        "-map", "0:a?",
        "-c:v", "libx264",
        "-preset", "fast",
        "-crf", "18",
        "-threads", "0",
        "-thread_type", "frame+slice",
        "-c:a", "copy",
        "-shortest",
        &effects_path,
    ])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| format!("FFmpeg spawn error: {}", e))?;
    let mut stdin = child.stdin.take().ok_or("Failed to open FFmpeg stdin")?;

    let frame_size = (width * height * 4) as usize;
    let ms_per_frame = 1000.0 / fps as f64;
    let trail_duration_ms: f64 = 600.0; // Comet tail length in ms (configurable)

    use std::io::Write;

    // ═══ PRE-COMPUTE PER-FRAME DATA (PARALLEL) ═══
    // Each frame's trail segment + click data is independent → parallel compute.
    let smooth_path_ref = &smooth_path;
    let click_ref = &meta.click_events;
    let region_ref = &region;
    let pre_computed: Vec<(Vec<(f32, f32, f64, f64)>, Vec<(f32, f32, f64, bool)>)> = (0..total_frames)
        .into_par_iter()
        .map(|frame_idx| {
            let frame_time_ms = frame_idx as f64 * ms_per_frame;
            let head_idx = find_path_index_at_time(smooth_path_ref, frame_time_ms);

            // Trail segment with motion blur speed
            let segment: Vec<(f32, f32, f64, f64)> = if head_idx > 0 {
                let tail_start_time = (frame_time_ms - trail_duration_ms).max(0.0);
                let tail_idx = find_path_index_at_time(smooth_path_ref, tail_start_time);
                if head_idx > tail_idx {
                    let span = (head_idx - tail_idx).max(1) as f64;
                    let seg: Vec<(f32, f32, f64, f64)> = (tail_idx..=head_idx)
                        .map(|i| {
                            let p = &smooth_path_ref[i];
                            let age = 1.0 - ((i - tail_idx) as f64 / span);
                            // Speed between this point and the next (px/ms)
                            let speed = if i + 1 < smooth_path_ref.len() {
                                let p_next = &smooth_path_ref[i + 1];
                                let dx = (p_next.0 - p.0) as f64;
                                let dy = (p_next.1 - p.1) as f64;
                                let dt = (p_next.2 - p.2).abs().max(1.0);
                                (dx * dx + dy * dy).sqrt() / dt
                            } else {
                                0.0
                            };
                            let (px, py) = if let Some(ref r) = region_ref {
                                (p.0 - r.x as f32, p.1 - r.y as f32)
                            } else {
                                (p.0, p.1)
                            };
                            (px, py, age, speed)
                        })
                        .collect();
                    seg
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // Active clicks — with left/right button detection
            let click_window_ms = trail_duration_ms.max(400.0);
            let active_clicks: Vec<(f32, f32, f64, bool)> = click_ref
                .iter()
                .filter_map(|click| {
                    let click_age_ms = frame_time_ms - click.timestamp_ms as f64;
                    if click_age_ms >= 0.0 && click_age_ms < click_window_ms {
                        let is_right = click.button == "right";
                        let (cx, cy) = if let Some(ref r) = region_ref {
                            (click.x - r.x as f32, click.y - r.y as f32)
                        } else {
                            (click.x, click.y)
                        };
                        Some((cx, cy, click_age_ms / click_window_ms, is_right))
                    } else {
                        None
                    }
                })
                .collect();

            (segment, active_clicks)
        })
        .collect();

    // ═══ PRE-COMPUTE CONFETTI PARTICLES (SEQUENTIAL) ═══
    // Confetti particles have persistent physics state across frames,
    // so we simulate them sequentially once, then use the results in parallel rendering.
    let confetti_per_frame = precompute_confetti(
        &meta.click_events,
        total_frames,
        fps,
        &meta.click_effect,
        color,
        secondary_color,
        region.as_ref(),
    );

    tracing::info!(
        "Pre-computed {} frames, starting parallel render (batch=8)",
        total_frames
    );

    // ═══ PARALLEL BATCH RENDER → FFmpeg PIPE ═══
    // Render 8 frames in parallel, write in order. Each frame is independent.
    // Use all available CPU cores for batch rendering
    let batch_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2) // 2x cores — each frame is independent, slight oversubscription helps
        .unwrap_or(8)
        .max(8);

    // Find first/last frames with cursor activity for start/stop markers
    let first_cursor_frame = pre_computed.iter().position(|(seg, _)| seg.len() >= 2);
    let last_cursor_frame = pre_computed.iter().rposition(|(seg, _)| seg.len() >= 2);

    // Parse keyboard overlay bubbles
    let bubble_timeout_ms = config.keyboard_overlay_bubble_timeout_ms as u64;
    let max_bubbles = config.keyboard_overlay_max_bubbles as usize;
    let keyboard_bubbles = parse_keyboard_bubbles(&meta.keyboard_events, bubble_timeout_ms, max_bubbles);

    for batch_start in (0..total_frames as usize).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total_frames as usize);

        // Render batch in parallel with rayon
        let rendered: Vec<Vec<u8>> = (batch_start..batch_end)
            .into_par_iter()
            .map(|frame_idx| {
                let mut frame_buf = vec![0u8; frame_size];
                let (ref segment, ref active_clicks) = pre_computed[frame_idx];

                // Trail
                if segment.len() >= 2 {
                    render_smooth_trail(
                        &mut frame_buf,
                        width,
                        height,
                        segment,
                        color,
                        secondary_color,
                        &meta.trail_style,
                        1.0,
                    );
                }

                // Click effects — use left/right button color
                for &(cx, cy, progress, is_right) in active_clicks {
                    let click_color = if is_right { secondary_color } else { color };
                    render_click_effect(
                        &mut frame_buf,
                        width,
                        height,
                        cx,
                        cy,
                        progress,
                        click_color,
                        &meta.click_effect,
                    );
                }

                // Confetti particles
                let confetti_slice = &confetti_per_frame[frame_idx];
                for cs in confetti_slice {
                    render_confetti_particle(&mut frame_buf, width, height, cs);
                }

                // Keyboard overlay
                if config.keyboard_overlay_enabled {
                    let frame_time_ms = frame_idx as f64 * ms_per_frame;
                    let active_bubbles = get_active_bubbles_at_time(&keyboard_bubbles, frame_time_ms as u64, max_bubbles);
                    if !active_bubbles.is_empty() {
                        let scale_factor = crate::app_handle()
                            .and_then(|app| app.primary_monitor().ok().flatten())
                            .map(|m| m.scale_factor())
                            .unwrap_or(1.0);
                        render_keyboard_overlay(
                            &mut frame_buf,
                            width,
                            height,
                            &active_bubbles,
                            &config,
                            scale_factor,
                            region.as_ref(),
                        );
                    }
                }

                // Recording start/stop markers
                if Some(frame_idx) == first_cursor_frame
                    || Some(frame_idx) == last_cursor_frame
                {
                    if let Some(&(x, y, _, _)) = segment.last() {
                        draw_glow_circle(
                            &mut frame_buf,
                            width,
                            height,
                            x,
                            y,
                            40.0,
                            (255, 255, 255),
                            0.3,
                        );
                        draw_glow_circle(
                            &mut frame_buf,
                            width,
                            height,
                            x,
                            y,
                            25.0,
                            (255, 255, 255),
                            0.5,
                        );
                        draw_glow_circle(
                            &mut frame_buf,
                            width,
                            height,
                            x,
                            y,
                            15.0,
                            (255, 255, 255),
                            0.6,
                        );
                    }
                }

                frame_buf
            })
            .collect();

        // Write batch in order (FFmpeg needs sequential frames)
        for frame_buf in rendered {
            if stdin.write_all(&frame_buf).is_err() {
                break;
            }
        }
    }

    drop(stdin); // Close pipe → FFmpeg finishes

    let output = child.wait_with_output().map_err(|e| format!("FFmpeg wait error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("FFmpeg effects failed: {}", &stderr[..stderr.len().min(500)]);
        let _ = std::fs::remove_dir_all(&trail_dir);
        return Ok(input.to_string());
    }

    let _ = std::fs::remove_dir_all(&trail_dir);
    tracing::info!("apply_effects complete: {} frames", total_frames);
    Ok(effects_path)
}

/// Build a pre-smoothed path from raw cursor samples using Catmull-Rom interpolation.
/// Returns uniformly-spaced points along the smooth curve, each with timestamp.
fn build_smooth_path(samples: &[CursorSample]) -> Vec<(f32, f32, f64)> {
    if samples.is_empty() { return Vec::new(); }
    if samples.len() == 1 {
        return vec![(samples[0].x, samples[0].y, samples[0].timestamp_ms as f64)];
    }

    let mut path: Vec<(f32, f32, f64)> = Vec::with_capacity(samples.len() * 4);

    for i in 0..samples.len() - 1 {
        let p0 = &samples[i.saturating_sub(1)];
        let p1 = &samples[i];
        let p2 = &samples[(i + 1).min(samples.len() - 1)];
        let p3 = &samples[(i + 2).min(samples.len() - 1)];

        // Adaptive step count based on distance between p1 and p2
        let dist = ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt();
        // More steps for longer segments (1 step per ~3px), min 2, max 10
        let steps = (dist / 3.0).max(2.0).min(10.0) as u32;

        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let x = catmull_rom(p0.x, p1.x, p2.x, p3.x, t);
            let y = catmull_rom(p0.y, p1.y, p2.y, p3.y, t);
            // Interpolate timestamp linearly
            let time = p1.timestamp_ms as f64 + (p2.timestamp_ms as f64 - p1.timestamp_ms as f64) * t as f64;
            path.push((x, y, time));
        }
    }

    // Add final point
    let last = samples.last().unwrap();
    path.push((last.x, last.y, last.timestamp_ms as f64));

    // Deduplicate very close points (< 0.5px apart)
    let mut deduped: Vec<(f32, f32, f64)> = Vec::with_capacity(path.len());
    for p in &path {
        if deduped.last().map_or(true, |prev: &(f32, f32, f64)| {
            (p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2) > 0.25
        }) {
            deduped.push(*p);
        }
    }

    deduped
}

/// Binary search smooth_path for the index closest to given time_ms
fn find_path_index_at_time(path: &[(f32, f32, f64)], time_ms: f64) -> usize {
    if path.is_empty() { return 0; }
    if time_ms <= path[0].2 { return 0; }
    if time_ms >= path.last().unwrap().2 { return path.len() - 1; }

    // Binary search
    let mut lo = 0;
    let mut hi = path.len() - 1;
    while lo < hi - 1 {
        let mid = (lo + hi) / 2;
        if path[mid].2 <= time_ms {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    // Return closest
    if (time_ms - path[lo].2).abs() < (time_ms - path[hi].2).abs() { lo } else { hi }
}

/// Probe video duration in milliseconds using FFmpeg
fn probe_video_duration(ffmpeg: &str, input: &str) -> Option<u64> {
    // Use ffprobe-style: ffmpeg -i file -f null - (read duration from stderr)
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(["-i", input, "-f", "null", "-"]);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd.output().ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Parse "Duration: HH:MM:SS.ms" from stderr
    for line in stderr.lines() {
        if let Some(pos) = line.find("Duration:") {
            let dur_str = &line[pos + 9..].trim();
            if let Some(comma) = dur_str.find(',') {
                let time_str = &dur_str[..comma].trim();
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() == 3 {
                    let h: f64 = parts[0].parse().unwrap_or(0.0);
                    let m: f64 = parts[1].parse().unwrap_or(0.0);
                    let s: f64 = parts[2].parse().unwrap_or(0.0);
                    let total_ms = ((h * 3600.0 + m * 60.0 + s) * 1000.0) as u64;
                    if total_ms > 0 {
                        return Some(total_ms);
                    }
                }
            }
        }
    }
    None
}

/// Interpolate cursor position at exact frame time using linear interp between samples
#[allow(dead_code)]
fn interpolate_cursor_at_time(samples: &[CursorSample], time_ms: f64) -> Option<(f32, f32)> {
    if samples.is_empty() { return None; }

    // Before first sample
    if time_ms <= samples[0].timestamp_ms as f64 {
        return Some((samples[0].x, samples[0].y));
    }
    // After last sample — hold last position
    if time_ms >= samples.last().unwrap().timestamp_ms as f64 {
        let last = samples.last().unwrap();
        return Some((last.x, last.y));
    }

    // Binary search for surrounding samples
    let mut lo = 0;
    let mut hi = samples.len() - 1;
    while lo < hi - 1 {
        let mid = (lo + hi) / 2;
        if samples[mid].timestamp_ms as f64 <= time_ms {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let s0 = &samples[lo];
    let s1 = &samples[hi];
    let dt = (s1.timestamp_ms - s0.timestamp_ms) as f64;
    if dt <= 0.0 {
        return Some((s0.x, s0.y));
    }

    let t = (time_ms - s0.timestamp_ms as f64) / dt;
    let x = s0.x + (s1.x - s0.x) * t as f32;
    let y = s0.y + (s1.y - s0.y) * t as f32;
    Some((x, y))
}

/// Parse hex color string to (r, g, b) tuple
fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    if h.len() >= 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(136);
        (r, g, b)
    } else {
        (0, 255, 136) // default green
    }
}

/// Catmull-Rom spline interpolation between 4 points
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Draw anti-aliased circle with glow onto RGBA buffer
fn draw_glow_circle(buf: &mut [u8], w: u32, h: u32, cx: f32, cy: f32, radius: f32, color: (u8, u8, u8), alpha: f32) {
    // Skip negligible alpha — avoids touching pixels for invisible glows
    if alpha < 0.005 || radius < 0.5 {
        return;
    }
    let r2 = radius * radius;
    let x_min = ((cx - radius - 2.0).max(0.0)) as u32;
    let x_max = ((cx + radius + 2.0).min(w as f32 - 1.0)) as u32;
    let y_min = ((cy - radius - 2.0).max(0.0)) as u32;
    let y_max = ((cy + radius + 2.0).min(h as f32 - 1.0)) as u32;

    for py in y_min..=y_max {
        // Row-stride: precompute row base index (avoids multiply per pixel)
        let row_base = (py * w * 4) as usize;
        for px in x_min..=x_max {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= r2 {
                // Inside circle — full alpha with soft edge
                let edge_factor = 1.0 - (dist_sq / r2).sqrt();
                let a = (alpha * edge_factor * 255.0) as u8;
                let idx = row_base + (px * 4) as usize;
                if idx + 3 < buf.len() {
                    // Alpha-blend (premultiplied)
                    let src_a = a as f32 / 255.0;
                    let dst_a = buf[idx + 3] as f32 / 255.0;
                    let out_a = src_a + dst_a * (1.0 - src_a);
                    if out_a > 0.0 {
                        buf[idx] = ((color.0 as f32 * src_a + buf[idx] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        buf[idx + 1] = ((color.1 as f32 * src_a + buf[idx + 1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        buf[idx + 2] = ((color.2 as f32 * src_a + buf[idx + 2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        buf[idx + 3] = (out_a * 255.0) as u8;
                    }
                }
            }
        }
    }
}

/// Linear interpolation between two RGB colors
fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t) as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t) as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t) as u8,
    )
}

/// Render smooth cursor trail onto RGBA frame buffer
fn render_smooth_trail(
    buf: &mut [u8],
    w: u32,
    h: u32,
    points: &[(f32, f32, f64, f64)], // (x, y, age 0..1, speed px/ms)
    color: (u8, u8, u8),
    secondary_color: (u8, u8, u8),
    style: &str,
    trail_width: f32,
) {
    if points.len() < 2 { return; }

    // Interpolate between points using Catmull-Rom for smoothness
    let mut interpolated: Vec<(f32, f32, f64, f64)> = Vec::with_capacity(points.len() * 4);

    for i in 0..points.len() - 1 {
        let p0 = points[i.saturating_sub(1)];
        let p1 = points[i];
        let p2 = points[(i + 1).min(points.len() - 1)];
        let p3 = points[(i + 2).min(points.len() - 1)];

        let dist = ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
        let steps = (dist / 3.0).max(2.0).min(8.0) as u32;

        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let x = catmull_rom(p0.0, p1.0, p2.0, p3.0, t);
            let y = catmull_rom(p0.1, p1.1, p2.1, p3.1, t);
            let age = p1.2 + (p2.2 - p1.2) * t as f64;
            let speed = p1.3 + (p2.3 - p1.3) * t as f64;
            interpolated.push((x, y, age, speed));
        }
    }
    // Add last point
    if let Some(&last) = points.last() {
        interpolated.push(last);
    }

    // Draw trail based on style
    match style {
        "glow" => {
            // Multi-layer glow: outer diffuse → core bright
            // Per-style palette: blend primary→secondary along trail, motion blur via speed
            for &(x, y, age, speed) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                // Motion blur: faster = bigger glow (max 1.5x), slightly reduced alpha
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                let alpha_mod = (1.0 - speed as f32 * 0.03).max(0.7);
                // Per-style color palette: blend primary→secondary along trail
                let blended = lerp_color(color, secondary_color, age as f32);
                // Outer glow
                draw_glow_circle(buf, w, h, x, y, 12.0 * life * speed_factor * trail_width, blended, 0.04 * life * alpha_mod);
                draw_glow_circle(buf, w, h, x, y, 7.0 * life * speed_factor * trail_width, blended, 0.12 * life * alpha_mod);
                // Core
                draw_glow_circle(buf, w, h, x, y, 3.5 * life * speed_factor * trail_width, blended, 0.6 * life * alpha_mod);
                // White center
                draw_glow_circle(buf, w, h, x, y, 1.5 * life * trail_width, (255, 255, 255), 0.5 * life);
            }
            // Extra bright head glow at newest point
            if let Some(&(x, y, _, speed)) = interpolated.last() {
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                draw_glow_circle(buf, w, h, x, y, 16.0 * speed_factor * trail_width, color, 0.08);
                draw_glow_circle(buf, w, h, x, y, 9.0 * speed_factor * trail_width, color, 0.2);
                draw_glow_circle(buf, w, h, x, y, 4.0 * trail_width, color, 0.8);
                draw_glow_circle(buf, w, h, x, y, 2.0 * trail_width, (255, 255, 255), 0.9);
            }
        }
        "particles" | "dots" => {
            for &(x, y, age, speed) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                let size = (3.0 + life * 4.0) * speed_factor * trail_width;
                draw_glow_circle(buf, w, h, x, y, size, color, 0.7 * life);
                draw_glow_circle(buf, w, h, x, y, size * 0.4 * trail_width, (255, 255, 255), 0.5 * life);
            }
        }
        "ribbon" => {
            for &(x, y, age, speed) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                let alpha_mod = (1.0 - speed as f32 * 0.03).max(0.7);
                // Wider, more diffuse
                draw_glow_circle(buf, w, h, x, y, 8.0 * life * speed_factor * trail_width, color, 0.15 * life * alpha_mod);
                draw_glow_circle(buf, w, h, x, y, 4.0 * life * speed_factor * trail_width, color, 0.4 * life * alpha_mod);
                draw_glow_circle(buf, w, h, x, y, 1.5 * life * trail_width, (255, 255, 255), 0.3 * life);
            }
        }
        "aurora" => {
            for (i, &(x, y, age, speed)) in interpolated.iter().enumerate() {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                // Shift hue along trail
                let hue_shift = (i as f32 * 3.0) % 360.0;
                let shifted_color = hue_rotate_rgb(color, hue_shift);
                draw_glow_circle(buf, w, h, x, y, 10.0 * life * speed_factor * trail_width, shifted_color, 0.08 * life);
                draw_glow_circle(buf, w, h, x, y, 5.0 * life * speed_factor * trail_width, shifted_color, 0.2 * life);
                draw_glow_circle(buf, w, h, x, y, 2.0 * life * trail_width, (255, 255, 255), 0.3 * life);
            }
        }
        _ => {
            // Default: simple dots
            for &(x, y, age, speed) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                let speed_factor = (1.0 + speed as f32 * 0.1).min(1.5);
                draw_glow_circle(buf, w, h, x, y, 4.0 * life * speed_factor * trail_width, color, 0.6 * life);
            }
        }
    }
}

/// Render click effect onto RGBA frame buffer
fn render_click_effect(
    buf: &mut [u8],
    w: u32,
    h: u32,
    cx: f32,
    cy: f32,
    progress: f64, // 0.0 = just clicked, 1.0 = fully faded
    color: (u8, u8, u8),
    style: &str,
) {
    let p = progress as f32;
    if p >= 1.0 {
        return;
    }

    // Scale factor for 1080p — effects designed for this resolution
    let sf = (w as f32 / 1920.0).max(0.5);

    match style {
        "ripple" => {
            // 4 expanding rings with staggered timing + bright flash
            for ring in 0..4 {
                let ring_p = (p - ring as f32 * 0.1).max(0.0) / 0.65;
                if ring_p >= 1.0 || ring_p <= 0.0 {
                    continue;
                }
                let radius = ring_p * 60.0 * sf;
                let alpha = (1.0 - ring_p) * 0.7;
                // Draw ring as glow + bright stroke
                draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha * 0.3);
                draw_glow_circle(
                    buf,
                    w,
                    h,
                    cx,
                    cy,
                    (radius - 3.0 * sf).max(1.0),
                    color,
                    alpha,
                );
            }
            // Bright center flash
            if p < 0.2 {
                let flash = 1.0 - p / 0.2;
                draw_glow_circle(buf, w, h, cx, cy, 12.0 * sf, (255, 255, 255), flash * 0.9);
                draw_glow_circle(buf, w, h, cx, cy, 8.0 * sf, color, flash * 0.7);
            }
        }
        "spotlight" => {
            let radius = (10.0 + p * 60.0) * sf;
            let alpha = (1.0 - p) * 0.65;
            // Outer glow
            draw_glow_circle(buf, w, h, cx, cy, radius * 1.5, color, alpha * 0.15);
            // Main spotlight
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
            // Bright inner core
            draw_glow_circle(
                buf,
                w,
                h,
                cx,
                cy,
                radius * 0.4,
                (255, 255, 255),
                alpha * 0.7,
            );
            // Cross flare rays
            if p < 0.5 {
                let ray_alpha = (1.0 - p / 0.5) * 0.5;
                let ray_len = (20.0 + p * 40.0) * sf;
                for a in 0..6 {
                    let angle = a as f32 * std::f32::consts::PI / 3.0 + p * 0.5;
                    let rx = cx + angle.cos() * ray_len;
                    let ry = cy + angle.sin() * ray_len;
                    draw_glow_circle(buf, w, h, rx, ry, 4.0 * sf, color, ray_alpha);
                }
            }
        }
        "ring" => {
            let radius = (8.0 + p * 50.0) * sf;
            let alpha = (1.0 - p) * 0.8;
            // Outer expanding ring
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha * 0.3);
            draw_glow_circle(
                buf,
                w,
                h,
                cx,
                cy,
                (radius - 3.0 * sf).max(1.0),
                color,
                alpha,
            );
            // Inner contracting ring
            if p < 0.5 {
                let inner_t = p / 0.5;
                let inner_r = (30.0 * (1.0 - inner_t)) * sf;
                let inner_alpha = (1.0 - inner_t) * 0.7;
                draw_glow_circle(buf, w, h, cx, cy, inner_r, color, inner_alpha * 0.3);
                draw_glow_circle(
                    buf,
                    w,
                    h,
                    cx,
                    cy,
                    (inner_r - 2.0 * sf).max(1.0),
                    color,
                    inner_alpha,
                );
            }
            // Center dot
            let dot_size = (6.0 * (1.0 - p)) * sf;
            if dot_size > 0.5 {
                draw_glow_circle(buf, w, h, cx, cy, dot_size, color, (1.0 - p) * 0.9);
                draw_glow_circle(
                    buf,
                    w,
                    h,
                    cx,
                    cy,
                    dot_size * 0.5,
                    (255, 255, 255),
                    (1.0 - p) * 0.8,
                );
            }
        }
        "pulse" => {
            for i in 0..5 {
                let phase = (p * 1.5 + i as f32 * 0.18) % 1.0;
                let radius = phase * 45.0 * sf;
                let alpha = (1.0 - phase) * 0.6;
                draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha * 0.3);
                draw_glow_circle(
                    buf,
                    w,
                    h,
                    cx,
                    cy,
                    (radius - 2.0 * sf).max(1.0),
                    color,
                    alpha,
                );
            }
            // Breathing center glow
            let breathe = (p * std::f32::consts::PI * 8.0).sin() * 0.3 + 0.7;
            let glow_size = (10.0 + breathe * 6.0) * sf;
            draw_glow_circle(
                buf,
                w,
                h,
                cx,
                cy,
                glow_size,
                color,
                breathe * (1.0 - p) * 0.8,
            );
        }
        "confetti" => {
            // Confetti is handled separately via precompute_confetti
        }
        _ => {
            // Default: visible expanding circle
            let radius = (8.0 + p * 50.0) * sf;
            let alpha = (1.0 - p) * 0.6;
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
            draw_glow_circle(
                buf,
                w,
                h,
                cx,
                cy,
                radius * 0.4,
                (255, 255, 255),
                alpha * 0.5,
            );
        }
    }
}

/// Rotate RGB color by hue degrees (simple approximation)
fn hue_rotate_rgb(color: (u8, u8, u8), degrees: f32) -> (u8, u8, u8) {
    let (r, g, b) = (color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0);
    let angle = degrees * std::f32::consts::PI / 180.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Rotation matrix for hue
    let nr = r * (0.213 + 0.787 * cos_a - 0.213 * sin_a)
           + g * (0.715 - 0.715 * cos_a - 0.715 * sin_a)
           + b * (0.072 - 0.072 * cos_a + 0.928 * sin_a);
    let ng = r * (0.213 - 0.213 * cos_a + 0.143 * sin_a)
           + g * (0.715 + 0.285 * cos_a + 0.140 * sin_a)
           + b * (0.072 - 0.072 * cos_a - 0.283 * sin_a);
    let nb = r * (0.213 - 0.213 * cos_a - 0.787 * sin_a)
           + g * (0.715 - 0.715 * cos_a + 0.715 * sin_a)
           + b * (0.072 + 0.928 * cos_a + 0.072 * sin_a);

    (
        (nr.clamp(0.0, 1.0) * 255.0) as u8,
        (ng.clamp(0.0, 1.0) * 255.0) as u8,
        (nb.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

// ═══ CONFETTI CLICK EFFECT ═══
// Post-processing particle system: deterministic, pre-simulated, parallel-rendered.

/// Splitmix64 — deterministic pseudo-random generator (no external crate needed)
fn splitmix64(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Get a deterministic f32 in [0, 1) from the RNG
fn rand_f32(seed: &mut u64) -> f32 {
    (splitmix64(seed) >> 11) as f32 / (1u64 << 53) as f32
}

/// Pre-simulate confetti particles for ALL frames.
/// Returns Vec of length total_frames, each containing the confetti particles alive at that frame.
/// This runs sequentially (confetti is a persistent particle system), but the results are used
/// in the parallel frame render.
fn precompute_confetti(
    click_events: &[ClickEvent],
    total_frames: u32,
    fps: u32,
    click_effect: &str,
    primary_color: (u8, u8, u8),
    _secondary_color: (u8, u8, u8),
    region: Option<&crate::region::CaptureRegion>,
) -> Vec<Vec<ConfettiState>> {
    if click_effect != "confetti" {
        return vec![Vec::new(); total_frames as usize];
    }

    let ms_per_frame = 1000.0 / fps as f64;
    let mut result = vec![Vec::new(); total_frames as usize];

    // Color palette: primary click color + 6 rainbow colors (matching canvas version)
    let rainbow: [(u8, u8, u8); 6] = [
        (255, 68, 85),  // red
        (68, 136, 255), // blue
        (255, 204, 34), // yellow
        (255, 68, 204), // pink
        (68, 255, 204), // cyan
        (255, 136, 68), // orange
    ];

    for click in click_events {
        // Deterministic seed from click timestamp
        let seed_base = (click.timestamp_ms as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add(0xBEEF);
        let click_frame = (click.timestamp_ms as f64 / ms_per_frame).round() as i64;

        // Spawn 30 particles (matching canvas version)
        for i in 0..30u32 {
            let mut seed = seed_base.wrapping_add(i as u64 * 12345);

            let angle = rand_f32(&mut seed) * std::f32::consts::PI * 2.0;
            let speed = 2.5 + rand_f32(&mut seed) * 5.0;
            let mut vx = angle.cos() * speed;
            let mut vy = angle.sin() * speed - 2.5; // initial upward bias
            let size = 2.0 + rand_f32(&mut seed) * 4.0;
            let rot_speed = (rand_f32(&mut seed) - 0.5) * 0.25;
            let color_idx = (rand_f32(&mut seed) * 7.0) as usize;
            let pcolor = if color_idx == 0 {
                primary_color
            } else {
                rainbow[(color_idx - 1).min(5)]
            };
            let pshape = (rand_f32(&mut seed) * 3.0) as u8; // 0=rect, 1=circle, 2=triangle
            let max_age = 45 + (rand_f32(&mut seed) * 25.0) as u32;

            let mut px = if let Some(r) = region {
                click.x - r.x as f32
            } else {
                click.x
            };
            let mut py = if let Some(r) = region {
                click.y - r.y as f32
            } else {
                click.y
            };
            let mut rotation = 0.0f32;

            // Simulate this particle's lifetime frame by frame
            for frame_offset in 0..max_age {
                let global_frame = click_frame + frame_offset as i64;
                if global_frame < 0 || global_frame >= total_frames as i64 {
                    // Still advance physics even if out of bounds
                    px += vx;
                    py += vy;
                    vy += 0.13;
                    vx *= 0.985;
                    rotation += rot_speed;
                    continue;
                }

                let alpha = (1.0 - frame_offset as f32 / max_age as f32) * 0.9;
                result[global_frame as usize].push(ConfettiState {
                    x: px,
                    y: py,
                    size,
                    color: pcolor,
                    rotation,
                    shape: pshape,
                    alpha,
                });

                // Physics step (matching canvas: gravity=0.13, drag=0.985)
                px += vx;
                py += vy;
                vy += 0.13;
                vx *= 0.985;
                rotation += rot_speed;
            }
        }
    }

    result
}

/// Render a single confetti particle onto the RGBA frame buffer
fn render_confetti_particle(buf: &mut [u8], w: u32, h: u32, p: &ConfettiState) {
    if p.alpha < 0.01 || p.size < 0.5 {
        return;
    }

    match p.shape {
        1 => {
            // Circle — reuse existing glow circle
            draw_glow_circle(buf, w, h, p.x, p.y, p.size * 0.5, p.color, p.alpha);
        }
        2 => {
            // Triangle
            let half = p.size * 0.5;
            let cos_r = p.rotation.cos();
            let sin_r = p.rotation.sin();
            // Vertices: top, bottom-right, bottom-left (rotated)
            let v0 = rotate_point(0.0, -half, cos_r, sin_r);
            let v1 = rotate_point(half * 0.866, half * 0.5, cos_r, sin_r);
            let v2 = rotate_point(-half * 0.866, half * 0.5, cos_r, sin_r);

            let x1 = p.x + v0.0;
            let y1 = p.y + v0.1;
            let x2 = p.x + v1.0;
            let y2 = p.y + v1.1;
            let x3 = p.x + v2.0;
            let y3 = p.y + v2.1;

            // Bounding box
            let min_x = x1.min(x2).min(x3).max(0.0) as u32;
            let max_x = (x1.max(x2).max(x3) + 1.0).min(w as f32 - 1.0) as u32;
            let min_y = y1.min(y2).min(y3).max(0.0) as u32;
            let max_y = (y1.max(y2).max(y3) + 1.0).min(h as f32 - 1.0) as u32;

            for py in min_y..=max_y {
                for px in min_x..=max_x {
                    if point_in_triangle(px as f32, py as f32, x1, y1, x2, y2, x3, y3) {
                        let idx = ((py * w + px) * 4) as usize;
                        if idx + 3 < buf.len() {
                            blend_pixel(&mut buf[idx..idx + 4], p.color, p.alpha);
                        }
                    }
                }
            }
        }
        _ => {
            // Rectangle (rotated)
            let half_w = p.size * 0.5;
            let half_h = p.size * 0.25;
            let cos_r = p.rotation.cos();
            let sin_r = p.rotation.sin();
            let max_r = (half_w + half_h) as i32 + 1;
            let cx_i = p.x as i32;
            let cy_i = p.y as i32;

            for py in (cy_i - max_r).max(0)..=(cy_i + max_r).min(h as i32 - 1) {
                for px in (cx_i - max_r).max(0)..=(cx_i + max_r).min(w as i32 - 1) {
                    let dx = px as f32 - p.x;
                    let dy = py as f32 - p.y;
                    // Rotate point to rect's local space
                    let rx = dx * cos_r + dy * sin_r;
                    let ry = -dx * sin_r + dy * cos_r;
                    if rx.abs() <= half_w && ry.abs() <= half_h {
                        let idx = ((py as u32 * w + px as u32) * 4) as usize;
                        if idx + 3 < buf.len() {
                            blend_pixel(&mut buf[idx..idx + 4], p.color, p.alpha);
                        }
                    }
                }
            }
        }
    }
}

/// Rotate a point by angle (given precomputed cos/sin)
fn rotate_point(x: f32, y: f32, cos_a: f32, sin_a: f32) -> (f32, f32) {
    (x * cos_a - y * sin_a, x * sin_a + y * cos_a)
}

/// Point-in-triangle test using barycentric sign method
fn point_in_triangle(
    px: f32,
    py: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
) -> bool {
    let d1 = sign(px, py, x1, y1, x2, y2);
    let d2 = sign(px, py, x2, y2, x3, y3);
    let d3 = sign(px, py, x3, y3, x1, y1);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2)
}

/// Alpha-blend a color onto a pixel in the RGBA buffer
fn blend_pixel(pixel: &mut [u8], color: (u8, u8, u8), alpha: f32) {
    let src_a = alpha.max(0.0).min(1.0);
    if src_a < 0.01 {
        return;
    }
    let dst_a = pixel[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a > 0.0 {
        pixel[0] =
            ((color.0 as f32 * src_a + pixel[0] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        pixel[1] =
            ((color.1 as f32 * src_a + pixel[1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        pixel[2] =
            ((color.2 as f32 * src_a + pixel[2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        pixel[3] = (out_a * 255.0) as u8;
    }
}

#[derive(Debug, Clone)]
struct BakedBubble {
    text: String,
    start_time_ms: u64,
    end_time_ms: u64,
}

fn parse_keyboard_bubbles(events: &[KeyboardEvent], bubble_timeout_ms: u64, _max_bubbles: usize) -> Vec<BakedBubble> {
    let mut bubbles: Vec<BakedBubble> = Vec::new();
    let mut active_text: Option<(String, u64, u64)> = None;

    let is_modifier = |k: &str| -> bool {
        ["Ctrl", "Shift", "Alt", "Win"].contains(&k)
    };
    let is_delimiter = |k: &str| -> bool {
        ["Enter", "Tab", "Esc", "NumEnter"].contains(&k)
    };

    let flush_active = |active: &mut Option<(String, u64, u64)>, bubbles: &mut Vec<BakedBubble>| {
        if let Some((text, start, last)) = active.take() {
            bubbles.push(BakedBubble {
                text,
                start_time_ms: start,
                end_time_ms: last + bubble_timeout_ms,
            });
        }
    };

    let mut last_event_time = 0;

    for ev in events {
        let key = &ev.key;
        let t = ev.timestamp_ms;

        // Skip "Activity" events (they are only for auto-zoom typing detection)
        if key == "Activity" {
            continue;
        }

        if active_text.is_some() && last_event_time > 0 && t - last_event_time > 1200 {
            flush_active(&mut active_text, &mut bubbles);
        }
        last_event_time = t;

        if key == "Space" {
            flush_active(&mut active_text, &mut bubbles);
            continue;
        }

        if is_delimiter(key) {
            flush_active(&mut active_text, &mut bubbles);
            bubbles.push(BakedBubble {
                text: key.to_string(),
                start_time_ms: t,
                end_time_ms: t + bubble_timeout_ms,
            });
            continue;
        }

        if key == "Backspace" {
            if let Some(ref mut act) = active_text {
                if !act.0.is_empty() {
                    act.0.pop();
                    act.2 = t;
                    if act.0.is_empty() {
                        active_text = None;
                    }
                }
            }
            continue;
        }

        if is_modifier(key) || key.contains(" + ") {
            flush_active(&mut active_text, &mut bubbles);
            bubbles.push(BakedBubble {
                text: key.to_string(),
                start_time_ms: t,
                end_time_ms: t + bubble_timeout_ms,
            });
            continue;
        }

        let char_str = if key.len() == 1 {
            key.to_lowercase()
        } else {
            key.to_string()
        };

        if let Some(ref mut act) = active_text {
            act.0.push_str(&char_str);
            act.2 = t;
        } else {
            active_text = Some((char_str, t, t));
        }
    }

    flush_active(&mut active_text, &mut bubbles);
    bubbles
}

fn get_active_bubbles_at_time<'a>(bubbles: &'a [BakedBubble], time_ms: u64, max_bubbles: usize) -> Vec<&'a BakedBubble> {
    let mut active: Vec<&BakedBubble> = bubbles
        .iter()
        .filter(|b| time_ms >= b.start_time_ms && time_ms < b.end_time_ms)
        .collect();

    active.sort_by_key(|b| b.start_time_ms);

    if active.len() > max_bubbles {
        let skip = active.len() - max_bubbles;
        active.drain(0..skip);
    }

    active
}

fn parse_color_string(color_str: &str) -> ((u8, u8, u8), f32) {
    let clean = color_str.trim();
    if clean.starts_with('#') {
        let rgb = parse_hex_color(clean);
        (rgb, 1.0)
    } else if clean.starts_with("rgba") {
        let content = clean
            .trim_start_matches("rgba(")
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<&str>>();
        if content.len() >= 4 {
            let r = content[0].parse::<u8>().unwrap_or(0);
            let g = content[1].parse::<u8>().unwrap_or(0);
            let b = content[2].parse::<u8>().unwrap_or(0);
            let a = content[3].parse::<f32>().unwrap_or(1.0);
            ((r, g, b), a)
        } else {
            ((20, 20, 20), 0.75)
        }
    } else if clean.starts_with("rgb") {
        let content = clean
            .trim_start_matches("rgb(")
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<&str>>();
        if content.len() >= 3 {
            let r = content[0].parse::<u8>().unwrap_or(0);
            let g = content[1].parse::<u8>().unwrap_or(0);
            let b = content[2].parse::<u8>().unwrap_or(0);
            ((r, g, b), 1.0)
        } else {
            ((20, 20, 20), 1.0)
        }
    } else {
        ((20, 20, 20), 0.75)
    }
}

fn draw_rect(buf: &mut [u8], w: u32, h: u32, rx: u32, ry: u32, rw: u32, rh: u32, color: (u8, u8, u8), alpha: f32) {
    for y in ry..(ry + rh) {
        if y >= h { continue; }
        for x in rx..(rx + rw) {
            if x >= w { continue; }
            let idx = ((y * w + x) * 4) as usize;
            if idx + 3 < buf.len() {
                blend_pixel(&mut buf[idx..idx + 4], color, alpha);
            }
        }
    }
}

fn draw_border(
    buf: &mut [u8],
    w: u32,
    h: u32,
    rx: u32,
    ry: u32,
    rw: u32,
    rh: u32,
    color: (u8, u8, u8),
    alpha: f32,
) {
    for x in rx..(rx + rw) {
        if x >= w { continue; }
        // top
        let idx_t = ((ry * w + x) * 4) as usize;
        if idx_t + 3 < buf.len() {
            blend_pixel(&mut buf[idx_t..idx_t + 4], color, alpha);
        }
        // bottom
        let by = ry + rh - 1;
        if by < h {
            let idx_b = ((by * w + x) * 4) as usize;
            if idx_b + 3 < buf.len() {
                blend_pixel(&mut buf[idx_b..idx_b + 4], color, alpha);
            }
        }
    }
    for y in ry..(ry + rh) {
        if y >= h { continue; }
        // left
        let idx_l = ((y * w + rx) * 4) as usize;
        if idx_l + 3 < buf.len() {
            blend_pixel(&mut buf[idx_l..idx_l + 4], color, alpha);
        }
        // right
        let rx_r = rx + rw - 1;
        if rx_r < w {
            let idx_r = ((y * w + rx_r) * 4) as usize;
            if idx_r + 3 < buf.len() {
                blend_pixel(&mut buf[idx_r..idx_r + 4], color, alpha);
            }
        }
    }
}

fn draw_text(
    buf: &mut [u8],
    w: u32,
    h: u32,
    text: &str,
    x: u32,
    y: u32,
    scale: u32,
    color: (u8, u8, u8),
    alpha: f32,
) {
    use font8x8::UnicodeFonts;
    let mut current_x = x;
    for c in text.chars() {
        if let Some(glyph) = font8x8::BASIC_FONTS.get(c) {
            for gy in 0..8 {
                let byte = glyph[gy];
                for gx in 0..8 {
                    if byte & (1 << gx) != 0 {
                        for dy in 0..scale {
                            let py = y + gy as u32 * scale + dy;
                            if py >= h { continue; }
                            for dx in 0..scale {
                                let px = current_x + gx as u32 * scale + dx;
                                if px >= w { continue; }
                                let idx = ((py * w + px) * 4) as usize;
                                if idx + 3 < buf.len() {
                                    blend_pixel(&mut buf[idx..idx + 4], color, alpha);
                                }
                            }
                        }
                    }
                }
            }
        }
        current_x += 8 * scale + scale;
    }
}

fn render_keyboard_overlay(
    buf: &mut [u8],
    w: u32,
    h: u32,
    active_bubbles: &[&BakedBubble],
    config: &crate::config::AppConfig,
    scale_factor: f64,
    region: Option<&crate::region::CaptureRegion>,
) {
    let scale = (config.keyboard_overlay_font_size as f64 * scale_factor / 8.0).round().max(1.0) as u32;
    let padding_x = 8 * scale;
    let padding_y = 6 * scale;
    let char_w = 8 * scale + scale;
    let gap = 10 * scale;

    let physical_ox = config.keyboard_overlay_x as f64 * scale_factor;
    let physical_oy = config.keyboard_overlay_y as f64 * scale_factor;

    let (ox, oy) = if let Some(r) = region {
        (physical_ox - r.x as f64, physical_oy - r.y as f64)
    } else {
        (physical_ox, physical_oy)
    };

    let mut total_width = 0;
    for (i, b) in active_bubbles.iter().enumerate() {
        let text_w = b.text.chars().count() as u32 * char_w;
        let bubble_w = text_w + padding_x * 2;
        total_width += bubble_w;
        if i > 0 {
            total_width += gap;
        }
    }

    let container_w = (config.keyboard_overlay_width as f64 * scale_factor) as u32;
    let mut start_x = if total_width < container_w {
        ox as i32 + (container_w as i32 - total_width as i32) / 2
    } else {
        ox as i32
    };

    let start_y = oy as i32;

    let (bg_color, bg_alpha) = parse_color_string(&config.keyboard_overlay_background_color);
    let (border_color, border_alpha) = parse_color_string(&config.keyboard_overlay_border_color);
    let (text_color, _text_alpha) = parse_color_string(&config.keyboard_overlay_text_color);

    let final_bg_alpha = bg_alpha * config.keyboard_overlay_opacity;
    let final_border_alpha = border_alpha * config.keyboard_overlay_opacity;

    for b in active_bubbles {
        let text_w = b.text.chars().count() as u32 * char_w;
        let bubble_w = text_w + padding_x * 2;
        let bubble_h = 8 * scale + padding_y * 2;

        if start_x >= 0 && start_y >= 0 {
            draw_rect(
                buf,
                w,
                h,
                start_x as u32,
                start_y as u32,
                bubble_w,
                bubble_h,
                bg_color,
                final_bg_alpha,
            );

            if config.keyboard_overlay_border_width > 0 {
                draw_border(
                    buf,
                    w,
                    h,
                    start_x as u32,
                    start_y as u32,
                    bubble_w,
                    bubble_h,
                    border_color,
                    final_border_alpha,
                );
            }

            draw_text(
                buf,
                w,
                h,
                &b.text,
                (start_x as u32) + padding_x,
                (start_y as u32) + padding_y,
                scale,
                text_color,
                config.keyboard_overlay_opacity,
            );
        }

        start_x += (bubble_w + gap) as i32;
    }
}


