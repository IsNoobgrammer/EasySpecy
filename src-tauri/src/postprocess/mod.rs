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

/// Collected cursor + click metadata for a recording session
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingMetadata {
    pub cursor_trail: Vec<CursorSample>,
    pub click_events: Vec<ClickEvent>,
    pub trail_style: String,
    pub click_effect: String,
    pub trail_color: String,
}

static METADATA: Mutex<Option<RecordingMetadata>> = Mutex::new(None);
static SESSION_START: Mutex<Option<Instant>> = Mutex::new(None);

/// Start collecting cursor metadata for a new recording session
pub fn start_collection() {
    let config = crate::config::AppConfig::load();
    *METADATA.lock().unwrap() = Some(RecordingMetadata {
        trail_style: config.trail_style.clone(),
        click_effect: config.click_effect.clone(),
        trail_color: config.cursor_trail_color.clone(),
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
    }
    tracing::info!("Cursor session start reset to CAPTURE_ARMED instant");
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
pub fn apply_effects(input: &str, _output: &str, meta: &RecordingMetadata) -> Result<String, String> {
    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;

    // Skip if no effects enabled
    if meta.trail_style == "none" && meta.click_effect == "none" {
        return Ok(input.to_string());
    }
    if meta.cursor_trail.is_empty() && meta.click_events.is_empty() {
        return Ok(input.to_string());
    }

    tracing::info!(
        "Applying effects: trail={}, click={}, {} samples, {} clicks",
        meta.trail_style, meta.click_effect,
        meta.cursor_trail.len(), meta.click_events.len()
    );

    let config = crate::config::AppConfig::load();
    let fps = config.fps;

    // Video is captured at MONITOR resolution, not config resolution
    // Get actual monitor size for the trail frame buffer
    let (width, height) = {
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

    if total_frames == 0 {
        return Ok(input.to_string());
    }

    // Create temp dir for trail frames
    let trail_dir = std::env::temp_dir().join("easyspecy_trail");
    let _ = std::fs::remove_dir_all(&trail_dir);
    std::fs::create_dir_all(&trail_dir).map_err(|e| e.to_string())?;

    tracing::info!("Rendering {} trail frames at {}x{} {}fps", total_frames, width, height, fps);

    // Parse trail color
    let color = parse_hex_color(&meta.trail_color);

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
    let trail_duration_ms: f64 = 400.0; // Comet tail length in ms

    use std::io::Write;

    // ═══ PRE-COMPUTE PER-FRAME DATA (PARALLEL) ═══
    // Each frame's trail segment + click data is independent → parallel compute.
    let smooth_path_ref = &smooth_path;
    let click_ref = &meta.click_events;
    let pre_computed: Vec<(Vec<(f32, f32, f64)>, Vec<(f32, f32, f64)>)> = (0..total_frames)
        .into_par_iter()
        .map(|frame_idx| {
            let frame_time_ms = frame_idx as f64 * ms_per_frame;
            let head_idx = find_path_index_at_time(smooth_path_ref, frame_time_ms);

            // Trail segment
            let segment: Vec<(f32, f32, f64)> = if head_idx > 0 {
                let tail_start_time = (frame_time_ms - trail_duration_ms).max(0.0);
                let tail_idx = find_path_index_at_time(smooth_path_ref, tail_start_time);
                if head_idx > tail_idx {
                    (tail_idx..=head_idx)
                        .map(|i| {
                            let p = &smooth_path_ref[i];
                            let age = 1.0 - ((i - tail_idx) as f64 / (head_idx - tail_idx) as f64);
                            (p.0, p.1, age)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // Active clicks — binary search since click_events are sorted by timestamp
            let active_clicks: Vec<(f32, f32, f64)> = click_ref
                .iter()
                .filter_map(|click| {
                    let click_age_ms = frame_time_ms - click.timestamp_ms as f64;
                    if click_age_ms >= 0.0 && click_age_ms < 600.0 {
                        Some((click.x, click.y, click_age_ms / 600.0))
                    } else {
                        None
                    }
                })
                .collect();

            (segment, active_clicks)
        })
        .collect();

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
                        &meta.trail_style,
                    );
                }

                // Click effects
                for &(cx, cy, progress) in active_clicks {
                    render_click_effect(
                        &mut frame_buf,
                        width,
                        height,
                        cx,
                        cy,
                        progress,
                        color,
                        &meta.click_effect,
                    );
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
        tracing::warn!("FFmpeg effects failed: {}", stderr);
        let _ = std::fs::remove_dir_all(&trail_dir);
        return Ok(input.to_string());
    }

    let _ = std::fs::remove_dir_all(&trail_dir);
    tracing::info!("Trail effects applied: {} frames rendered", total_frames);
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

/// Render smooth cursor trail onto RGBA frame buffer
fn render_smooth_trail(
    buf: &mut [u8],
    w: u32, h: u32,
    points: &[(f32, f32, f64)], // (x, y, age 0..1)
    color: (u8, u8, u8),
    style: &str,
) {
    if points.len() < 2 { return; }

    // Interpolate between points using Catmull-Rom for smoothness
    let mut interpolated: Vec<(f32, f32, f64)> = Vec::with_capacity(points.len() * 4);

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
            interpolated.push((x, y, age));
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
            for &(x, y, age) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                // Outer glow
                draw_glow_circle(buf, w, h, x, y, 12.0 * life, color, 0.04 * life);
                draw_glow_circle(buf, w, h, x, y, 7.0 * life, color, 0.12 * life);
                // Core
                draw_glow_circle(buf, w, h, x, y, 3.5 * life, color, 0.6 * life);
                // White center
                draw_glow_circle(buf, w, h, x, y, 1.5 * life, (255, 255, 255), 0.5 * life);
            }
            // Extra bright head glow at newest point
            if let Some(&(x, y, _)) = interpolated.last() {
                draw_glow_circle(buf, w, h, x, y, 16.0, color, 0.08);
                draw_glow_circle(buf, w, h, x, y, 9.0, color, 0.2);
                draw_glow_circle(buf, w, h, x, y, 4.0, color, 0.8);
                draw_glow_circle(buf, w, h, x, y, 2.0, (255, 255, 255), 0.9);
            }
        }
        "particles" | "dots" => {
            for &(x, y, age) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                let size = 3.0 + life * 4.0;
                draw_glow_circle(buf, w, h, x, y, size, color, 0.7 * life);
                draw_glow_circle(buf, w, h, x, y, size * 0.4, (255, 255, 255), 0.5 * life);
            }
        }
        "ribbon" => {
            for &(x, y, age) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                // Wider, more diffuse
                draw_glow_circle(buf, w, h, x, y, 8.0 * life, color, 0.15 * life);
                draw_glow_circle(buf, w, h, x, y, 4.0 * life, color, 0.4 * life);
                draw_glow_circle(buf, w, h, x, y, 1.5 * life, (255, 255, 255), 0.3 * life);
            }
        }
        "aurora" => {
            for (i, &(x, y, age)) in interpolated.iter().enumerate() {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                // Shift hue along trail
                let hue_shift = (i as f32 * 3.0) % 360.0;
                let shifted_color = hue_rotate_rgb(color, hue_shift);
                draw_glow_circle(buf, w, h, x, y, 10.0 * life, shifted_color, 0.08 * life);
                draw_glow_circle(buf, w, h, x, y, 5.0 * life, shifted_color, 0.2 * life);
                draw_glow_circle(buf, w, h, x, y, 2.0 * life, (255, 255, 255), 0.3 * life);
            }
        }
        _ => {
            // Default: simple dots
            for &(x, y, age) in &interpolated {
                let life = (1.0 - age as f32).max(0.0);
                if life <= 0.0 { continue; }
                draw_glow_circle(buf, w, h, x, y, 4.0 * life, color, 0.6 * life);
            }
        }
    }
}

/// Render click effect onto RGBA frame buffer
fn render_click_effect(
    buf: &mut [u8],
    w: u32, h: u32,
    cx: f32, cy: f32,
    progress: f64, // 0.0 = just clicked, 1.0 = fully faded
    color: (u8, u8, u8),
    style: &str,
) {
    let p = progress as f32;
    if p >= 1.0 { return; }

    match style {
        "ripple" => {
            // Expanding rings
            for ring in 0..3 {
                let ring_p = (p - ring as f32 * 0.12).max(0.0) / 0.7;
                if ring_p >= 1.0 || ring_p <= 0.0 { continue; }
                let radius = ring_p * 40.0;
                let alpha = (1.0 - ring_p) * 0.5;
                draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
            }
            // Center flash
            if p < 0.15 {
                let flash = 1.0 - p / 0.15;
                draw_glow_circle(buf, w, h, cx, cy, 5.0, (255, 255, 255), flash * 0.8);
            }
        }
        "spotlight" => {
            let radius = 6.0 + p * 35.0;
            let alpha = (1.0 - p) * 0.4;
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
            draw_glow_circle(buf, w, h, cx, cy, radius * 0.3, (255, 255, 255), alpha * 0.5);
        }
        "ring" => {
            let radius = 4.0 + p * 30.0;
            let alpha = (1.0 - p) * 0.6;
            // Ring = outer circle minus inner
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha * 0.3);
            draw_glow_circle(buf, w, h, cx, cy, 3.0 * (1.0 - p), color, (1.0 - p) * 0.8);
        }
        "pulse" => {
            for i in 0..4 {
                let phase = (p * 1.5 + i as f32 * 0.18) % 1.0;
                let radius = phase * 25.0;
                let alpha = (1.0 - phase) * 0.4;
                draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
            }
        }
        _ => {
            // Default ripple
            let radius = p * 35.0;
            let alpha = (1.0 - p) * 0.5;
            draw_glow_circle(buf, w, h, cx, cy, radius, color, alpha);
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

    ((nr.clamp(0.0, 1.0) * 255.0) as u8,
     (ng.clamp(0.0, 1.0) * 255.0) as u8,
     (nb.clamp(0.0, 1.0) * 255.0) as u8)
}
