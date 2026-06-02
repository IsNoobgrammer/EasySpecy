mod audio;
pub mod capture;
mod commands;
mod config;
pub mod cursors;
mod history;
mod keyboard;
mod postprocess;
mod region;
pub mod sync_verifier;
pub mod webcam;
mod tray;

use std::sync::OnceLock;
use tauri::AppHandle;
use tracing_subscriber::EnvFilter;

/// Global AppHandle for emitting events from background threads (e.g. cursor tracking)
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Get the global AppHandle (available after app setup)
pub fn app_handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

/// Get the directory next to the exe (release folder)
fn exe_log_dir() -> std::path::PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let log_dir = exe_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    log_dir
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ═══ LOG TO BOTH STDERR + FILE ═══
    // File goes to <exe_dir>/logs/easyspecy.log (next to the release binary)
    let log_dir = exe_log_dir();
    let file_appender = tracing_appender::rolling::never(&log_dir, "easyspecy.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // Keep _guard alive for the entire process — dropping it flushes logs
    // We leak it intentionally since this is the app's main run function
    std::mem::forget(_guard);

    use tracing_subscriber::prelude::*;
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(false)
        .with_ansi(false); // No color codes in log file

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    tracing::info!("EasySpecy starting... (logs → {})", log_dir.display());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::update_config_field,
            commands::get_audio_devices,
            commands::get_screens,
            commands::get_version,
            commands::get_estimated_size,
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording_cmd,
            commands::resume_recording_cmd,
            commands::get_recording_status,
            commands::is_capture_ready,
            commands::get_encoding_progress,
            commands::get_estimated_size,
            commands::detect_gpu_encoders,
            commands::get_recording_history,
            commands::clear_recording_history,
            commands::delete_recording,
            commands::set_capture_region,
            commands::get_capture_region,
            commands::clear_capture_region,
            commands::get_windows,
            commands::enter_region_mode,
            commands::exit_region_mode,
            commands::open_path,
            commands::get_cursor_packs,
            commands::apply_cursor_pack,
            commands::restore_cursors,
            commands::create_effects_overlay,
            commands::destroy_effects_overlay,
            commands::start_audio_monitor_cmd,
            commands::stop_audio_monitor_cmd,
            commands::get_audio_levels,
            commands::get_webcam_devices,
            commands::get_keyboard_events,
        ])
        .setup(|app| {
            // Store AppHandle globally for background thread access (cursor events)
            let _ = APP_HANDLE.set(app.handle().clone());
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running EasySpecy");
}
