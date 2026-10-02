//! Encoder scan — which encoders this machine can actually run, and how fast.
//!
//! Two layers matter:
//! - **Live capture** (Media Foundation, while recording): H.264 / HEVC. If the live codec
//!   matches the chosen output, saving is a stream copy (~1 s), so this decides the default.
//! - **Re-encode** (FFmpeg, only for region/window crops or codecs the live encoder can't
//!   make, e.g. AV1/VP9): speed here is what the user waits for after stopping.
//!
//! Each FFmpeg encoder gets a 4 s 1080p30 synthetic clip; we time it and read the process's
//! peak memory. Results are cached in `<config>/encoders.json`.

use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderResult {
    /// Config enum value this maps to (what the dropdown stores)
    pub id: String,
    pub ffmpeg: String,
    pub label: String,
    pub gpu: bool,
    pub supported: bool,
    /// Encode speed in frames/s on a 1080p30 clip (≥30 = faster than real time)
    pub fps: f64,
    pub peak_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderScan {
    pub live_h264: bool,
    pub live_hevc: bool,
    pub encoders: Vec<EncoderResult>,
    /// Config enum value we suggest as default
    pub recommended: String,
    pub scanned_at: String,
}

const CANDIDATES: &[(&str, &str, &str, bool, &[&str])] = &[
    // id,           ffmpeg name,   label,                       gpu,   extra args
    // veryfast = what "Smaller file" and crops actually use
    ("H264",       "libx264",     "H.264 (CPU)",               false, &["-preset", "veryfast"]),
    ("H265",       "libx265",     "H.265 (CPU)",               false, &["-preset", "veryfast"]),
    ("AV1",        "libsvtav1",   "AV1 (CPU)",                 false, &["-preset", "10"]),
    ("VP9",        "libvpx-vp9",  "VP9 (CPU)",                 false, &["-deadline", "realtime", "-cpu-used", "8", "-row-mt", "1"]),
    ("H264_NVENC", "h264_nvenc",  "H.264 NVENC (NVIDIA GPU)",  true,  &["-preset", "p4"]),
    ("H265_NVENC", "hevc_nvenc",  "H.265 NVENC (NVIDIA GPU)",  true,  &["-preset", "p4"]),
    ("AV1_NVENC",  "av1_nvenc",   "AV1 NVENC (RTX 40+ GPU)",   true,  &["-preset", "p4"]),
];

fn cache_path() -> std::path::PathBuf {
    crate::config::AppConfig::config_path().with_file_name("encoders.json")
}

pub fn cached() -> Option<EncoderScan> {
    serde_json::from_str(&std::fs::read_to_string(cache_path()).ok()?).ok()
}

pub fn scan() -> Result<EncoderScan, String> {
    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;
    let (live_h264, live_hevc) = crate::capture::probe_live_encoders();

    let encoders: Vec<EncoderResult> = CANDIDATES
        .iter()
        .map(|&(id, name, label, gpu, extra)| {
            let (supported, fps, peak_mb) = bench(&ffmpeg, name, extra);
            tracing::info!("Encoder scan: {} supported={} {:.0} fps {:.0} MB", name, supported, fps, peak_mb);
            EncoderResult { id: id.into(), ffmpeg: name.into(), label: label.into(), gpu, supported, fps, peak_mb }
        })
        .collect();

    // Default = whatever the live encoder already produces, so saving stays a stream copy.
    // H.264 when available (plays everywhere); HEVC only if that's all the live path has.
    let recommended = if live_h264 { "H264" } else if live_hevc { "H265" } else { "H264" }.to_string();

    let scan = EncoderScan {
        live_h264,
        live_hevc,
        encoders,
        recommended,
        scanned_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&scan) {
        let _ = std::fs::write(cache_path(), json);
    }
    Ok(scan)
}

/// Encode 120 frames of 1080p30 test pattern to null; returns (works, fps, peak MB).
fn bench(ffmpeg: &str, encoder: &str, extra: &[&str]) -> (bool, f64, f64) {
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1920x1080:rate=30", "-frames:v", "120", "-c:v", encoder])
        .args(extra)
        .args(["-pix_fmt", "yuv420p", "-f", "null", "-"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let t = Instant::now();
    let Ok(mut child) = cmd.spawn() else { return (false, 0.0, 0.0) };
    crate::capture::full_speed(&child);
    // ponytail: a hung encoder would hang the scan — kill after 30 s
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if t.elapsed().as_secs() < 30 => std::thread::sleep(std::time::Duration::from_millis(20)),
            _ => { let _ = child.kill(); break None; }
        }
    };
    let secs = t.elapsed().as_secs_f64();
    let peak_mb = peak_memory_mb(&child);
    match status {
        Some(s) if s.success() => (true, 120.0 / secs.max(0.001), peak_mb),
        _ => (false, 0.0, 0.0),
    }
}

#[cfg(target_os = "windows")]
fn peak_memory_mb(child: &std::process::Child) -> f64 {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    let mut pmc = PROCESS_MEMORY_COUNTERS::default();
    // The handle stays valid after exit until `child` is dropped, and keeps the peak counters
    let ok = unsafe {
        GetProcessMemoryInfo(HANDLE(child.as_raw_handle()), &mut pmc, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32)
    };
    if ok.is_ok() { pmc.PeakWorkingSetSize as f64 / 1_048_576.0 } else { 0.0 }
}

#[cfg(not(target_os = "windows"))]
fn peak_memory_mb(_child: &std::process::Child) -> f64 { 0.0 }
