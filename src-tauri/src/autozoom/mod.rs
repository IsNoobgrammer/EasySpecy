//! Auto-Zoom Engine — Intelligent post-processing zoom for screen recordings
//!
//! Analyzes cursor positions, click events, window bounds, and keyboard activity
//! to generate a timeline of zoom regions. Uses spring-physics camera animation
//! for smooth, cinematic transitions.
//!
//! Pipeline integration:
//!   Source Frame → Crop+Scale (zoom) → Cursor Trail → Click Effects → FFmpeg

pub mod camera;
pub mod detection;
pub mod renderer;

use serde::{Deserialize, Serialize};

/// A zoom region — a time span with a target viewport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomRegion {
    pub start_ms: u64,
    pub end_ms: u64,
    pub center_x: f32,
    pub center_y: f32,
    pub zoom_level: f32,
    pub trigger: ZoomTrigger,
    pub priority: u8,
}

/// What triggered this zoom
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZoomTrigger {
    ClickCluster,
    WindowDwell,
    CircleGesture,
    TextSelection,
    Typing,
    Manual,
}

impl ZoomTrigger {
    pub fn priority(&self) -> u8 {
        match self {
            ZoomTrigger::CircleGesture => 5,
            ZoomTrigger::ClickCluster => 4,
            ZoomTrigger::WindowDwell => 3,
            ZoomTrigger::Typing => 2,
            ZoomTrigger::TextSelection => 1,
            ZoomTrigger::Manual => 6,
        }
    }
}

/// Window bounds captured during recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBoundsEvent {
    pub timestamp_ms: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub title: String,
}

/// Keyboard event (timestamp only, no key content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    pub timestamp_ms: u64,
}

/// Auto-zoom configuration (from user settings)
#[derive(Debug, Clone)]
pub struct ZoomConfig {
    pub enabled: bool,
    pub sensitivity: f32,       // 0.0-1.0, scales thresholds
    pub zoom_speed: f32,        // 0.1-2.0, scales spring stiffness
    pub max_zoom: f32,          // maximum zoom level (default 4.0)
    pub min_zoom: f32,          // minimum meaningful zoom (default 1.2)
    pub min_duration_ms: u64,   // minimum zoom duration (default 1500)
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sensitivity: 0.5,
            zoom_speed: 1.0,
            max_zoom: 1.8,          // SUBTLE — never more than 1.8x
            min_zoom: 1.15,         // minimum meaningful zoom
            min_duration_ms: 800,   // shorter minimum — zoom should feel responsive
        }
    }
}

/// The complete zoom timeline for a recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomTimeline {
    pub regions: Vec<ZoomRegion>,
    pub source_width: u32,
    pub source_height: u32,
    pub fps: u32,
}
