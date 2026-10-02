//! Post-processing benchmark: runs the app's real final pass (`encode_final_with`) over
//! prepared intermediates — what live capture would have produced — so each variant takes
//! seconds instead of a fresh 1–2 minute recording.
//!
//! Intermediates (made by the bench script from one real master capture):
//!   live_{h264|hevc}_{fps}_{secs}.mp4   live-encoder output for that fps/codec/duration
//!   live_h264_30_{secs}_partA/B.mp4     the same split in two (a pause/resume)
//!   audio_{secs}.wav                    48 kHz stereo audio
//!
//! Run from the repo root (so src-tauri/resources/ffmpeg.exe is found):
//!   cargo run --release --example bench_post -- <intermediates dir> <out dir>

use easyspecy_lib::capture::encode_final_with;
use easyspecy_lib::config::{AppConfig, VideoEncoder, VideoQuality};
use std::time::Instant;

struct Case {
    name: &'static str,
    encoder: VideoEncoder,
    fps: u32,
    secs: u32,
    crop: Option<&'static str>,
    paused: bool,
    small: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dir, out) = (args.get(1).expect("intermediates dir"), args.get(2).expect("out dir"));
    std::fs::create_dir_all(out).unwrap();

    use VideoEncoder::*;
    // 30 fps (the default) everywhere, so rows compare like for like
    let fps = 30;
    let mut cases = Vec::new();
    for &secs in &[60, 120] {
        for (name, enc, small) in [("H.264", H264, false), ("H.265", H265, false), ("H.264 smaller file", H264, true), ("H.265 smaller file", H265, true)] {
            cases.push(Case { name, encoder: enc, fps, secs, crop: None, paused: false, small });
        }
        cases.push(Case { name: "H.264 + 1 pause", encoder: H264, fps, secs, crop: None, paused: true, small: false });
        cases.push(Case { name: "H.264 region 1280x720", encoder: H264, fps, secs, crop: Some("crop=1280:720:300:200"), paused: false, small: false });
        cases.push(Case { name: "H.265 region 1280x720", encoder: H265, fps, secs, crop: Some("crop=1280:720:300:200"), paused: false, small: false });
        for (name, enc) in [("AV1", AV1), ("VP9", VP9), ("AV1 NVENC", AV1_NVENC)] {
            cases.push(Case { name, encoder: enc, fps, secs, crop: None, paused: false, small: false });
        }
    }

    println!("| Video | Encoder / path | FPS | Save time | Final size | MB/min |");
    println!("|---|---|---|---|---|---|");
    let mut csv = String::from("secs,encoder,fps,path,save_s,size_mb\n");
    for c in &cases {
        let hevc = matches!(c.encoder, H265 | H265_NVENC); // live capture makes HEVC only for H.265 targets
        let codec = if hevc { "hevc" } else { "h264" };
        let segments: Vec<String> = if c.paused {
            ["A", "B"].iter().map(|p| format!("{dir}/live_{codec}_{}_{}_part{p}.mp4", c.fps, c.secs)).collect()
        } else {
            vec![format!("{dir}/live_{codec}_{}_{}.mp4", c.fps, c.secs)]
        };
        let audio = format!("{dir}/audio_{}.wav", c.secs);
        let output = format!("{out}/{}_{}fps_{}s.mp4", c.name.replace([' ', '.', '+'], ""), c.fps, c.secs);
        let _ = std::fs::remove_file(&output);

        let config = AppConfig { video_encoder: c.encoder.clone(), video_quality: VideoQuality::High, fps: c.fps, compact_output: c.small, gpu_encoders_enabled: true, ..AppConfig::default() };
        let t = Instant::now();
        let r = encode_final_with(&config, hevc, &segments, &[], c.secs as f64 * 1000.0, Some(&audio), &output, c.crop, c.crop.is_none());
        let secs = t.elapsed().as_secs_f64();
        let mb = std::fs::metadata(&output).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
        let path = match (&r, !c.small && c.crop.is_none() && (hevc || matches!(c.encoder, H264 | MobileShareable))) {
            (Err(_), _) => "FAILED",
            (_, true) => "copy",
            _ => "re-encode",
        };
        if let Err(e) = &r { eprintln!("{} {}fps {}s failed: {}", c.name, c.fps, c.secs, &e[..e.len().min(300)]); }
        println!("| {} min | {} ({}) | {} | {:.1} s | {:.1} MB | {:.1} |", c.secs / 60, c.name, path, c.fps, secs, mb, mb / (c.secs as f64 / 60.0));
        csv += &format!("{},{},{},{},{:.2},{:.2}\n", c.secs, c.name, c.fps, path, secs, mb);
        let _ = std::fs::remove_file(&output); // keep the disk clean; size is recorded
    }
    std::fs::write(format!("{out}/results.csv"), csv).unwrap();
}
