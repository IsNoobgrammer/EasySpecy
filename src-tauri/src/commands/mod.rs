//! Tauri IPC commands — exposed to the React frontend via invoke()

use crate::capture;
use crate::config::AppConfig;
use cpal::traits::{DeviceTrait, HostTrait};

/// Get the current configuration
#[tauri::command]
pub fn get_config() -> AppConfig {
    AppConfig::load()
}

/// Save configuration
#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

/// Get list of available audio input devices
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

/// Get list of available screens/monitors
#[tauri::command]
pub fn get_screens() -> Result<Vec<capture::DisplayInfo>, String> {
    Ok(capture::get_displays())
}

/// Get EasySpecy version
#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get recording status (placeholder for Phase 1)
#[tauri::command]
pub fn get_recording_status() -> bool {
    false // TODO Phase 1: track actual recording state
}

/// Open a path in the system file explorer
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
