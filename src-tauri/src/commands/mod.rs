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
    config.save().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        if config.keyboard_game_capture && !crate::is_elevated() {
            if crate::relaunch_as_admin() {
                std::process::exit(0);
            }
        } else if !config.keyboard_game_capture && crate::is_elevated() {
            if crate::relaunch_as_standard() {
                std::process::exit(0);
            }
        }
    }

    Ok(())
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
                _ => crate::config::RecordingMode::FullScreen,
            };
        }
        "auto_zoom_enabled" => config.auto_zoom_enabled = value.as_bool().unwrap_or(false),
        "zoom_sensitivity" => config.zoom_sensitivity = value.as_f64().unwrap_or(0.5) as f32,
        "zoom_speed" => config.zoom_speed = value.as_f64().unwrap_or(1.0) as f32,
        "cursor_trail_enabled" => config.cursor_trail_enabled = value.as_bool().unwrap_or(false),
        "webcam_enabled" => config.webcam_enabled = value.as_bool().unwrap_or(false),
        "cursor_pack" => {
            config.cursor_pack = value.as_str().unwrap_or("default").to_string();
        }
        "trail_style" => {
            config.trail_style = value.as_str().unwrap_or("glow").to_string();
        }
        "click_effect" => {
            config.click_effect = value.as_str().unwrap_or("ripple").to_string();
        }
        "cursor_trail_color" => {
            config.cursor_trail_color = value.as_str().unwrap_or("#00ff88").to_string();
        }
        "video_encoder" => {
            config.video_encoder = match value.as_str() {
                Some("H264") => crate::config::VideoEncoder::H264,
                Some("H265") => crate::config::VideoEncoder::H265,
                Some("AV1") => crate::config::VideoEncoder::AV1,
                Some("AV1_NVENC") => crate::config::VideoEncoder::AV1_NVENC,
                Some("H264_NVENC") => crate::config::VideoEncoder::H264_NVENC,
                Some("H265_NVENC") => crate::config::VideoEncoder::H265_NVENC,
                Some("VP9") => crate::config::VideoEncoder::VP9,
                Some("MobileShareable") => crate::config::VideoEncoder::MobileShareable,
                _ => crate::config::VideoEncoder::AV1,
            };
        }
        "video_quality" => {
            config.video_quality = match value.as_str() {
                Some("Low") => crate::config::VideoQuality::Low,
                Some("Medium") => crate::config::VideoQuality::Medium,
                Some("High") => crate::config::VideoQuality::High,
                Some("Ultra") => crate::config::VideoQuality::Ultra,
                Some("Insane") => crate::config::VideoQuality::Insane,
                Some("Custom") => crate::config::VideoQuality::Custom,
                _ => crate::config::VideoQuality::Medium,
            };
        }
        "video_bitrate_kbps" => config.video_bitrate_kbps = value.as_u64().unwrap_or(4000) as u32,
        "cursor_secondary_color" => {
            config.cursor_secondary_color = value.as_str().unwrap_or("#ff4488").to_string();
        }
        "keyboard_game_capture" => {
            config.keyboard_game_capture = value.as_bool().unwrap_or(false);
        }
        "auto_check_updates" => {
            config.auto_check_updates = value.as_bool().unwrap_or(true);
        }
        "gpu_encoders_enabled" => {
            config.gpu_encoders_enabled = value.as_bool().unwrap_or(false);
        }
        "compact_output" => config.compact_output = value.as_bool().unwrap_or(false),
        // Keyboard overlay fields
        "keyboard_overlay_enabled" => config.keyboard_overlay_enabled = value.as_bool().unwrap_or(false),
        "keyboard_overlay_font_family" => config.keyboard_overlay_font_family = value.as_str().unwrap_or("JetBrains Mono").to_string(),
        "keyboard_overlay_font_size" => config.keyboard_overlay_font_size = value.as_u64().unwrap_or(14) as u32,
        "keyboard_overlay_opacity" => config.keyboard_overlay_opacity = value.as_f64().unwrap_or(0.9) as f32,
        "keyboard_overlay_x" => config.keyboard_overlay_x = value.as_i64().unwrap_or(480) as i32,
        "keyboard_overlay_y" => config.keyboard_overlay_y = value.as_i64().unwrap_or(800) as i32,
        "keyboard_overlay_corner_radius" => config.keyboard_overlay_corner_radius = value.as_u64().unwrap_or(8) as u32,
        "keyboard_overlay_border_width" => config.keyboard_overlay_border_width = value.as_u64().unwrap_or(1) as u32,
        "keyboard_overlay_border_color" => config.keyboard_overlay_border_color = value.as_str().unwrap_or("rgba(255, 255, 255, 0.15)").to_string(),
        "keyboard_overlay_background_color" => config.keyboard_overlay_background_color = value.as_str().unwrap_or("rgba(0, 0, 0, 0.65)").to_string(),
        "keyboard_overlay_text_color" => config.keyboard_overlay_text_color = value.as_str().unwrap_or("#e5e5e0").to_string(),
        "keyboard_overlay_theme" => config.keyboard_overlay_theme = value.as_str().unwrap_or("speccy-classic").to_string(),
        "keyboard_overlay_key_mappings" => config.keyboard_overlay_key_mappings = value.as_str().unwrap_or("{}").to_string(),
        "keyboard_overlay_max_bubbles" => config.keyboard_overlay_max_bubbles = value.as_u64().unwrap_or(4) as u32,
        "keyboard_overlay_bubble_timeout_ms" => config.keyboard_overlay_bubble_timeout_ms = value.as_u64().unwrap_or(3000) as u32,
        "keyboard_overlay_width" => config.keyboard_overlay_width = value.as_u64().unwrap_or(360) as u32,

        _ => return Err(format!("Unknown config key: {}", key)),
    }
    config.save().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        if config.keyboard_game_capture && !crate::is_elevated() {
            if crate::relaunch_as_admin() {
                std::process::exit(0);
            }
        } else if !config.keyboard_game_capture && crate::is_elevated() {
            if crate::relaunch_as_standard() {
                std::process::exit(0);
            }
        }
    }

    Ok(())
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
pub fn get_estimated_size() -> (f64, u32, String) {
    let config = AppConfig::load();
    let mb_per_min = config.estimated_mb_per_minute();
    let bitrate = config.effective_bitrate_kbps();
    let encoder = config.ffmpeg_encoder().to_string();
    (mb_per_min, bitrate, encoder)
}

/// Auto-detect available GPU encoders by probing FFmpeg
#[tauri::command]
pub fn detect_gpu_encoders() -> Vec<String> {
    let ffmpeg = capture::find_ffmpeg_pub();
    let Some(ffmpeg_path) = ffmpeg else {
        return Vec::new();
    };

    let mut cmd = std::process::Command::new(&ffmpeg_path);
    cmd.args(["-encoders"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut available = Vec::new();

    // Check for NVENC encoders
    if stdout.contains("h264_nvenc") {
        available.push("H264_NVENC".to_string());
    }
    if stdout.contains("hevc_nvenc") {
        available.push("H265_NVENC".to_string());
    }
    if stdout.contains("av1_nvenc") {
        available.push("AV1_NVENC".to_string());
    }
    // Check for AMD AMF encoders
    if stdout.contains("h264_amf") {
        available.push("H264_AMF".to_string());
    }
    if stdout.contains("hevc_amf") {
        available.push("H265_AMF".to_string());
    }
    if stdout.contains("av1_amf") {
        available.push("AV1_AMF".to_string());
    }
    // Check for Intel QSV encoders
    if stdout.contains("h264_qsv") {
        available.push("H264_QSV".to_string());
    }
    if stdout.contains("hevc_qsv") {
        available.push("H265_QSV".to_string());
    }
    if stdout.contains("av1_qsv") {
        available.push("AV1_QSV".to_string());
    }
    // Check for SVT-AV1 (CPU but important)
    if stdout.contains("libsvtav1") {
        available.push("AV1".to_string());
    }

    tracing::info!("Detected GPU encoders: {:?}", available);
    available
}

#[tauri::command]
pub fn get_cursor_packs() -> Vec<crate::cursors::CursorPackInfo> {
    crate::cursors::list_cursor_packs()
}

#[tauri::command]
pub fn apply_cursor_pack(pack_id: String) -> Result<(), String> {
    crate::cursors::apply_cursor_pack(&pack_id)
}

#[tauri::command]
pub fn restore_cursors() -> Result<(), String> {
    crate::cursors::restore_cursors()
}

/// Start recording — waits until capture is actually armed (first frame received)
/// before returning success. This ensures the frontend timer is perfectly synced.
/// True while the current session is a preview (temp file, not added to history).
static PREVIEW_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn preview_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("easyspecy_preview")
}

/// Previews are throwaway: removed on startup and exit, replaced by each new preview.
pub fn remove_preview() {
    let _ = std::fs::remove_dir_all(preview_dir());
}

#[tauri::command]
pub async fn start_recording(app: tauri::AppHandle, output_path: Option<String>, preview: Option<bool>) -> Result<(), String> {
    let config = AppConfig::load();
    let preview = preview.unwrap_or(false);
    PREVIEW_ACTIVE.store(preview, std::sync::atomic::Ordering::SeqCst);
    let output_path = if preview {
        remove_preview(); // one preview at a time — the new one replaces the old
        let _ = std::fs::create_dir_all(preview_dir());
        Some(preview_dir().join("preview.mp4").to_string_lossy().into_owned())
    } else {
        output_path
    };

    // ═══ Apply cursor pack BEFORE capture starts ═══
    if config.cursor_pack != "default" && !config.cursor_pack.is_empty() {
        if let Err(e) = crate::cursors::apply_cursor_pack(&config.cursor_pack) {
            tracing::warn!("Cursor pack '{}' failed to apply: {}", config.cursor_pack, e);
        } else {
            tracing::info!("Cursor pack '{}' applied", config.cursor_pack);
        }
    }

    let path = output_path.unwrap_or_else(|| {
        let dir = &config.output_dir;
        let filename = chrono::Local::now().format("EasySpecy_%d_%B_%Y_%H_%M_%S.mp4").to_string();
        std::path::Path::new(dir)
            .join(filename)
            .to_string_lossy()
            .to_string()
    });

    // The recording streams feed the level meter themselves — a monitor stream on the same
    // devices during recording just captured everything twice.
    crate::audio::stop_audio_monitor();
    crate::audio::reset_levels();

    // Initialize capture pipeline (spawns threads, creates streams)
    let started = capture::start_recording(RecordingConfig {
        output_path: path.clone(),
        enable_audio: config.audio_enabled,
        audio_source: format!("{:?}", config.audio_source),
        audio_sample_rate: config.audio_sample_rate,
        fps: config.fps,
    });
    if let Err(e) = started {
        let _ = crate::cursors::restore_cursors();
        let _ = start_audio_monitor_cmd();
        return Err(e);
    }

    // Start keyboard capture for the overlay
    if config.keyboard_overlay_enabled {
        crate::keyboard::start_keyboard_capture();
    }

    // Create effects overlay window — handles cursor trail, keyboard overlay, AND webcam PiP.
    // All three live inside the same fullscreen transparent WebView2 window (overlay.html).
    //
    // NOTE: Even when only the keyboard overlay is enabled (no cursor trail, no webcam),
    // we still create a fullscreen transparent window. This is required because the keyboard
    // overlay is rendered inside overlay.html, which runs in this window. A future optimization
    // could use a smaller, positioned window for keyboard-only mode to reduce memory and
    // compositing overhead, but that requires changes to overlay.html coordinate math and
    // always-on-top window management. Deferred to a future PR.
    // Always created: besides effects, its heartbeat keeps capture frames flowing on static screens.
    let _ = create_effects_overlay(app.clone());

    // ═══ WAIT until capture is actually armed (first video frame received) ═══
    // This is the key fix: frontend won't show "recording" until we're ACTUALLY recording.
    // Timeout after 10s to avoid hanging forever if something goes wrong.
    let wait_result = tauri::async_runtime::spawn_blocking(|| {
        let start = std::time::Instant::now();
        while !capture::is_capture_ready() {
            if start.elapsed() > std::time::Duration::from_secs(10) {
                return Err("Capture initialization timeout (10s) — no frames received".to_string());
            }
            if !capture::is_recording() {
                return Err("Capture failed to start".to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    if let Err(e) = wait_result {
        // Undo everything start did, or the next start says "already in progress" and the
        // user is left with custom cursors, a live keyboard hook and a hidden overlay.
        tracing::error!("start_recording failed: {}", e);
        crate::keyboard::stop_keyboard_capture();
        let _ = destroy_effects_overlay(app.clone());
        let _ = crate::cursors::restore_cursors();
        tauri::async_runtime::spawn_blocking(capture::abort_recording).await.ok();
        let _ = start_audio_monitor_cmd();
        return Err(e);
    }

    // Minimize / hide main window to tray if enabled
    if config.minimize_to_tray {
        if let Some(main_window) = app.get_webview_window("main") {
            let _ = main_window.hide();
            crate::emit_main_visible(false);
        }
    }

    // Make effects overlay visible from Rust side
    if let Some(overlay) = app.get_webview_window("effects-overlay") {
        let _ = overlay.show();
        tracing::info!("Effects overlay window made visible from Rust");
    }


    tracing::info!("start_recording: capture armed, returning to frontend");
    crate::tray::update_tray_state(true, false);
    Ok(())
}

/// Check if capture is actively recording (ready state)
#[tauri::command]
pub fn is_capture_ready() -> bool {
    capture::is_capture_ready()
}

/// Get encoding progress (0-100) and stage description
#[tauri::command]
pub fn get_encoding_progress() -> (u32, String) {
    capture::get_encoding_progress()
}

#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<capture::RecordingResult, String> {
    // Restore main window if minimized/hidden to tray
    let config = AppConfig::load();
    if config.minimize_to_tray {
        if let Some(main_window) = app.get_webview_window("main") {
            let _ = main_window.show();
            let _ = main_window.set_focus();
            crate::emit_main_visible(true);
        }
    }

    // Stop keyboard capture FIRST (while overlay is still alive to display final events)
    crate::keyboard::stop_keyboard_capture();

    // THEN destroy effects overlay window (contains cursor trail, keyboard overlay AND webcam PiP)
    let _ = destroy_effects_overlay(app.clone());

    // ═══ Restore cursors so user sees normal cursor during encoding ═══
    if let Err(e) = crate::cursors::restore_cursors() {
        tracing::warn!("Cursor restore failed: {}", e);
    }

    // Run encoding on a background thread to avoid blocking the UI
    let result = tauri::async_runtime::spawn_blocking(|| {
        capture::stop_recording()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            crate::tray::update_tray_state(false, false); // don't leave the tray stuck on "recording"
            return Err(e);
        }
    };

    if PREVIEW_ACTIVE.swap(false, std::sync::atomic::Ordering::SeqCst) {
        crate::audio::reset_levels();
        let _ = start_audio_monitor_cmd();
        crate::tray::update_tray_state(false, false);
        return Ok(result); // previews never enter history
    }
    let config = AppConfig::load();
    let entry = RecordingEntry {
        id: uuid::Uuid::new_v4().to_string(),
        output_path: result.output_path.clone(),
        duration_secs: result.duration_secs,
        file_size_bytes: result.file_size_bytes,
        has_audio: result.has_audio,
        resolution: format!("{}x{}", result.width, result.height),
        fps: config.fps,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let mut history = RecordingHistory::load();
    history.add(entry);
    // Back to idle: level meter returns to the lightweight monitor
    crate::audio::reset_levels();
    let _ = start_audio_monitor_cmd();
    crate::tray::update_tray_state(false, false);
    Ok(result)
}

#[tauri::command]
pub fn pause_recording_cmd() {
    capture::pause_recording();
    crate::tray::update_tray_state(true, true);
}

#[tauri::command]
pub fn resume_recording_cmd() {
    capture::resume_recording();
    crate::tray::update_tray_state(true, false);
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

/// Enter region selection: hide the app, screenshot the primary monitor (the one we capture),
/// then cover it fullscreen. Returns the screenshot so the selector draws on the real desktop
/// (the main window isn't transparent, so without this the user saw the app's own background).
#[tauri::command]
pub async fn enter_region_mode(app: tauri::AppHandle) -> Result<String, String> {
    let window = app.get_webview_window("main").ok_or("No main window")?;
    window.hide().map_err(|e| e.to_string())?;
    let shot = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(250)); // let the hide animation finish
        region::capture_screen_png()
    })
    .await
    .map_err(|e| e.to_string())??;
    if let Some(m) = app.primary_monitor().map_err(|e| e.to_string())? {
        let _ = window.set_position(*m.position());
    }
    window.set_decorations(false).map_err(|e| e.to_string())?;
    window.set_fullscreen(true).map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();
    Ok(shot)
}

/// Bring the window chosen for Window mode to the front before recording starts.
#[tauri::command]
pub fn focus_window(hwnd: isize) {
    region::focus_window(hwnd);
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

#[tauri::command]
pub fn delete_recording(path: String) -> Result<(), String> {
    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("File not found".to_string());
    }
    std::fs::remove_file(file_path).map_err(|e| e.to_string())?;
    // Also remove from history
    let mut history = RecordingHistory::load();
    history.entries.retain(|entry| entry.output_path != path);
    history.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Create a transparent fullscreen overlay window for trail + click effects during recording.
#[tauri::command]
pub fn create_effects_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewUrl;

    if app.get_webview_window("effects-overlay").is_some() {
        return Ok(());
    }

    // Get primary monitor size via Tauri
    let monitor = app
        .primary_monitor()
        .map_err(|e| format!("Monitor query failed: {}", e))?
        .ok_or("No primary monitor found")?;
    let scale_factor = monitor.scale_factor();
    let size = monitor.size();
    let logical_width = size.width as f64 / scale_factor;
    let logical_height = size.height as f64 / scale_factor;

    let overlay = tauri::WebviewWindowBuilder::new(
        &app,
        "effects-overlay",
        WebviewUrl::App("/overlay.html".into()),
    )
    .title("EasySpecy Effects")
    .inner_size(logical_width, logical_height)
    .position(0.0, 0.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .visible(false) // Start hidden to prevent white background flash on Windows
    .build()
    .map_err(|e| format!("Failed to create overlay window: {}", e))?;

    let _ = overlay.set_ignore_cursor_events(true);

    // Periodically re-assert always-on-top state to prevent games from going over the overlay HUD
    let overlay_clone = overlay.clone();
    std::thread::spawn(move || {
        while overlay_clone.is_minimized().is_ok() {
            if let Ok(true) = overlay_clone.is_visible() {
                let _ = overlay_clone.set_always_on_top(true);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    tracing::info!("Effects overlay window created: {}x{} (logical)", logical_width, logical_height);
    Ok(())
}

/// Start audio level monitoring (pre-recording mic/system check)
#[tauri::command]
pub fn start_audio_monitor_cmd() -> Result<(), String> {
    let config = AppConfig::load();
    if !config.audio_enabled {
        return Ok(());
    }
    let source = match config.audio_source {
        crate::config::AudioSource::Mic => crate::audio::AudioSource::Mic,
        crate::config::AudioSource::System => crate::audio::AudioSource::System,
        crate::config::AudioSource::Both => crate::audio::AudioSource::Both,
    };
    crate::audio::start_audio_monitor(&source)
}

/// Stop audio level monitoring
#[tauri::command]
pub fn stop_audio_monitor_cmd() {
    crate::audio::stop_audio_monitor();
}

/// Get current audio levels (mic + system RMS/peak/dB)
/// Returns snapshot from atomic levels — no locks, real-time safe.
#[tauri::command]
pub fn get_audio_levels() -> crate::audio::AudioLevels {
    crate::audio::get_audio_levels()
}

/// Destroy the effects overlay window
#[tauri::command]
pub fn destroy_effects_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("effects-overlay") {
        window.close().map_err(|e| e.to_string())?;
        tracing::info!("Effects overlay window destroyed");
    }
    Ok(())
}

/// Run the encoder benchmark (several seconds) and cache the result.
#[tauri::command]
pub async fn scan_encoders() -> Result<crate::encoders::EncoderScan, String> {
    tauri::async_runtime::spawn_blocking(crate::encoders::scan).await.map_err(|e| e.to_string())?
}

/// Render what each quality preset looks like on the user's own screen (~5 s).
#[tauri::command]
pub async fn quality_preview(app: tauri::AppHandle, custom_kbps: Option<u32>) -> Result<Vec<crate::quality_preview::QualitySample>, String> {
    let t0 = std::time::Instant::now();
    let config = AppConfig::load();
    crate::quality_preview::progress(0.0, 0.1, "Capturing your screen…", 0.4);
    // Sample the user's real screen, not this window: hide it just for the screenshot
    let main = app.get_webview_window("main");
    if let Some(w) = &main { let _ = w.hide(); }
    let shot = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(250));
        crate::region::capture_screen_png_bytes()
    }).await.map_err(|e| e.to_string())?;
    if let Some(w) = &main { let _ = w.show(); let _ = w.set_focus(); }
    let shot = shot?;
    tracing::info!("Quality preview: screenshot ready after {:.1}s", t0.elapsed().as_secs_f64());
    tauri::async_runtime::spawn_blocking(move || crate::quality_preview::render(&config, custom_kbps, shot))
        .await
        .map_err(|e| e.to_string())?
}

/// Last scan result, if one exists.
#[tauri::command]
pub fn get_encoder_scan() -> Option<crate::encoders::EncoderScan> {
    crate::encoders::cached()
}

#[tauri::command]
pub fn get_keyboard_events() -> Vec<crate::keyboard::KeyEvent> {
    crate::keyboard::get_keyboard_events()
}

/// Detect if the app is running in portable mode (`.portable` marker file next to exe)
#[tauri::command]
pub fn is_portable_mode() -> bool {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    match exe_dir {
        Some(dir) => dir.join(".portable").exists(),
        None => false,
    }
}

/// Install a portable update: download ZIP, extract, replace exe, relaunch.
/// This handles the full lifecycle for portable (non-NSIS) installations.
#[tauri::command]
pub async fn install_portable_update(url: String) -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Cannot find exe: {}", e))?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Cannot determine exe directory")?
        .to_path_buf();

    tracing::info!("Portable update: downloading from {}", url);

    // 1. Download the ZIP to a temp file
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read download failed: {}", e))?;

    let temp_dir = std::env::temp_dir().join("easyspecy_update");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Cannot create temp dir: {}", e))?;

    let zip_path = temp_dir.join("update.zip");
    std::fs::write(&zip_path, &bytes)
        .map_err(|e| format!("Cannot write zip: {}", e))?;

    tracing::info!("Portable update: downloaded {} bytes", bytes.len());

    // 2. Extract the ZIP
    let extract_dir = temp_dir.join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Cannot create extract dir: {}", e))?;

    let zip_file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Cannot read zip: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Zip entry error: {}", e))?;
        let out_path = extract_dir.join(entry.mangled_name());
        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut outfile = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create file: {}", e))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Cannot extract: {}", e))?;
        }
    }
    drop(archive);

    tracing::info!("Portable update: extracted to {}", extract_dir.display());

    // 3. Find the new exe inside the extracted archive
    //    The ZIP contains "EasySpecy/EasySpecy.exe" (portable layout)
    let new_exe = find_exe_in_dir(&extract_dir)?;
    tracing::info!("Portable update: found new exe at {}", new_exe.display());

    // 4. Copy resources (ffmpeg, cursors) from the extracted archive
    let new_resources = new_exe.parent().unwrap_or(&extract_dir).join("resources");
    let current_resources = exe_dir.join("resources");
    if new_resources.exists() {
        copy_dir_recursive(&new_resources, &current_resources)?;
        tracing::info!("Portable update: resources updated");
    }

    // 5. Replace the exe: rename old to .old, copy new in place
    let backup_path = exe_path.with_extension("exe.old");
    let _ = std::fs::remove_file(&backup_path); // Remove any previous backup
    std::fs::rename(&exe_path, &backup_path)
        .map_err(|e| format!("Cannot backup old exe: {}", e))?;

    std::fs::copy(&new_exe, &exe_path)
        .map_err(|e| {
            // Try to restore backup on failure
            let _ = std::fs::rename(&backup_path, &exe_path);
            format!("Cannot install new exe: {}", e)
        })?;

    tracing::info!("Portable update: exe replaced successfully");

    // 6. Clean up temp files
    let _ = std::fs::remove_dir_all(&temp_dir);

    // 7. Launch the new exe and exit current process
    std::process::Command::new(&exe_path)
        .spawn()
        .map_err(|e| format!("Cannot launch new exe: {}", e))?;

    tracing::info!("Portable update: launched new version, exiting");
    std::process::exit(0);
}

/// Recursively find the main exe inside an extracted directory
fn find_exe_in_dir(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    // Look for EasySpecy.exe (case-insensitive) recursively
    fn search_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = search_dir(&path) {
                        return Some(found);
                    }
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.eq_ignore_ascii_case("easyspecy.exe") {
                        return Some(path);
                    }
                }
            }
        }
        None
    }
    search_dir(dir).ok_or_else(|| "EasySpecy.exe not found in update archive".to_string())
}

/// Recursively copy a directory tree
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("mkdir failed: {}", e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("readdir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

