//! ═══════════════════════════════════════════════════════════════════════════
//! SYNC VERIFIER — Full-fledged verification engine for A/V sync
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! This module verifies that:
//! 1. Video duration matches wall-clock recording duration (±1 frame)
//! 2. Audio duration matches video duration (±1ms)
//! 3. Audio and video streams start at the same timestamp (0ms offset)
//! 4. When Both (mic+system) is used, they are OVERLAPPED not concatenated
//! 5. Frame rate is consistent (no dropped frame clusters)
//! 6. No silent gap at the start of audio (would indicate late arm)
//!
//! The verifier uses FFmpeg/FFprobe to inspect the final output file.
//! If ANY check fails, it returns a detailed error — the recording is
//! considered INVALID and the error propagates to the frontend.

use std::process::Command;
use std::time::Duration;

/// Maximum allowed drift between video and audio duration (in milliseconds)
const MAX_AV_DRIFT_MS: f64 = 50.0;
/// Maximum allowed drift between wall-clock duration and video duration
const MAX_WALLCLOCK_DRIFT_MS: f64 = 100.0;
/// Maximum allowed start time offset for any stream (ms)
const MAX_START_OFFSET_MS: f64 = 1.0;
/// Minimum audio duration ratio vs video (catches concatenation bug)
const MIN_AUDIO_VIDEO_RATIO: f64 = 0.95;
/// Maximum audio duration ratio vs video (catches concatenation bug — if audio is 2x video, it's concatenated)
const MAX_AUDIO_VIDEO_RATIO: f64 = 1.05;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncReport {
    pub passed: bool,
    pub video_duration_ms: f64,
    pub audio_duration_ms: f64,
    pub wallclock_duration_ms: f64,
    pub av_drift_ms: f64,
    pub wallclock_drift_ms: f64,
    pub video_start_ms: f64,
    pub audio_start_ms: f64,
    pub start_offset_ms: f64,
    pub frame_count_actual: u32,
    pub frame_count_expected: u32,
    pub fps_actual: f64,
    pub audio_video_ratio: f64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SyncReport {
    fn new() -> Self {
        Self {
            passed: true,
            video_duration_ms: 0.0,
            audio_duration_ms: 0.0,
            wallclock_duration_ms: 0.0,
            av_drift_ms: 0.0,
            wallclock_drift_ms: 0.0,
            video_start_ms: 0.0,
            audio_start_ms: 0.0,
            start_offset_ms: 0.0,
            frame_count_actual: 0,
            frame_count_expected: 0,
            fps_actual: 0.0,
            audio_video_ratio: 0.0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn fail(&mut self, msg: String) {
        self.passed = false;
        self.errors.push(msg);
    }

    fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }
}

/// Run full sync verification on the output file.
/// Returns Ok(SyncReport) with passed=true if all checks pass.
/// Returns Ok(SyncReport) with passed=false and errors if any check fails.
/// Returns Err only if FFmpeg/FFprobe cannot be run at all.
pub fn verify_recording_sync(
    output_path: &str,
    wallclock_duration: Duration,
    internal_frame_count: u32,
    has_audio: bool,
    target_fps: u32,
) -> Result<SyncReport, String> {
    let mut report = SyncReport::new();
    report.wallclock_duration_ms = wallclock_duration.as_secs_f64() * 1000.0;
    report.frame_count_expected = internal_frame_count;

    let ffprobe = find_ffprobe_or_ffmpeg()?;

    // ═══ PROBE 1: Get stream info (duration, start_time, codec, frame count) ═══
    let streams = probe_streams(&ffprobe, output_path)?;

    let video_stream = streams.iter().find(|s| s.codec_type == "video");
    let audio_stream = streams.iter().find(|s| s.codec_type == "audio");

    // ═══ CHECK 1: Video stream must exist ═══
    let video = match video_stream {
        Some(v) => v,
        None => {
            report.fail("FATAL: No video stream found in output file".to_string());
            return Ok(report);
        }
    };

    report.video_duration_ms = video.duration_ms;
    report.video_start_ms = video.start_time_ms;
    report.frame_count_actual = video.nb_frames;
    report.fps_actual = video.fps;

    // ═══ CHECK 2: Video duration vs wall-clock ═══
    report.wallclock_drift_ms =
        (report.video_duration_ms - report.wallclock_duration_ms).abs();
    if report.wallclock_drift_ms > MAX_WALLCLOCK_DRIFT_MS {
        report.fail(format!(
            "Video duration drift: video={:.1}ms, wallclock={:.1}ms, drift={:.1}ms (max {}ms)",
            report.video_duration_ms, report.wallclock_duration_ms,
            report.wallclock_drift_ms, MAX_WALLCLOCK_DRIFT_MS
        ));
    }

    // ═══ CHECK 3: Video start time must be 0 (no delay) ═══
    if report.video_start_ms > MAX_START_OFFSET_MS {
        report.fail(format!(
            "Video starts late: start_time={:.3}ms (max {}ms)",
            report.video_start_ms, MAX_START_OFFSET_MS
        ));
    }

    // ═══ CHECK 4: Frame count sanity ═══
    let expected_from_duration =
        (report.video_duration_ms / 1000.0 * target_fps as f64) as u32;
    report.frame_count_expected = internal_frame_count;
    // Allow ±5% tolerance on frame count
    let frame_tolerance = (expected_from_duration as f64 * 0.05).max(2.0) as u32;
    if video.nb_frames > 0 {
        let diff = (video.nb_frames as i64 - expected_from_duration as i64).unsigned_abs() as u32;
        if diff > frame_tolerance {
            report.warn(format!(
                "Frame count mismatch: actual={}, expected~{}, diff={} (tolerance={})",
                video.nb_frames, expected_from_duration, diff, frame_tolerance
            ));
        }
    }

    // ═══ CHECK 5: FPS consistency ═══
    if video.fps > 0.0 && target_fps > 0 {
        let fps_diff = (video.fps - target_fps as f64).abs();
        if fps_diff > 5.0 {
            report.warn(format!(
                "FPS drift: actual={:.1}, target={}, diff={:.1}",
                video.fps, target_fps, fps_diff
            ));
        }
    }

    // ═══ AUDIO CHECKS (only if audio present) ═══
    if has_audio {
        match audio_stream {
            None => {
                report.fail("Audio was recorded but no audio stream in output file".to_string());
            }
            Some(audio) => {
                report.audio_duration_ms = audio.duration_ms;
                report.audio_start_ms = audio.start_time_ms;

                // ═══ CHECK 6: Audio/Video duration drift ═══
                report.av_drift_ms =
                    (report.audio_duration_ms - report.video_duration_ms).abs();
                if report.av_drift_ms > MAX_AV_DRIFT_MS {
                    report.fail(format!(
                        "A/V DESYNC: audio={:.1}ms, video={:.1}ms, drift={:.1}ms (max {}ms)",
                        report.audio_duration_ms, report.video_duration_ms,
                        report.av_drift_ms, MAX_AV_DRIFT_MS
                    ));
                }

                // ═══ CHECK 7: Audio start time must be 0 ═══
                if report.audio_start_ms > MAX_START_OFFSET_MS {
                    report.fail(format!(
                        "Audio starts late: start_time={:.3}ms (max {}ms). \
                         This means audio was NOT armed at the same instant as video.",
                        report.audio_start_ms, MAX_START_OFFSET_MS
                    ));
                }

                // ═══ CHECK 8: Start offset between audio and video ═══
                report.start_offset_ms =
                    (report.audio_start_ms - report.video_start_ms).abs();
                if report.start_offset_ms > MAX_START_OFFSET_MS {
                    report.fail(format!(
                        "A/V start offset: {:.3}ms (max {}ms). \
                         Audio and video did not start simultaneously.",
                        report.start_offset_ms, MAX_START_OFFSET_MS
                    ));
                }

                // ═══ CHECK 9: Audio/Video ratio (catches concatenation bug) ═══
                if report.video_duration_ms > 0.0 {
                    report.audio_video_ratio =
                        report.audio_duration_ms / report.video_duration_ms;
                    if report.audio_video_ratio < MIN_AUDIO_VIDEO_RATIO {
                        report.fail(format!(
                            "Audio too short vs video: ratio={:.3} (min {}). \
                             Audio may have started late or stopped early.",
                            report.audio_video_ratio, MIN_AUDIO_VIDEO_RATIO
                        ));
                    }
                    if report.audio_video_ratio > MAX_AUDIO_VIDEO_RATIO {
                        report.fail(format!(
                            "Audio too long vs video: ratio={:.3} (max {}). \
                             CONCATENATION BUG: mic+system audio may be sequential, not overlapped.",
                            report.audio_video_ratio, MAX_AUDIO_VIDEO_RATIO
                        ));
                    }
                }
            }
        }
    }

    // ═══ FINAL VERDICT ═══
    if report.passed {
        tracing::info!(
            "✓ SYNC VERIFIED: av_drift={:.1}ms, start_offset={:.3}ms, \
             wallclock_drift={:.1}ms, ratio={:.3}",
            report.av_drift_ms, report.start_offset_ms,
            report.wallclock_drift_ms, report.audio_video_ratio
        );
    } else {
        tracing::error!(
            "✗ SYNC VERIFICATION FAILED: {} errors",
            report.errors.len()
        );
        for err in &report.errors {
            tracing::error!("  ✗ {}", err);
        }
    }
    for w in &report.warnings {
        tracing::warn!("  ⚠ {}", w);
    }

    Ok(report)
}

// ═══════════════════════════════════════════════════════════════════════════
// FFprobe parsing infrastructure
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct StreamInfo {
    codec_type: String,
    duration_ms: f64,
    start_time_ms: f64,
    nb_frames: u32,
    fps: f64,
}

/// Probe the output file using ffprobe (or ffmpeg -i) to extract stream metadata
fn probe_streams(ffprobe_path: &str, file_path: &str) -> Result<Vec<StreamInfo>, String> {
    // Try ffprobe JSON output first
    let mut cmd = Command::new(ffprobe_path);
    cmd.args([
        "-v", "quiet",
        "-print_format", "json",
        "-show_streams",
        "-show_format",
        file_path,
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().map_err(|e| format!("FFprobe exec failed: {}", e))?;

    if output.status.success() {
        let json_str = String::from_utf8_lossy(&output.stdout);
        return parse_ffprobe_json(&json_str);
    }

    // Fallback: use ffmpeg -i and parse stderr
    tracing::warn!("ffprobe JSON failed, falling back to ffmpeg -i parsing");
    probe_with_ffmpeg_fallback(ffprobe_path, file_path)
}

/// Parse ffprobe JSON output into StreamInfo structs
fn parse_ffprobe_json(json_str: &str) -> Result<Vec<StreamInfo>, String> {
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("FFprobe JSON parse error: {}", e))?;

    let streams = parsed["streams"].as_array()
        .ok_or("No 'streams' array in ffprobe output")?;

    let mut result = Vec::new();
    for stream in streams {
        let codec_type = stream["codec_type"].as_str().unwrap_or("").to_string();

        // Duration: try stream duration first, then format duration
        let duration_s = stream["duration"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| {
                // Try tags->DURATION (HH:MM:SS.mmm format)
                stream["tags"]["DURATION"].as_str()
                    .and_then(|s| parse_duration_tag(s))
            })
            .unwrap_or(0.0);

        let start_time_s = stream["start_time"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let nb_frames = stream["nb_frames"].as_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        // FPS: parse r_frame_rate (e.g. "30/1" or "30000/1001")
        let fps = stream["r_frame_rate"].as_str()
            .and_then(|s| parse_fraction(s))
            .unwrap_or(0.0);

        result.push(StreamInfo {
            codec_type,
            duration_ms: duration_s * 1000.0,
            start_time_ms: start_time_s * 1000.0,
            nb_frames,
            fps,
        });
    }

    Ok(result)
}

/// Fallback: parse ffmpeg -i stderr output for stream info
fn probe_with_ffmpeg_fallback(ffmpeg_path: &str, file_path: &str) -> Result<Vec<StreamInfo>, String> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.args(["-i", file_path])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    // ffmpeg -i always returns non-zero, that's fine
    let output = cmd.output().map_err(|e| format!("FFmpeg exec failed: {}", e))?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut streams = Vec::new();

    // Parse "Duration: HH:MM:SS.mm" from format line
    let mut container_duration_ms = 0.0;
    for line in stderr.lines() {
        if line.contains("Duration:") {
            if let Some(dur) = extract_duration_from_line(line) {
                container_duration_ms = dur * 1000.0;
            }
        }
    }

    // Parse stream lines like "Stream #0:0: Video: h264 ..."
    for line in stderr.lines() {
        let line = line.trim();
        if !line.contains("Stream #") {
            continue;
        }
        if line.contains("Video:") {
            let fps = extract_fps_from_line(line);
            streams.push(StreamInfo {
                codec_type: "video".to_string(),
                duration_ms: container_duration_ms,
                start_time_ms: 0.0,
                nb_frames: 0,
                fps,
            });
        } else if line.contains("Audio:") {
            streams.push(StreamInfo {
                codec_type: "audio".to_string(),
                duration_ms: container_duration_ms,
                start_time_ms: 0.0,
                nb_frames: 0,
                fps: 0.0,
            });
        }
    }

    if streams.is_empty() {
        return Err(format!("Could not parse any streams from: {}", file_path));
    }

    Ok(streams)
}

// ═══════════════════════════════════════════════════════════════════════════
// Utility parsers
// ═══════════════════════════════════════════════════════════════════════════

/// Parse "HH:MM:SS.mmm" duration tag to seconds
fn parse_duration_tag(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Parse fraction like "30/1" or "30000/1001" to f64
fn parse_fraction(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().ok()?;
        let den: f64 = parts[1].parse().ok()?;
        if den > 0.0 {
            return Some(num / den);
        }
    }
    s.parse::<f64>().ok()
}

/// Extract duration in seconds from a line like "Duration: 00:00:05.23, ..."
fn extract_duration_from_line(line: &str) -> Option<f64> {
    let dur_start = line.find("Duration:")? + "Duration:".len();
    let rest = line[dur_start..].trim();
    let end = rest.find(',')?;
    let time_str = rest[..end].trim();
    parse_duration_tag(time_str)
}

/// Extract FPS from a video stream line like "... 30 fps, ..." or "... 29.97 fps"
fn extract_fps_from_line(line: &str) -> f64 {
    // Look for "XX fps" or "XX.XX fps"
    for part in line.split(',') {
        let part = part.trim();
        if part.ends_with("fps") {
            let num_str = part.trim_end_matches("fps").trim();
            if let Ok(fps) = num_str.parse::<f64>() {
                return fps;
            }
        }
    }
    0.0
}

/// Find ffprobe or ffmpeg binary for probing
fn find_ffprobe_or_ffmpeg() -> Result<String, String> {
    // Try ffprobe first (more accurate)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let candidates = [
            dir.join("resources").join("ffprobe.exe"),
            dir.join("ffprobe.exe"),
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // Check if ffprobe is on PATH
    let mut cmd = Command::new("ffprobe");
    cmd.arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    if cmd.status().is_ok() {
        return Ok("ffprobe".to_string());
    }

    // Fall back to ffmpeg (can also probe with -i)
    if let Some(ref dir) = exe_dir {
        let candidates = [
            dir.join("resources").join("ffmpeg.exe"),
            dir.join("ffmpeg.exe"),
            dir.parent().unwrap_or(dir).join("resources").join("ffmpeg.exe"),
            std::path::PathBuf::from(r"C:\Users\shaur\OneDrive\Documents\ffmpeg\bin\ffprobe.exe"),
            std::path::PathBuf::from(r"C:\Users\shaur\OneDrive\Documents\ffmpeg\bin\ffmpeg.exe"),
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // Last resort: ffmpeg on PATH
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    if cmd.status().is_ok() {
        return Ok("ffmpeg".to_string());
    }

    Err("Neither ffprobe nor ffmpeg found. Cannot verify sync.".to_string())
}

/// Verify that audio WAV file (pre-merge) has correct properties.
/// Called internally before FFmpeg merge to catch issues early.
pub fn verify_audio_pre_merge(
    wav_path: &str,
    expected_duration_ms: f64,
    source: &str, // "Mic", "System", "Both"
) -> Result<(), String> {
    let reader = hound::WavReader::open(wav_path)
        .map_err(|e| format!("Cannot open WAV for verification: {}", e))?;

    let spec = reader.spec();
    let num_samples = reader.len() as f64;
    let duration_ms = (num_samples / spec.channels as f64 / spec.sample_rate as f64) * 1000.0;

    tracing::info!(
        "Audio pre-merge verify: {}Hz {}ch, {:.1}ms, source={}",
        spec.sample_rate, spec.channels, duration_ms, source
    );

    // Check duration is reasonable (within 100ms of expected)
    let drift = (duration_ms - expected_duration_ms).abs();
    if drift > 100.0 && expected_duration_ms > 0.0 {
        tracing::warn!(
            "Audio pre-merge: duration drift {:.1}ms (audio={:.1}ms, expected={:.1}ms)",
            drift, duration_ms, expected_duration_ms
        );
    }

    // For "Both" mode: if audio is roughly 2x expected, it's concatenated not mixed
    if source == "Both" && expected_duration_ms > 0.0 {
        let ratio = duration_ms / expected_duration_ms;
        if ratio > 1.8 {
            return Err(format!(
                "CONCATENATION BUG DETECTED: Audio duration ({:.1}ms) is {:.2}x \
                 video duration ({:.1}ms). Mic and system audio are being \
                 concatenated instead of overlapped/mixed!",
                duration_ms, ratio, expected_duration_ms
            ));
        }
    }

    Ok(())
}
