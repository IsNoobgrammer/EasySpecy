use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Syncer {
    pub name: String,
    pub paused: AtomicBool,
    pub should_stop: AtomicBool,
}

impl Syncer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            paused: AtomicBool::new(false),
            should_stop: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct AudioSyncers {
    pub mic: Syncer,
    pub system: Syncer,
}

#[derive(Debug)]
pub struct OverlaySyncers {
    pub webcam: Syncer,
    pub cursor: Syncer,
    pub keyboard: Syncer,
}

#[derive(Debug)]
pub struct SyncManager {
    // 3 main syncers as requested:
    // 1. Audio syncer (containing mic_audio and system_audio sub-syncers)
    pub audio: AudioSyncers,
    // 2. Video syncer
    pub video: Syncer,
    // 3. Overlay syncer (containing webcam, cursor_overlay, and keyboard_overlay sub-syncers)
    pub overlay: OverlaySyncers,

    // Global states
    pub recording_active: AtomicBool,
    pub recording_paused: AtomicBool,
    pub capture_armed: AtomicBool,
    pub capture_ready: AtomicBool,
    pub should_stop: AtomicBool,

    pub start_time: Mutex<Option<Instant>>,
    pub total_paused_duration: Mutex<Duration>,
    pub current_pause_start: Mutex<Option<Instant>>,
}

impl SyncManager {
    pub const fn new() -> Self {
        Self {
            audio: AudioSyncers {
                mic: Syncer {
                    name: String::new(), // we'll treat name as placeholder or handle it cleanly
                    paused: AtomicBool::new(false),
                    should_stop: AtomicBool::new(false),
                },
                system: Syncer {
                    name: String::new(),
                    paused: AtomicBool::new(false),
                    should_stop: AtomicBool::new(false),
                },
            },
            video: Syncer {
                name: String::new(),
                paused: AtomicBool::new(false),
                should_stop: AtomicBool::new(false),
            },
            overlay: OverlaySyncers {
                webcam: Syncer {
                    name: String::new(),
                    paused: AtomicBool::new(false),
                    should_stop: AtomicBool::new(false),
                },
                cursor: Syncer {
                    name: String::new(),
                    paused: AtomicBool::new(false),
                    should_stop: AtomicBool::new(false),
                },
                keyboard: Syncer {
                    name: String::new(),
                    paused: AtomicBool::new(false),
                    should_stop: AtomicBool::new(false),
                },
            },
            recording_active: AtomicBool::new(false),
            recording_paused: AtomicBool::new(false),
            capture_armed: AtomicBool::new(false),
            capture_ready: AtomicBool::new(false),
            should_stop: AtomicBool::new(false),

            start_time: Mutex::new(None),
            total_paused_duration: Mutex::new(Duration::from_secs(0)),
            current_pause_start: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        self.recording_active.store(true, Ordering::SeqCst);
        self.recording_paused.store(false, Ordering::SeqCst);
        self.capture_armed.store(false, Ordering::SeqCst);
        self.capture_ready.store(false, Ordering::SeqCst);
        self.should_stop.store(false, Ordering::SeqCst);
        *self.start_time.lock().unwrap() = None;
        *self.total_paused_duration.lock().unwrap() = Duration::from_secs(0);
        *self.current_pause_start.lock().unwrap() = None;

        // Initialize / reset all syncs
        self.audio.mic.paused.store(false, Ordering::SeqCst);
        self.audio.mic.should_stop.store(false, Ordering::SeqCst);
        self.audio.system.paused.store(false, Ordering::SeqCst);
        self.audio.system.should_stop.store(false, Ordering::SeqCst);

        self.video.paused.store(false, Ordering::SeqCst);
        self.video.should_stop.store(false, Ordering::SeqCst);

        self.overlay.webcam.paused.store(false, Ordering::SeqCst);
        self.overlay.webcam.should_stop.store(false, Ordering::SeqCst);
        self.overlay.cursor.paused.store(false, Ordering::SeqCst);
        self.overlay.cursor.should_stop.store(false, Ordering::SeqCst);
        self.overlay.keyboard.paused.store(false, Ordering::SeqCst);
        self.overlay.keyboard.should_stop.store(false, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        self.recording_paused.store(true, Ordering::SeqCst);
        *self.current_pause_start.lock().unwrap() = Some(Instant::now());

        // Sync and pause all threads at the same time
        self.audio.mic.paused.store(true, Ordering::SeqCst);
        self.audio.system.paused.store(true, Ordering::SeqCst);
        self.video.paused.store(true, Ordering::SeqCst);
        self.overlay.webcam.paused.store(true, Ordering::SeqCst);
        self.overlay.cursor.paused.store(true, Ordering::SeqCst);
        self.overlay.keyboard.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        if let Some(start) = self.current_pause_start.lock().unwrap().take() {
            *self.total_paused_duration.lock().unwrap() += start.elapsed();
        }
        self.recording_paused.store(false, Ordering::SeqCst);

        // Sync and resume all threads at the same time
        self.audio.mic.paused.store(false, Ordering::SeqCst);
        self.audio.system.paused.store(false, Ordering::SeqCst);
        self.video.paused.store(false, Ordering::SeqCst);
        self.overlay.webcam.paused.store(false, Ordering::SeqCst);
        self.overlay.cursor.paused.store(false, Ordering::SeqCst);
        self.overlay.keyboard.paused.store(false, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);

        // Sync and stop all threads at the same time
        self.audio.mic.should_stop.store(true, Ordering::SeqCst);
        self.audio.system.should_stop.store(true, Ordering::SeqCst);
        self.video.should_stop.store(true, Ordering::SeqCst);
        self.overlay.webcam.should_stop.store(true, Ordering::SeqCst);
        self.overlay.cursor.should_stop.store(true, Ordering::SeqCst);
        self.overlay.keyboard.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.recording_active.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.recording_paused.load(Ordering::SeqCst)
    }

    pub fn is_capture_ready(&self) -> bool {
        self.capture_ready.load(Ordering::SeqCst)
    }

    pub fn set_armed(&self) {
        self.capture_armed.store(true, Ordering::SeqCst);
        self.capture_ready.store(true, Ordering::SeqCst);
        *self.start_time.lock().unwrap() = Some(Instant::now());
    }

    pub fn get_active_recording_time(&self) -> u64 {
        let start = self.start_time.lock().unwrap();
        if let Some(s) = *start {
            let total_dur = s.elapsed();
            let paused_dur = *self.total_paused_duration.lock().unwrap();
            let current_pause = self.current_pause_start.lock().unwrap();
            let extra_pause = current_pause.map(|p| p.elapsed()).unwrap_or(Duration::from_secs(0));
            let total_paused = paused_dur + extra_pause;
            if total_dur > total_paused {
                (total_dur - total_paused).as_millis() as u64
            } else {
                0
            }
        } else {
            0
        }
    }
}
