//! CLI sync test — records for 5 seconds, stops, runs full verifier
//! Usage: cargo run --bin sync_test
//!
//! This binary exercises the EXACT same code path as the real app:
//! capture::start_recording → wait for arm → sleep 5s → capture::stop_recording → verify

use easyspecy_lib::capture::{self, RecordingConfig};
use easyspecy_lib::sync_verifier;
use std::time::{Duration, Instant};

fn main() {
    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("═══════════════════════════════════════════════════");
    println!("  EasySpecy SYNC TEST — CLI Verification");
    println!("═══════════════════════════════════════════════════");
    println!();

    let output_dir = std::env::temp_dir().join("easyspecy_sync_test");
    std::fs::create_dir_all(&output_dir).unwrap();
    let output_path = output_dir
        .join("sync_test.mp4")
        .to_string_lossy()
        .to_string();

    // Clean previous test file
    let _ = std::fs::remove_file(&output_path);

    let record_duration_secs = 5;
    let fps = 30;

    println!("[1/5] Starting recording...");
    println!("       Output: {}", output_path);
    println!("       Duration: {}s, FPS: {}, Audio: Both (mic+system)", record_duration_secs, fps);
    println!();

    let config = RecordingConfig {
        output_path: output_path.clone(),
        enable_audio: true,
        audio_source: "Both".to_string(),
        audio_sample_rate: 44100,
        fps,
        webcam_enabled: false,
        webcam_device: "default".to_string(),
        webcam_size: 200,
        webcam_x: 1200,
        webcam_y: 600,
        webcam_shape: "Circle".to_string(),
        webcam_border_color: "#00e88a".to_string(),
        webcam_border_width: 3,
        webcam_opacity: 1.0,
    };

    capture::start_recording(config).expect("start_recording failed");

    // Wait for capture to be armed (same as the async command does)
    println!("[2/5] Waiting for capture to arm (first video frame)...");
    let arm_start = Instant::now();
    loop {
        if capture::is_capture_ready() {
            break;
        }
        if arm_start.elapsed() > Duration::from_secs(10) {
            eprintln!("FATAL: Capture did not arm within 10 seconds!");
            std::process::exit(1);
        }
        if !capture::is_recording() {
            eprintln!("FATAL: Capture thread died before arming!");
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let arm_latency = arm_start.elapsed();
    println!("       ✓ Armed in {:.1}ms", arm_latency.as_secs_f64() * 1000.0);
    println!();

    // Record for exactly N seconds
    println!("[3/5] Recording for {}s...", record_duration_secs);
    let record_start = Instant::now();
    std::thread::sleep(Duration::from_secs(record_duration_secs));
    let actual_record_duration = record_start.elapsed();
    println!("       Actual sleep: {:.3}ms", actual_record_duration.as_secs_f64() * 1000.0);
    println!();

    // Stop recording
    println!("[4/5] Stopping recording...");
    let stop_start = Instant::now();
    let result = capture::stop_recording().expect("stop_recording failed");
    let stop_latency = stop_start.elapsed();
    println!("       ✓ Stopped in {:.1}ms", stop_latency.as_secs_f64() * 1000.0);
    println!("       Duration: {:.2}s", result.duration_secs);
    println!("       Frames: {}", result.frame_count);
    println!("       File size: {} bytes", result.file_size_bytes);
    println!("       Has audio: {}", result.has_audio);
    println!();

    // Run sync verifier
    println!("[5/5] Running SYNC VERIFIER...");
    println!("═══════════════════════════════════════════════════");
    let report = sync_verifier::verify_recording_sync(
        &result.output_path,
        Duration::from_secs_f64(result.duration_secs),
        result.frame_count,
        result.has_audio,
        fps,
    );

    match report {
        Ok(r) => {
            println!();
            println!("  ┌─────────────────────────────────────────────┐");
            println!("  │         SYNC VERIFICATION REPORT            │");
            println!("  ├─────────────────────────────────────────────┤");
            println!("  │ Video duration:    {:>10.1} ms            │", r.video_duration_ms);
            println!("  │ Audio duration:    {:>10.1} ms            │", r.audio_duration_ms);
            println!("  │ Wallclock dur:     {:>10.1} ms            │", r.wallclock_duration_ms);
            println!("  │ A/V drift:         {:>10.1} ms            │", r.av_drift_ms);
            println!("  │ Wallclock drift:   {:>10.1} ms            │", r.wallclock_drift_ms);
            println!("  │ Video start:       {:>10.3} ms            │", r.video_start_ms);
            println!("  │ Audio start:       {:>10.3} ms            │", r.audio_start_ms);
            println!("  │ Start offset:      {:>10.3} ms            │", r.start_offset_ms);
            println!("  │ Frames (actual):   {:>10}               │", r.frame_count_actual);
            println!("  │ Frames (expected): {:>10}               │", r.frame_count_expected);
            println!("  │ FPS (actual):      {:>10.1}               │", r.fps_actual);
            println!("  │ Audio/Video ratio: {:>10.4}               │", r.audio_video_ratio);
            println!("  ├─────────────────────────────────────────────┤");

            if r.passed {
                println!("  │  ✓✓✓  ALL CHECKS PASSED  ✓✓✓              │");
                println!("  │  Video, Mic, System audio are IN SYNC     │");
            } else {
                println!("  │  ✗✗✗  SYNC VERIFICATION FAILED  ✗✗✗       │");
                println!("  ├─────────────────────────────────────────────┤");
                for err in &r.errors {
                    println!("  │ ERROR: {}",  err);
                }
            }
            if !r.warnings.is_empty() {
                println!("  ├─────────────────────────────────────────────┤");
                for w in &r.warnings {
                    println!("  │ WARN: {}", w);
                }
            }
            println!("  └─────────────────────────────────────────────┘");
            println!();

            if !r.passed {
                eprintln!("SYNC TEST FAILED — see errors above");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("VERIFIER ERROR: {}", e);
            eprintln!("(FFprobe/FFmpeg may not be available)");
            std::process::exit(2);
        }
    }

    println!("Output file: {}", output_path);
    println!();
    println!("═══ SYNC TEST PASSED ═══");
}
