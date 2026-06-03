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
            let parsed = match value.as_str() {
                Some("H264") => crate::config::VideoEncoder::H264,
                Some("H265") => crate::config::VideoEncoder::H265,
                Some("AV1") => crate::config::VideoEncoder::AV1,
                Some("AV1_NVENC") => crate::config::VideoEncoder::AV1_NVENC,
                Some("H264_NVENC") => crate::config::VideoEncoder::H264_NVENC,
                Some("H265_NVENC") => crate::config::VideoEncoder::H265_NVENC,
                Some("VP9") => crate::config::VideoEncoder::VP9,
                _ => crate::config::VideoEncoder::AV1,
            };
            if !config.gpu_encoders_enabled {
                config.video_encoder = match parsed {
                    crate::config::VideoEncoder::AV1_NVENC => crate::config::VideoEncoder::AV1,
                    crate::config::VideoEncoder::H264_NVENC => crate::config::VideoEncoder::H264,
                    crate::config::VideoEncoder::H265_NVENC => crate::config::VideoEncoder::H265,
                    other => other,
                };
            } else {
                config.video_encoder = parsed;
            }
        }
        "gpu_encoders_enabled" => {
            let enabled = value.as_bool().unwrap_or(false);
            config.gpu_encoders_enabled = enabled;
            if !enabled {
                match config.video_encoder {
                    crate::config::VideoEncoder::AV1_NVENC => config.video_encoder = crate::config::VideoEncoder::AV1,
                    crate::config::VideoEncoder::H264_NVENC => config.video_encoder = crate::config::VideoEncoder::H264,
                    crate::config::VideoEncoder::H265_NVENC => config.video_encoder = crate::config::VideoEncoder::H265,
                    _ => {}
                }
            }
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
#[tauri::command]
pub async fn start_recording(app: tauri::AppHandle, output_path: Option<String>) -> Result<(), String> {
    let config = AppConfig::load();

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
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        std::path::Path::new(dir)
            .join(format!("recording_{}.mp4", timestamp))
            .to_string_lossy()
            .to_string()
    });

    // Initialize capture pipeline (spawns threads, creates streams)
    capture::start_recording(RecordingConfig {
        output_path: path.clone(),
        enable_audio: config.audio_enabled,
        audio_source: format!("{:?}", config.audio_source),
        audio_sample_rate: config.audio_sample_rate,
        fps: config.fps,
        webcam_enabled: false, // Bypassed because we use live HTML5 overlay window!
        webcam_device: config.webcam_device.clone(),
        webcam_size: config.webcam_size,
        webcam_x: config.webcam_x,
        webcam_y: config.webcam_y,
        webcam_shape: format!("{:?}", config.webcam_shape),
        webcam_border_color: config.webcam_border_color.clone(),
        webcam_border_width: config.webcam_border_width,
        webcam_opacity: config.webcam_opacity,
    })?;

    // Start cursor metadata collection for post-processing
    crate::postprocess::start_collection();

    // Start keyboard capture for overlay or auto-zoom if enabled
    if config.keyboard_overlay_enabled || config.auto_zoom_enabled {
        crate::keyboard::start_keyboard_capture();
    }

    // Create effects overlay window — handles cursor trail, keyboard overlay, AND webcam PiP.
    // All three live inside the same fullscreen transparent WebView2 window (overlay.html).
    if config.keyboard_overlay_enabled || config.cursor_trail_enabled || config.webcam_enabled {
        let _ = create_effects_overlay(app.clone());
    }

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

    wait_result?;

    // Minimize / hide main window to tray if enabled
    if config.minimize_to_tray {
        if let Some(main_window) = app.get_webview_window("main") {
            let _ = main_window.hide();
        }
    }

    // Make effects overlay visible from Rust side
    if let Some(overlay) = app.get_webview_window("effects-overlay") {
        let _ = overlay.show();
        tracing::info!("Effects overlay window made visible from Rust");
    }

    // Webcam overlay is shown from JS (webcam.html) once the video stream starts playing.
    // The JS has a 2-second fallback timer as well, so no Rust-side show needed.

    tracing::info!("start_recording: capture armed, returning to frontend");
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
    // Restore main window if minimized/hidden/out of focus so user sees encoding progress
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.unminimize();
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }

    // Destroy effects overlay window (contains cursor trail, keyboard overlay AND webcam PiP)
    let _ = destroy_effects_overlay(app.clone());
    // Note: webcam-overlay window no longer exists as a separate window — it's part of effects overlay

    // Stop keyboard capture
    crate::keyboard::stop_keyboard_capture();

    // ═══ Restore cursors so user sees normal cursor during encoding ═══
    if let Err(e) = crate::cursors::restore_cursors() {
        tracing::warn!("Cursor restore failed: {}", e);
    }

    // Run encoding on a background thread to avoid blocking the UI
    let result = tauri::async_runtime::spawn_blocking(|| {
        capture::stop_recording()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    ?;

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

#[derive(serde::Serialize)]
pub struct RecordingStatus {
    pub is_recording: bool,
    pub is_paused: bool,
    pub active_time_ms: u64,
}

#[tauri::command]
pub fn get_recording_status() -> RecordingStatus {
    RecordingStatus {
        is_recording: capture::is_recording(),
        is_paused: capture::is_paused(),
        active_time_ms: capture::get_active_recording_time(),
    }
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
            std::thread::sleep(std::time::Duration::from_millis(150));
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

/// Create a transparent webcam overlay window for live webcam PiP during recording.
#[tauri::command]
pub fn create_webcam_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewUrl;

    if app.get_webview_window("webcam-overlay").is_some() {
        return Ok(());
    }

    let config = AppConfig::load();
    if !config.webcam_enabled {
        tracing::info!("Webcam overlay skipped: webcam_enabled=false");
        return Ok(());
    }

    // Get primary monitor dimensions and scale factor
    let monitor = app
        .primary_monitor()
        .map_err(|e| format!("Monitor query failed: {}", e))?
        .ok_or("No primary monitor found")?;
    let scale_factor = monitor.scale_factor();
    let mon_phys_w = monitor.size().width as f64;
    let mon_phys_h = monitor.size().height as f64;
    // Logical monitor dimensions = physical / scale_factor
    let mon_log_w = mon_phys_w / scale_factor;
    let mon_log_h = mon_phys_h / scale_factor;

    // Config stores x/y/size in OUTPUT resolution space (resolution_width x resolution_height).
    // Convert to logical monitor pixels:
    //   logical = config_val * (monitor_logical_dim / config_resolution_dim)
    let res_w = config.resolution_width as f64;
    let res_h = config.resolution_height as f64;
    let scale_x = mon_log_w / res_w;
    let scale_y = mon_log_h / res_h;

    let mut logical_x = config.webcam_x as f64 * scale_x;
    let mut logical_y = config.webcam_y as f64 * scale_y;
    // Use average scale for size (keep aspect ratio)
    let logical_size = config.webcam_size as f64 * ((scale_x + scale_y) / 2.0);

    // In region mode, offset by region origin (also scaled)
    if config.recording_mode == crate::config::RecordingMode::Region {
        if let Some(r) = crate::region::get_region() {
            logical_x += r.x as f64 * scale_x;
            logical_y += r.y as f64 * scale_y;
        }
    }

    // Clamp to screen bounds
    logical_x = logical_x.clamp(0.0, mon_log_w - logical_size);
    logical_y = logical_y.clamp(0.0, mon_log_h - logical_size);

    tracing::info!(
        "Webcam overlay: config({}, {}) size={} | monitor={}x{} (logical) scale={} | res={}x{} \
         → logical({:.1}, {:.1}) size={:.1}",
        config.webcam_x, config.webcam_y, config.webcam_size,
        mon_log_w, mon_log_h, scale_factor,
        res_w, res_h,
        logical_x, logical_y, logical_size
    );

    let overlay = tauri::WebviewWindowBuilder::new(
        &app,
        "webcam-overlay",
        WebviewUrl::App("/webcam.html".into()),
    )
    .title("EasySpecy Webcam")
    .inner_size(logical_size, logical_size)
    .position(logical_x, logical_y)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .visible(false) // JS shows it once camera stream starts
    .build()
    .map_err(|e| format!("Failed to create webcam window: {}", e))?;

    let _ = overlay.set_ignore_cursor_events(true);

    // Periodically re-assert always-on-top state to keep webcam overlay visible over games
    let overlay_clone = overlay.clone();
    std::thread::spawn(move || {
        while overlay_clone.is_minimized().is_ok() {
            if let Ok(true) = overlay_clone.is_visible() {
                let _ = overlay_clone.set_always_on_top(true);
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    });

    tracing::info!("Webcam overlay window created OK");
    Ok(())
}

/// Destroy the webcam overlay window
#[tauri::command]
pub fn destroy_webcam_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("webcam-overlay") {
        window.close().map_err(|e| e.to_string())?;
        tracing::info!("Webcam overlay window destroyed");
    }
    Ok(())
}


#[derive(serde::Serialize)]
pub struct WebcamDeviceInfo {
    pub index: String,
    pub name: String,
}

#[tauri::command]
pub fn get_webcam_devices() -> Result<Vec<WebcamDeviceInfo>, String> {
    use nokhwa::utils::ApiBackend;
    let devices = nokhwa::query(ApiBackend::Auto)
        .map_err(|e| format!("Failed to query webcam devices: {}", e))?;
    
    let mut list = Vec::new();
    for d in devices {
        list.push(WebcamDeviceInfo {
            index: d.index().to_string(),
            name: d.human_name(),
        });
    }
    Ok(list)
}

#[tauri::command]
pub fn get_keyboard_events() -> Vec<crate::keyboard::KeyEvent> {
    crate::keyboard::get_keyboard_events()
}

