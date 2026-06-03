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
pub mod sync_manager;


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
    // Disable background throttling for Webview2 (forces overlay to render at 60 FPS in background/when game is focused)
    #[cfg(target_os = "windows")]
    {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-features=CalculateWindowOcclusionForOcclusionState"
        );
    }

    // Check if we need to run as administrator
    let config = config::AppConfig::load();
    #[cfg(target_os = "windows")]
    {
        if config.keyboard_game_capture && !is_elevated() {
            if relaunch_as_admin() {
                std::process::exit(0);
            }
        }
    }


    // ═══ LOG TO BOTH STDERR + FILE (TRUNCATED ON STARTUP) ═══
    // File goes to <exe_dir>/logs/easyspecy.log (next to the release binary)
    let log_dir = exe_log_dir();
    let log_path = log_dir.join("easyspecy.log");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)
        .expect("Failed to open log file");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file);
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
            commands::create_webcam_overlay,
            commands::destroy_webcam_overlay,
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

#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};

    unsafe {
        let mut handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            
            let result = GetTokenInformation(
                handle,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size,
                &mut size,
            );
            
            let _ = CloseHandle(handle);
            return result.is_ok() && elevation.TokenIsElevated != 0;
        }
    }
    false
}

#[cfg(target_os = "windows")]
pub fn relaunch_as_admin() -> bool {
    use windows::core::w;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    use std::os::windows::ffi::OsStrExt;
    let mut exe_path_wide: Vec<u16> = exe_path.as_os_str().encode_wide().collect();
    exe_path_wide.push(0);

    unsafe {
        let result = ShellExecuteW(
            None,
            w!("runas"),
            windows::core::PCWSTR(exe_path_wide.as_ptr()),
            None,
            None,
            SW_NORMAL,
        );
        (result.0 as isize) > 32
    }
}

#[cfg(target_os = "windows")]
pub fn relaunch_as_standard() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let status = std::process::Command::new("explorer.exe")
        .arg(exe_path)
        .status();

    status.is_ok()
}




