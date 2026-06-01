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
pub async fn start_recording(output_path: Option<String>) -> Result<(), String> {
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
    })?;

    // Start cursor metadata collection for post-processing
    crate::postprocess::start_collection();


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
pub async fn stop_recording() -> Result<capture::RecordingResult, String> {
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
    let size = monitor.size();
    let width = size.width as f64;
    let height = size.height as f64;

    let _overlay = tauri::WebviewWindowBuilder::new(
        &app,
        "effects-overlay",
        WebviewUrl::App("/overlay.html".into()),
    )
    .title("EasySpecy Effects")
    .inner_size(width, height)
    .position(0.0, 0.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .build()
    .map_err(|e| format!("Failed to create overlay window: {}", e))?;

    tracing::info!("Effects overlay window created: {}x{}", width, height);
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
#[tauri::command]
pub fn get_audio_levels() -> (f32, f32, f32, f32, f32, f32) {
    let levels = crate::audio::get_audio_levels();
    (levels.mic_rms, levels.mic_peak, levels.mic_db, levels.sys_rms, levels.sys_peak, levels.sys_db)
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
