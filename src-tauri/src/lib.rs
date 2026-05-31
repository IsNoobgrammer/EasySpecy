mod audio;
pub mod capture;
mod commands;
mod config;
pub mod cursors;
mod history;
mod postprocess;
mod region;
pub mod sync_verifier;
mod tray;

use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("EasySpecy starting...");

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
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running EasySpecy");
}
