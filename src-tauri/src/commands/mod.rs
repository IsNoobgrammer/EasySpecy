//! Tauri IPC commands — exposed to the React frontend via invoke()

use crate::capture;
use crate::config::AppConfig;
use cpal::traits::{DeviceTrait, HostTrait};

#[tauri::command]
pub fn get_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
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
        format!("{}/recording_{}.mp4", dir, timestamp)
    });
    capture::start_recording(path, config.audio_enabled)
}

#[tauri::command]
pub fn stop_recording() -> Result<capture::RecordingResult, String> {
    capture::stop_recording()
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
