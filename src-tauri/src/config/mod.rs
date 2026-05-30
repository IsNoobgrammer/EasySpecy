use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration — persisted as TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    // Output
    pub output_dir: String,

    // Video
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub fps: u32,

    // Audio
    pub audio_enabled: bool,
    pub audio_sample_rate: u32,
    pub audio_device: String,

    // Webcam
    pub webcam_enabled: bool,
    pub webcam_device: String,
    pub webcam_position: WebcamPosition,
    pub webcam_size: u32,

    // Auto-zoom
    pub auto_zoom_enabled: bool,
    pub zoom_level: f32,
    pub zoom_dwell_ms: u32,

    // Cursor effects
    pub cursor_trail_enabled: bool,
    pub cursor_trail_color: String,
    pub cursor_trail_size: f32,
    pub cursor_smoothing: bool,
    pub cursor_size_multiplier: f32,

    // Hotkeys
    pub hotkey_start: String,
    pub hotkey_stop: String,
    pub hotkey_pause: String,

    // General
    pub minimize_to_tray: bool,
    pub copy_path_on_save: bool,
    pub recording_mode: RecordingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebcamPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordingMode {
    FullScreen,
    Region,
    Window,
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = dirs::video_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."))
            .join("EasySpecy")
            .to_string_lossy()
            .to_string();

        Self {
            output_dir,

            resolution_width: 1920,
            resolution_height: 1080,
            fps: 30,

            audio_enabled: true,
            audio_sample_rate: 44100,
            audio_device: "default".to_string(),

            webcam_enabled: false,
            webcam_device: "default".to_string(),
            webcam_position: WebcamPosition::BottomRight,
            webcam_size: 200,

            auto_zoom_enabled: false,
            zoom_level: 2.0,
            zoom_dwell_ms: 1500,

            cursor_trail_enabled: false,
            cursor_trail_color: "#00ff88".to_string(),
            cursor_trail_size: 8.0,
            cursor_smoothing: true,
            cursor_size_multiplier: 1.5,

            hotkey_start: "Ctrl+Shift+R".to_string(),
            hotkey_stop: "Ctrl+Shift+S".to_string(),
            hotkey_pause: "Ctrl+Shift+P".to_string(),

            minimize_to_tray: true,
            copy_path_on_save: true,
            recording_mode: RecordingMode::FullScreen,
        }
    }
}

impl AppConfig {
    /// Path to the config file: ~/.config/easyspecy/config.toml
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("easyspecy").join("config.toml")
    }

    /// Load config from disk, or return defaults if file doesn't exist
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let config = Self::default();
            let _ = config.save(); // write defaults on first run
            config
        }
    }

    /// Save config to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
