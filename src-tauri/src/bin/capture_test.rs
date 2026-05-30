use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);
static OUTPUT_PATH: Mutex<String> = Mutex::new(String::new());

struct Capture {
    encoder: Option<VideoEncoder>,
    start: Instant,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = (i32, i32);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(_ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let output_path = OUTPUT_PATH.lock().unwrap().clone();
        let width = _ctx.flags.0 as u32;
        let height = _ctx.flags.1 as u32;

        eprintln!("Encoder: {}x{} -> {}", width, height, output_path);

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            &output_path,
        )?;

        Ok(Self {
            encoder: Some(encoder),
            start: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        let elapsed = self.start.elapsed().as_secs_f64();

        if SHOULD_STOP.load(Ordering::Relaxed) {
            if let Some(mut encoder) = self.encoder.take() {
                eprintln!("Finishing encoder...");
                encoder.finish()?;
            }
            capture_control.stop();
            return Ok(());
        }

        self.encoder.as_mut().unwrap().send_frame(frame)?;

        if count % 60 == 0 {
            eprintln!("Frame {} ({:.1}s)", count, elapsed);
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let count = FRAME_COUNT.load(Ordering::Relaxed);
        eprintln!("Capture session closed. Total frames: {}", count);
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let duration: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(5);
    let output = args.get(2).cloned().unwrap_or_else(|| {
        let mut p = std::env::temp_dir().join("easyspecy_test");
        std::fs::create_dir_all(&p).ok();
        p.push("recording.mp4");
        p.to_string_lossy().to_string()
    });

    *OUTPUT_PATH.lock().unwrap() = output.clone();

    eprintln!("EasySpecy Screen Capture Test");
    eprintln!("Output: {}", output);
    eprintln!("Duration: {}s", duration);

    let monitor = Monitor::primary()?;
    let width = monitor.width()?;
    let height = monitor.height()?;

    eprintln!("Monitor: {}x{}", width, height);
    eprintln!("Starting capture...");

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (width as i32, height as i32),
    );

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(duration));
        eprintln!("Stopping capture...");
        SHOULD_STOP.store(true, Ordering::Relaxed);
    });

    Capture::start(settings)?;

    let frame_count = FRAME_COUNT.load(Ordering::Relaxed);
    eprintln!("Done! {} frames captured -> {}", frame_count, output);

    if let Ok(metadata) = std::fs::metadata(&output) {
        eprintln!("File size: {} bytes ({:.1} MB)", metadata.len(), metadata.len() as f64 / 1_048_576.0);
    } else {
        eprintln!("WARNING: Output file not found!");
    }

    Ok(())
}
