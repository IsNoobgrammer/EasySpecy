//! Post-processing module — FFmpeg encoding, auto-zoom, cursor trail filters
//!
//! Phase 2: Cursor trail + click metadata collection during recording.
//! On stop, metadata is fed to FFmpeg filter chains for baked-in effects.

use serde::{Deserialize, Serialize};
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

/// Record a cursor position sample
pub fn record_cursor(x: f32, y: f32) {
    let start = SESSION_START.lock().unwrap();
    if let Some(t) = *start {
        let ms = t.elapsed().as_millis() as u64;
        let mut meta = METADATA.lock().unwrap();
        if let Some(ref mut m) = *meta {
            if m.cursor_trail.last().map_or(true, |s| ms - s.timestamp_ms >= 16) {
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
/// Reads the metadata and generates drawbox filter chains.
/// Returns the path to the effects-applied video (or input if no effects).
pub fn apply_effects(input: &str, output: &str, meta: &RecordingMetadata) -> Result<String, String> {
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

    // Parse trail color (hex -> FFmpeg color)
    let color = hex_to_ffmpeg_color(&meta.trail_color);

    // Build FFmpeg filter chain
    let mut filters: Vec<String> = Vec::new();

    // ── Cursor trail ──
    if meta.trail_style != "none" && !meta.cursor_trail.is_empty() {
        let trail_filters = build_trail_filters(meta, &color);
        filters.extend(trail_filters);
    }

    // ── Click effects ──
    if meta.click_effect != "none" && !meta.click_events.is_empty() {
        let click_filters = build_click_filters(meta, &color);
        filters.extend(click_filters);
    }

    if filters.is_empty() {
        return Ok(input.to_string());
    }

    // Join all filters
    let filter_str = filters.join(",");
    tracing::info!("FFmpeg filter chain: {} filters", filters.len());

    // Apply filters with FFmpeg
    let effects_path = input.replace(".mp4", "_effects.mp4");

    let mut cmd = std::process::Command::new(&ffmpeg);
    cmd.args([
        "-y",
        "-i", input,
        "-vf", &filter_str,
        "-c:v", "libx264",
        "-preset", "fast",
        "-crf", "18",
        "-c:a", "copy",
        &effects_path,
    ])
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let result = cmd.output().map_err(|e| format!("FFmpeg effects error: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        tracing::warn!("FFmpeg effects failed, keeping original: {}", stderr);
        return Ok(input.to_string());
    }

    tracing::info!("Effects applied successfully");
    Ok(effects_path)
}

/// Convert hex color (#RRGGBB) to FFmpeg color format
fn hex_to_ffmpeg_color(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() >= 6 {
        format!("0x{}", &h[..6])
    } else {
        "0x00ff88".to_string()
    }
}

/// Build trail drawbox filters.
/// Downsamples to ~10fps and creates semi-transparent boxes along the cursor path.
fn build_trail_filters(meta: &RecordingMetadata, color: &str) -> Vec<String> {
    let mut filters = Vec::new();
    let samples = &meta.cursor_trail;

    if samples.is_empty() {
        return filters;
    }

    // Downsample to ~10fps (every 100ms)
    let mut last_ms = 0u64;
    let mut downsampled: Vec<&CursorSample> = Vec::new();
    for s in samples {
        if s.timestamp_ms - last_ms >= 100 {
            downsampled.push(s);
            last_ms = s.timestamp_ms;
        }
    }

    // Also keep every Nth sample for smooth trail (max 200 points)
    let step = if downsampled.len() > 200 {
        downsampled.len() / 200
    } else {
        1
    };

    let trail_size: u32 = match meta.trail_style.as_str() {
        "glow" => 8,
        "particles" => 4,
        "ribbon" => 12,
        "dots" => 6,
        "aurora" => 10,
        _ => 6,
    };

    let alpha: f32 = match meta.trail_style.as_str() {
        "glow" => 0.5,
        "particles" => 0.7,
        "ribbon" => 0.3,
        "dots" => 0.6,
        "aurora" => 0.4,
        _ => 0.5,
    };

    for (i, sample) in downsampled.iter().enumerate().step_by(step) {
        let t_start = sample.timestamp_ms as f64 / 1000.0;
        // Each box stays visible for 0.8 seconds
        let t_end = t_start + 0.8;

        let x = sample.x as i32 - (trail_size as i32 / 2);
        let y = sample.y as i32 - (trail_size as i32 / 2);

        let filter = format!(
            "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
            x.max(0), y.max(0), trail_size, trail_size,
            color, alpha, t_start, t_end
        );
        filters.push(filter);
    }

    tracing::info!("Trail: {} drawbox filters from {} downsampled points", filters.len(), downsampled.len());
    filters
}

/// Build click effect drawbox filters.
/// Creates expanding rings/boxes at click positions.
fn build_click_filters(meta: &RecordingMetadata, color: &str) -> Vec<String> {
    let mut filters = Vec::new();

    for click in &meta.click_events {
        let t_click = click.timestamp_ms as f64 / 1000.0;
        let cx = click.x as i32;
        let cy = click.y as i32;

        match meta.click_effect.as_str() {
            "ripple" => {
                // 3 expanding rings
                for ring in 0..3 {
                    let t_start = t_click + ring as f64 * 0.12;
                    let t_end = t_start + 0.6;
                    // Ring expands from 4px to 40px over 0.6s
                    // We approximate with multiple drawbox sizes at different times
                    for step in 0..6 {
                        let t = t_start + step as f64 * 0.1;
                        if t > t_end { break; }
                        let size = 4 + step * 7;
                        let alpha = 0.6 - step as f32 * 0.08;
                        let x = cx - size / 2;
                        let y = cy - size / 2;
                        let t_show = t + 0.1;
                        filters.push(format!(
                            "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
                            x.max(0), y.max(0), size, size, color, alpha.max(0.1), t, t_show
                        ));
                    }
                }
                // Center flash
                filters.push(format!(
                    "drawbox=x={}:y={}:w=6:h=6:color=0xffffff@0.8:t=fill:enable='between(t,{:.3},{:.3})'",
                    cx - 3, cy - 3, t_click, t_click + 0.15
                ));
            }
            "spotlight" => {
                // Large expanding glow
                for step in 0..8 {
                    let t = t_click + step as f64 * 0.06;
                    let size = 6 + step * 6;
                    let alpha = 0.5 - step as f32 * 0.05;
                    let x = cx - size / 2;
                    let y = cy - size / 2;
                    let t_end = t + 0.5;
                    filters.push(format!(
                        "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
                        x.max(0), y.max(0), size, size, color, alpha.max(0.05), t, t_end
                    ));
                }
            }
            "ring" => {
                // Outer expanding ring
                for step in 0..8 {
                    let t = t_click + step as f64 * 0.05;
                    let size = 4 + step * 5;
                    let alpha = 0.7 - step as f32 * 0.07;
                    let x = cx - size / 2;
                    let y = cy - size / 2;
                    let t_end = t + 0.4;
                    filters.push(format!(
                        "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
                        x.max(0), y.max(0), size, size, color, alpha.max(0.1), t, t_end
                    ));
                }
                // Inner contracting ring
                for step in 0..4 {
                    let t = t_click + step as f64 * 0.08;
                    let size = 18 - step * 4;
                    let alpha = 0.5 - step as f32 * 0.1;
                    let x = cx - size.max(2) / 2;
                    let y = cy - size.max(2) / 2;
                    let t_end = t + 0.3;
                    filters.push(format!(
                        "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
                        x.max(0), y.max(0), size.max(2), size.max(2), color, alpha.max(0.1), t, t_end
                    ));
                }
            }
            "pulse" => {
                // Concentric pulsing rings
                for wave in 0..4 {
                    for step in 0..6 {
                        let t = t_click + wave as f64 * 0.18 + step as f64 * 0.04;
                        let size = 4 + step * 5;
                        let alpha = 0.5 - step as f32 * 0.06;
                        let x = cx - size / 2;
                        let y = cy - size / 2;
                        let t_end = t + 0.15;
                        filters.push(format!(
                            "drawbox=x={}:y={}:w={}:h={}:color={}@{}:t=fill:enable='between(t,{:.3},{:.3})'",
                            x.max(0), y.max(0), size, size, color, alpha.max(0.1), t, t_end
                        ));
                    }
                }
            }
            "confetti" => {
                // Scattered boxes around click point
                for i in 0..12 {
                    let angle = (i as f64 / 12.0) * std::f64::consts::TAU;
                    let dist = 8.0 + (i as f64 * 3.0);
                    let px = cx + (angle.cos() * dist) as i32;
                    let py = cy + (angle.sin() * dist) as i32;
                    let t_start = t_click + i as f64 * 0.02;
                    let t_end = t_start + 0.6;
                    let size = 3 + (i % 3) * 2;
                    filters.push(format!(
                        "drawbox=x={}:y={}:w={}:h={}:color={}@0.7:t=fill:enable='between(t,{:.3},{:.3})'",
                        px - size/2, py - size/2, size, size, color, t_start, t_end
                    ));
                }
            }
            _ => {}
        }
    }

    tracing::info!("Clicks: {} drawbox filters for {} events", filters.len(), meta.click_events.len());
    filters
}
