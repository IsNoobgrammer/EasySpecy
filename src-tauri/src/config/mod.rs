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
    pub video_encoder: VideoEncoder,
    pub video_bitrate_kbps: u32,
    pub video_quality: VideoQuality,

    // Audio
    pub audio_enabled: bool,
    pub audio_source: AudioSource,
    pub audio_sample_rate: u32,
    pub audio_device: String,

    // Audio Processing
    pub mic_gain: f32,              // Mic volume multiplier (0.0 - 3.0, default 1.0)
    pub system_volume: f32,         // System audio volume in mix (0.0 - 1.0, default 0.55)
    pub noise_gate_threshold: f32,  // Noise gate sensitivity (0.0 = off, 1.0 = aggressive, default 0.5)
    pub noise_reduction: f32,       // Noise reduction strength (0.0 = off, 1.0 = max, default 0.6)

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
    pub cursor_secondary_color: String, // For right-click, gradients
    pub cursor_trail_size: f32,
    pub cursor_smoothing: bool,
    pub cursor_size_multiplier: f32,
    pub cursor_pack: String,
    pub trail_style: String,
    pub click_effect: String,
    pub trail_duration_ms: f64, // 200-2000ms, default 600
    pub cursor_hide_in_recording: bool,
    pub trail_width: f32,

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum VideoEncoder {
    H264,        // libx264 — fast, universal compatibility
    H265,        // libx265/hevc — 50% smaller, slower encode
    AV1,         // libsvtav1 — best compression, 60-70% smaller than H264
    AV1_NVENC,   // av1_nvenc — GPU-accelerated AV1 (Nvidia RTX 40xx)
    H264_NVENC,  // h264_nvenc — GPU-accelerated H264 (any Nvidia GPU)
    H265_NVENC,  // hevc_nvenc — GPU-accelerated H265 (Nvidia GTX 1650+)
    VP9,         // libvpx-vp9 — good compression, web-friendly
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoQuality {
    Low,       // ~2 MB/min @ 1080p30 — max compression, visible artifacts
    Medium,    // ~8 MB/min @ 1080p30 — good balance
    High,      // ~20 MB/min @ 1080p30 — near-lossless
    Ultra,     // ~40 MB/min @ 1080p30 — visually lossless
    Insane,    // AV1 only: ~1 MB/min @ 1080p30 — extreme compression, still watchable
    Custom,    // Use video_bitrate_kbps directly
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioSource {
    Mic,
    System,
    Both,
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
            video_encoder: VideoEncoder::AV1,
            video_bitrate_kbps: 4000,
            video_quality: VideoQuality::Medium,

            audio_enabled: true,
            audio_source: AudioSource::Mic,
            audio_sample_rate: 44100,
            audio_device: "default".to_string(),

            mic_gain: 1.0,
            system_volume: 0.55,
            noise_gate_threshold: 0.5,
            noise_reduction: 0.6,

            webcam_enabled: false,
            webcam_device: "default".to_string(),
            webcam_position: WebcamPosition::BottomRight,
            webcam_size: 200,

            auto_zoom_enabled: false,
            zoom_level: 2.0,
            zoom_dwell_ms: 1500,

            cursor_trail_enabled: false,
            cursor_trail_color: "#00ff88".to_string(),
            cursor_secondary_color: "#ff4488".to_string(),
            cursor_trail_size: 8.0,
            cursor_smoothing: true,
            cursor_size_multiplier: 1.5,
            cursor_pack: "default".to_string(),
            trail_style: "glow".to_string(),
            click_effect: "ripple".to_string(),
            trail_duration_ms: 600.0,
            cursor_hide_in_recording: false,
            trail_width: 1.0,

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

    /// Get the effective bitrate in kbps based on quality preset
    pub fn effective_bitrate_kbps(&self) -> u32 {
        match self.video_quality {
            VideoQuality::Custom => self.video_bitrate_kbps,
            _ => {
                // Base bitrate for 1080p30 — varies by encoder efficiency
                let base = match (&self.video_encoder, &self.video_quality) {
                    // AV1 is ~60-70% more efficient than H264 for screen content
                    (VideoEncoder::AV1, VideoQuality::Insane) => 140,      // ~1 MB/min
                    (VideoEncoder::AV1, VideoQuality::Low) => 400,         // ~3 MB/min
                    (VideoEncoder::AV1, VideoQuality::Medium) => 1500,     // ~5 MB/min
                    (VideoEncoder::AV1, VideoQuality::High) => 4000,       // ~12 MB/min
                    (VideoEncoder::AV1, VideoQuality::Ultra) => 10000,     // ~25 MB/min
                    (VideoEncoder::AV1_NVENC, VideoQuality::Insane) => 200,
                    (VideoEncoder::AV1_NVENC, VideoQuality::Low) => 500,
                    (VideoEncoder::AV1_NVENC, VideoQuality::Medium) => 2000,
                    (VideoEncoder::AV1_NVENC, VideoQuality::High) => 5000,
                    (VideoEncoder::AV1_NVENC, VideoQuality::Ultra) => 12000,
                    // H265 is ~40-50% more efficient than H264
                    (VideoEncoder::H265 | VideoEncoder::H265_NVENC, VideoQuality::Insane) => 500,
                    (VideoEncoder::H265 | VideoEncoder::H265_NVENC, VideoQuality::Low) => 1000,
                    (VideoEncoder::H265 | VideoEncoder::H265_NVENC, VideoQuality::Medium) => 4000,
                    (VideoEncoder::H265 | VideoEncoder::H265_NVENC, VideoQuality::High) => 12000,
                    (VideoEncoder::H265 | VideoEncoder::H265_NVENC, VideoQuality::Ultra) => 25000,
                    // H264 baseline
                    (VideoEncoder::H264 | VideoEncoder::H264_NVENC, VideoQuality::Insane) => 800,
                    (VideoEncoder::H264 | VideoEncoder::H264_NVENC, VideoQuality::Low) => 1500,
                    (VideoEncoder::H264 | VideoEncoder::H264_NVENC, VideoQuality::Medium) => 5000,
                    (VideoEncoder::H264 | VideoEncoder::H264_NVENC, VideoQuality::High) => 15000,
                    (VideoEncoder::H264 | VideoEncoder::H264_NVENC, VideoQuality::Ultra) => 30000,
                    // VP9 similar to H265
                    (VideoEncoder::VP9, VideoQuality::Insane) => 400,
                    (VideoEncoder::VP9, VideoQuality::Low) => 1000,
                    (VideoEncoder::VP9, VideoQuality::Medium) => 3500,
                    (VideoEncoder::VP9, VideoQuality::High) => 10000,
                    (VideoEncoder::VP9, VideoQuality::Ultra) => 22000,
                    (_, VideoQuality::Custom) => self.video_bitrate_kbps,
                };
                // Scale by resolution and fps
                let res_factor = (self.resolution_width * self.resolution_height) as f64
                    / (1920.0 * 1080.0);
                let fps_factor = self.fps as f64 / 30.0;
                (base as f64 * res_factor * fps_factor) as u32
            }
        }
    }

    /// Estimated file size per minute in MB
    pub fn estimated_mb_per_minute(&self) -> f64 {
        let video_kbps = self.effective_bitrate_kbps() as f64;
        let audio_kbps = if self.audio_enabled { 192.0 } else { 0.0 };
        let total_kbps = video_kbps + audio_kbps;
        // kbps * 60s / 8 bits / 1024 = MB per minute
        total_kbps * 60.0 / 8.0 / 1024.0
    }

    /// Get FFmpeg encoder name
    pub fn ffmpeg_encoder(&self) -> &str {
        match self.video_encoder {
            VideoEncoder::H264 => "libx264",
            VideoEncoder::H265 => "libx265",
            VideoEncoder::AV1 => "libsvtav1",
            VideoEncoder::AV1_NVENC => "av1_nvenc",
            VideoEncoder::H264_NVENC => "h264_nvenc",
            VideoEncoder::H265_NVENC => "hevc_nvenc",
            VideoEncoder::VP9 => "libvpx-vp9",
        }
    }

    /// Get FFmpeg CRF value for quality preset
    pub fn ffmpeg_crf(&self) -> u32 {
        match (&self.video_encoder, &self.video_quality) {
            // SVT-AV1: CRF 0-63, lower = better. Screen content sweet spot: 20-35
            (VideoEncoder::AV1, VideoQuality::Insane) => 42,
            (VideoEncoder::AV1, VideoQuality::Low) => 38,
            (VideoEncoder::AV1, VideoQuality::Medium) => 30,
            (VideoEncoder::AV1, VideoQuality::High) => 23,
            (VideoEncoder::AV1, VideoQuality::Ultra) => 18,
            // NVENC AV1 uses QP (0-51)
            (VideoEncoder::AV1_NVENC, VideoQuality::Insane) => 40,
            (VideoEncoder::AV1_NVENC, VideoQuality::Low) => 35,
            (VideoEncoder::AV1_NVENC, VideoQuality::Medium) => 28,
            (VideoEncoder::AV1_NVENC, VideoQuality::High) => 22,
            (VideoEncoder::AV1_NVENC, VideoQuality::Ultra) => 16,
            // H264
            (VideoEncoder::H264, VideoQuality::Insane) => 35,
            (VideoEncoder::H264, VideoQuality::Low) => 32,
            (VideoEncoder::H264, VideoQuality::Medium) => 23,
            (VideoEncoder::H264, VideoQuality::High) => 18,
            (VideoEncoder::H264, VideoQuality::Ultra) => 14,
            // H265
            (VideoEncoder::H265, VideoQuality::Insane) => 38,
            (VideoEncoder::H265, VideoQuality::Low) => 34,
            (VideoEncoder::H265, VideoQuality::Medium) => 26,
            (VideoEncoder::H265, VideoQuality::High) => 20,
            (VideoEncoder::H265, VideoQuality::Ultra) => 16,
            // NVENC H264/H265 use QP
            (VideoEncoder::H264_NVENC, VideoQuality::Insane) => 36,
            (VideoEncoder::H264_NVENC, VideoQuality::Low) => 32,
            (VideoEncoder::H264_NVENC, VideoQuality::Medium) => 24,
            (VideoEncoder::H264_NVENC, VideoQuality::High) => 18,
            (VideoEncoder::H264_NVENC, VideoQuality::Ultra) => 14,
            (VideoEncoder::H265_NVENC, VideoQuality::Insane) => 38,
            (VideoEncoder::H265_NVENC, VideoQuality::Low) => 34,
            (VideoEncoder::H265_NVENC, VideoQuality::Medium) => 26,
            (VideoEncoder::H265_NVENC, VideoQuality::High) => 20,
            (VideoEncoder::H265_NVENC, VideoQuality::Ultra) => 16,
            // VP9
            (VideoEncoder::VP9, VideoQuality::Insane) => 42,
            (VideoEncoder::VP9, VideoQuality::Low) => 38,
            (VideoEncoder::VP9, VideoQuality::Medium) => 30,
            (VideoEncoder::VP9, VideoQuality::High) => 24,
            (VideoEncoder::VP9, VideoQuality::Ultra) => 18,
            (_, VideoQuality::Custom) => 23,
        }
    }

    /// Get additional FFmpeg args specific to the encoder
    pub fn ffmpeg_extra_args(&self) -> Vec<String> {
        match &self.video_encoder {
            VideoEncoder::AV1 => {
                // SVT-AV1 specific: preset 6 is good speed/quality balance for screen content
                // tune=0 is default (PSNR), film-grain=0 for screen content
                vec![
                    "-preset".into(), "6".into(),
                    "-svtav1-params".into(),
                    "tune=0:film-grain=0:enable-overlays=1:scd=1".into(),
                ]
            }
            VideoEncoder::AV1_NVENC => {
                vec![
                    "-preset".into(), "p4".into(),
                    "-tune".into(), "hq".into(),
                    "-multipass".into(), "fullres".into(),
                ]
            }
            VideoEncoder::H264_NVENC | VideoEncoder::H265_NVENC => {
                vec![
                    "-preset".into(), "p4".into(),
                    "-tune".into(), "hq".into(),
                    "-rc".into(), "constqp".into(),
                ]
            }
            VideoEncoder::H264 => vec!["-preset".into(), "fast".into()],
            VideoEncoder::H265 => vec!["-preset".into(), "fast".into()],
            VideoEncoder::VP9 => vec![
                "-deadline".into(), "good".into(),
                "-cpu-used".into(), "4".into(),
                "-row-mt".into(), "1".into(),
            ],
        }
    }
}
