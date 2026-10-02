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
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dir, out) = (args.get(1).expect("intermediates dir"), args.get(2).expect("out dir"));
    std::fs::create_dir_all(out).unwrap();

    use VideoEncoder::*;
    let mut cases = Vec::new();
    for &secs in &[60, 120] {
        for &fps in &[24, 30, 60] {
            for (name, enc) in [("H.264", H264), ("H.265", H265), ("Mobile", MobileShareable)] {
                cases.push(Case { name, encoder: enc, fps, secs, crop: None, paused: false });
            }
        }
        cases.push(Case { name: "AV1", encoder: AV1, fps: 30, secs, crop: None, paused: false });
        cases.push(Case { name: "VP9", encoder: VP9, fps: 30, secs, crop: None, paused: false });
        cases.push(Case { name: "H.264 region 1280x720", encoder: H264, fps: 30, secs, crop: Some("crop=1280:720:300:200"), paused: false });
        cases.push(Case { name: "H.265 region 1280x720", encoder: H265, fps: 30, secs, crop: Some("crop=1280:720:300:200"), paused: false });
        cases.push(Case { name: "H.264 + 1 pause", encoder: H264, fps: 30, secs, crop: None, paused: true });
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

        let config = AppConfig { video_encoder: c.encoder.clone(), video_quality: VideoQuality::High, fps: c.fps, ..AppConfig::default() };
        let t = Instant::now();
        let r = encode_final_with(&config, hevc, &segments, c.secs as f64 * 1000.0, Some(&audio), &output, c.crop, c.crop.is_none());
        let secs = t.elapsed().as_secs_f64();
        let mb = std::fs::metadata(&output).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
        let path = match (&r, c.crop.is_none() && (hevc || matches!(c.encoder, H264 | MobileShareable))) {
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
