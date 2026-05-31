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
    pub button: String, // "left", "right", "middle"
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
            // Sample at ~60Hz max (skip if <16ms since last)
            if m.cursor_trail.last().map_or(true, |s| ms - s.timestamp_ms >= 16) {
                m.cursor_trail.push(CursorSample {
                    timestamp_ms: ms,
                    x,
                    y,
                });
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
            m.click_events.push(ClickEvent {
                timestamp_ms: ms,
                x,
                y,
                button: button.to_string(),
            });
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
