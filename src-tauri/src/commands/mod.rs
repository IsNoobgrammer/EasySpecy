//! Tauri IPC commands

use crate::capture;
use crate::capture::RecordingConfig;
use crate::config::AppConfig;
use crate::history::{RecordingEntry, RecordingHistory};
use crate::region;
use cpal::traits::{DeviceTrait, HostTrait};
use tauri::Manager;

#[tauri::command]
pub fn get_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_config_field(key: String, value: serde_json::Value) -> Result<(), String> {
    let mut config = AppConfig::load();
    match key.as_str() {
        "resolution" => {
            if let Some(v) = value.as_str() {
                let parts: Vec<&str> = v.split('x').collect();
                if parts.len() == 2 {
                    config.resolution_width = parts[0].parse().unwrap_or(1920);
                    config.resolution_height = parts[1].parse().unwrap_or(1080);
                }
            }
        }
        "fps" => config.fps = value.as_u64().unwrap_or(30) as u32,
        "audio_enabled" => config.audio_enabled = value.as_bool().unwrap_or(true),
        "audio_source" => {
            config.audio_source = match value.as_str() {
                Some("Mic") => crate::config::AudioSource::Mic,
                Some("System") => crate::config::AudioSource::System,
                Some("Both") => crate::config::AudioSource::Both,
                _ => crate::config::AudioSource::Mic,
            };
        }
        "audio_sample_rate" => config.audio_sample_rate = value.as_u64().unwrap_or(44100) as u32,
        "recording_mode" => {
            config.recording_mode = match value.as_str() {
                Some("FullScreen") => crate::config::RecordingMode::FullScreen,
                Some("Region") => crate::config::RecordingMode::Region,
                Some("Window") => crate::config::RecordingMode::Window,
                _ => crate::config::RecordingMode::FullScreen,
            };
        }
        "auto_zoom_enabled" => config.auto_zoom_enabled = value.as_bool().unwrap_or(false),
        "cursor_trail_enabled" => config.cursor_trail_enabled = value.as_bool().unwrap_or(false),
        "webcam_enabled" => config.webcam_enabled = value.as_bool().unwrap_or(false),
        _ => return Err(format!("Unknown config key: {}", key)),
    }
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(name);
            }
        }
    }
    Ok(devices)
}

#[tauri::command]
pub fn get_screens() -> Result<Vec<capture::DisplayInfo>, String> {
    Ok(capture::get_displays())
}

#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn start_recording(output_path: Option<String>) -> Result<(), String> {
    let config = AppConfig::load();
    let path = output_path.unwrap_or_else(|| {
        let dir = &config.output_dir;
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        std::path::Path::new(dir)
            .join(format!("recording_{}.mp4", timestamp))
            .to_string_lossy()
            .to_string()
    });

    capture::start_recording(RecordingConfig {
        output_path: path,
        enable_audio: config.audio_enabled,
        audio_sample_rate: config.audio_sample_rate,
        fps: config.fps,
    })
}

#[tauri::command]
pub fn stop_recording() -> Result<capture::RecordingResult, String> {
    let result = capture::stop_recording()?;
    let config = AppConfig::load();
    let entry = RecordingEntry {
        id: uuid::Uuid::new_v4().to_string(),
        output_path: result.output_path.clone(),
        duration_secs: result.duration_secs,
        file_size_bytes: result.file_size_bytes,
        has_audio: result.has_audio,
        resolution: format!("{}x{}", config.resolution_width, config.resolution_height),
        fps: config.fps,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let mut history = RecordingHistory::load();
    history.add(entry);
    Ok(result)
}

#[tauri::command]
pub fn pause_recording_cmd() {
    capture::pause_recording();
}

#[tauri::command]
pub fn resume_recording_cmd() {
    capture::resume_recording();
}

#[tauri::command]
pub fn get_recording_status() -> (bool, bool, u32) {
    (
        capture::is_recording(),
        capture::is_paused(),
        capture::frame_count(),
    )
}

#[tauri::command]
pub fn get_recording_history() -> Vec<RecordingEntry> {
    RecordingHistory::load().entries
}

#[tauri::command]
pub fn clear_recording_history() -> Result<(), String> {
    let mut history = RecordingHistory::load();
    history.clear();
    Ok(())
}

#[tauri::command]
pub fn set_capture_region(x: i32, y: i32, width: i32, height: i32) {
    region::set_region(region::CaptureRegion { x, y, width, height });
}

#[tauri::command]
pub fn get_capture_region() -> Option<region::CaptureRegion> {
    region::get_region()
}

#[tauri::command]
pub fn clear_capture_region() {
    region::clear_region();
}

#[tauri::command]
pub fn get_windows() -> Vec<region::WindowInfo> {
    region::get_windows()
}

/// Enter region selection mode: make window fullscreen and transparent
#[tauri::command]
pub fn enter_region_mode(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_fullscreen(true).map_err(|e| e.to_string())?;
        window.set_always_on_top(true).map_err(|e| e.to_string())?;
        window.set_decorations(false).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Exit region selection mode: restore window
#[tauri::command]
pub fn exit_region_mode(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
        window.set_always_on_top(false).map_err(|e| e.to_string())?;
        window.set_decorations(true).map_err(|e| e.to_string())?;
        window.set_size(tauri::LogicalSize::new(900, 640)).map_err(|e| e.to_string())?;
        window.center().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
