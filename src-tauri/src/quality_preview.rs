//! "See the difference" for quality presets: show what each bitrate actually looks like on
//! the user's own screen, instead of asking them to understand "Mbps".
//!
//! Screenshot → 2 s clip that scrolls (moving text is the hardest case for screen recording)
//! → encoded with the same Media Foundation hardware encoder live capture uses, at each
//! preset's live bitrate → one native-pixel crop per preset. Plus the "Smaller file" version
//! (x264/x265 CRF), so its trade-off is visible too.

use crate::config::{AppConfig, VideoEncoder, VideoQuality};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct QualitySample {
    /// Preset id ("Low" / "Medium" / "High" / "Ultra" / "Smaller") or "Custom"
    pub id: String,
    pub kbps: u32,
    pub mb_per_min: f64,
    /// PNG data URL of a 1:1 crop from the encoded clip
    pub image: String,
}

/// Progress for the modal's bar: (fraction done, what's happening, expected seconds until the
/// next update — the bar glides over that time instead of jumping between milestones).
pub fn progress(done: f64, towards: f64, label: &str, secs: f64) {
    if let Some(app) = crate::app_handle() {
        use tauri::Emitter;
        let _ = app.emit("quality-preview-progress", (done, towards, label, secs));
    }
}

const CROP_W: u32 = 640;
const CLIP_SECS: f64 = 2.0;
const CROP_H: u32 = 360;

pub fn render(config: &AppConfig, custom_kbps: Option<u32>, screenshot_png: Vec<u8>) -> Result<Vec<QualitySample>, String> {
    let t0 = std::time::Instant::now();
    let ffmpeg = crate::capture::find_ffmpeg_pub().ok_or("FFmpeg not found")?;
    let dir = std::env::temp_dir().join("easyspecy_quality");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let shot = dir.join("screen.png");
    std::fs::write(&shot, screenshot_png).map_err(|e| e.to_string())?;
    // Only the shown 640×360 crop is encoded, at the same bits-per-pixel the full screen would get:
    // identical-looking tiles for ~1/9 of the work (the GPU encoder only runs a few sessions at once).
    let (sw, sh) = image::image_dimensions(&shot).map_err(|e| e.to_string())?;
    let area = (CROP_W * CROP_H) as f64 / (sw as f64 * sh as f64);

    let hevc = matches!(config.effective_video_encoder(), VideoEncoder::H265 | VideoEncoder::H265_NVENC);
    let mut jobs: Vec<(String, u32)> = [VideoQuality::Low, VideoQuality::Medium, VideoQuality::High, VideoQuality::Ultra]
        .into_iter()
        .map(|q| {
            let kbps = AppConfig { video_quality: q.clone(), ..config.clone() }.live_bitrate_kbps();
            (format!("{:?}", q), kbps)
        })
        .collect();
    if let Some(k) = custom_kbps {
        jobs.push(("Custom".into(), k.clamp(300, 100_000)));
    }

    // Hardware sessions take ~1.2 s each just to start and the driver starts them one at a
    // time, so all bitrates share ONE ffmpeg process (one decode, split to N encoders). The
    // x264/x265 "Smaller" tile runs AFTER: in parallel it took every core and starved MF's CPU
    // side (8.7 s together vs 4.4 + 0.4 s in sequence).
    // "Smaller file" always encodes as H.264, so sample it with H.264's quality setting
    let crf = AppConfig { video_encoder: VideoEncoder::H264, ..config.clone() }.ffmpeg_crf().to_string();
    let fps = config.fps;
    let enc = if hevc { "hevc_mf" } else { "h264_mf" };
    let clips: Vec<std::path::PathBuf> = (0..jobs.len()).map(|i| dir.join(format!("q{i}.mp4"))).collect();
    let small_clip = dir.join("small.mp4");
    progress(0.1, 0.75, "Encoding each quality level on your GPU…", 1.2 * jobs.len() as f64);
    {
        let mut a: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-loop", "1", "-framerate"].iter().map(|s| s.to_string()).collect();
        a.extend([fps.max(1).to_string(), "-t".into(), CLIP_SECS.to_string(), "-i".into(), shot.to_string_lossy().into_owned()]);
        let labels: String = (0..jobs.len()).map(|i| format!("[v{i}]")).collect();
        a.extend(["-filter_complex".into(), format!("[0:v]{},split={}{labels}", scroll_crop(), jobs.len())]);
        for (i, (_, kbps)) in jobs.iter().enumerate() {
            let k = ((*kbps as f64) * area).max(50.0) as u32;
            a.extend([format!("-map"), format!("[v{i}]"), "-pix_fmt".into(), "nv12".into(), "-c:v".into(), enc.into(),
                "-rate_control".into(), "cbr".into(), "-hw_encoding".into(), "1".into(), "-b:v".into(), format!("{k}k"),
                clips[i].to_string_lossy().into_owned()]);
        }
        if !run(&ffmpeg, &a) {
            // No MF encoder reachable from FFmpeg — x264 at constant bitrate is a close stand-in
            for (i, (_, kbps)) in jobs.iter().enumerate() {
                let k = ((*kbps as f64) * area) as u32;
                run(&ffmpeg, &clip_args(&shot, fps, &["-pix_fmt", "yuv420p", "-c:v", "libx264", "-preset", "veryfast",
                    "-b:v", &format!("{k}k"), "-maxrate", &format!("{k}k"), "-bufsize", &format!("{}k", k / 2)], &clips[i]));
            }
        }
        progress(0.75, 0.88, "Encoding the smaller-file sample…", 0.6);
        // Same encoder "Smaller file" really uses (x264 veryfast, whatever the codec setting)
        run(&ffmpeg, &clip_args(&shot, fps, &["-pix_fmt", "yuv420p", "-c:v", "libx264", "-preset", "veryfast", "-crf", &crf], &small_clip));
    }
    progress(0.88, 0.98, "Preparing previews…", 0.5);

    // Frame extraction is cheap — do all of it in parallel
    let mut out: Vec<QualitySample> = std::thread::scope(|sc| {
        let mut hs: Vec<_> = jobs.iter().enumerate().map(|(i, (id, kbps))| {
            let (ffmpeg, dir, clip) = (&ffmpeg, &dir, &clips[i]);
            sc.spawn(move || -> Result<QualitySample, String> {
                Ok(QualitySample { id: id.clone(), kbps: *kbps, mb_per_min: kbps_to_mb_min(*kbps as f64), image: frame(ffmpeg, clip, dir, i)? })
            })
        }).collect();
        let (ffmpeg, dir, clip) = (&ffmpeg, &dir, &small_clip);
        hs.push(sc.spawn(move || -> Result<QualitySample, String> {
            // measured on the crop → scale back up to what a full-screen recording would be
            let kbps = std::fs::metadata(clip).map(|m| m.len() as f64 * 8.0 / 1000.0 / CLIP_SECS / area).unwrap_or(0.0);
            Ok(QualitySample { id: "Smaller".into(), kbps: kbps as u32, mb_per_min: kbps_to_mb_min(kbps), image: frame(ffmpeg, clip, dir, 99)? })
        }));
        hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("preview thread panicked".into()))).collect::<Result<Vec<_>, String>>()
    })?;
    tracing::info!("Quality preview: {} samples in {:.1}s", out.len(), t0.elapsed().as_secs_f64());
    progress(1.0, 1.0, "Done", 0.0);
    out.sort_by_key(|q| ["Low", "Medium", "High", "Ultra", "Custom", "Smaller"].iter().position(|x| *x == q.id).unwrap_or(9));

    let _ = std::fs::remove_dir_all(&dir);
    Ok(out)
}

fn kbps_to_mb_min(kbps: f64) -> f64 {
    kbps * 60.0 / 8.0 / 1024.0
}

fn scroll_crop() -> String {
    format!("scroll=vertical=0.01,crop={CROP_W}:{CROP_H}:(iw-{CROP_W})/2:(ih-{CROP_H})/2")
}

/// The screenshot scrolling upward at capture fps, cropped to the tile window.
fn clip_args(shot: &std::path::Path, fps: u32, enc: &[&str], out: &std::path::Path) -> Vec<String> {
    let mut a: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-loop", "1", "-framerate"].iter().map(|s| s.to_string()).collect();
    a.push(fps.max(1).to_string());
    a.extend(["-i".into(), shot.to_string_lossy().into_owned(), "-t".into(), CLIP_SECS.to_string(), "-vf".into(), scroll_crop()]);
    a.extend(enc.iter().map(|s| s.to_string()));
    a.push(out.to_string_lossy().into_owned());
    a
}

/// A late frame (encoder settled) of the already-cropped clip, as a PNG data URL.
fn frame(ffmpeg: &str, clip: &std::path::Path, dir: &std::path::Path, i: usize) -> Result<String, String> {
    let png = dir.join(format!("f{i}.png"));
    run(ffmpeg, &["-y", "-hide_banner", "-loglevel", "error", "-ss", "1.5", "-i", &clip.to_string_lossy(), "-frames:v", "1", &png.to_string_lossy()]
        .iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let bytes = std::fs::read(&png).map_err(|e| format!("preview frame missing: {e}"))?;
    use base64::Engine;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn run(ffmpeg: &str, args: &[String]) -> bool {
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let Ok(mut child) = cmd.spawn() else { return false };
    crate::capture::full_speed(&child);
    child.wait().map(|s| s.success()).unwrap_or(false)
}
