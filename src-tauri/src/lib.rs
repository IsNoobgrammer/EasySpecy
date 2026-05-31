mod audio;
mod capture;
mod commands;
mod config;
mod history;
mod postprocess;
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
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording_cmd,
            commands::resume_recording_cmd,
            commands::get_recording_status,
            commands::get_recording_history,
            commands::clear_recording_history,
            commands::open_path,
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running EasySpecy");
}
