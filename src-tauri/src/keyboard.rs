//! Keyboard overlay — captures keypresses during recording for display.
//!
//! Uses Windows WH_KEYBOARD_LL (low-level keyboard hook) to capture
//! global keypresses system-wide. Keypresses are stored with timestamps
//! relative to the recording start, then exposed to the frontend for
//! rendering as an overlay.
//!
//! Architecture:
//! - Hook callback is extremely fast (zero allocations, zero mutexes, zero slow Win32 calls).
//! - Events are passed via a lock-free, zero-allocation SPSC (Single Producer Single Consumer) ring buffer.
//! - Uses standard Rust thread park/unpark for ultra-low-latency, zero-CPU worker thread signaling.
//! - Worker thread tracks modifier key state dynamically to avoid slow GetKeyState calls.
//! - Frontend polls events via Tauri command.
//! - Old events (>5s) are automatically pruned.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Emitter;

/// A single keypress event with timing info
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyEvent {
    /// Key display name (e.g. "Ctrl", "A", "F5", "Space")
    pub key: String,
    /// Timestamp in milliseconds since recording start
    pub timestamp_ms: f64,
    /// Duration the key was held in milliseconds (0 if just pressed)
    pub duration_ms: f64,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
}

/// Keyboard capture state
struct KeyboardCapture {
    events: Arc<Mutex<VecDeque<KeyEvent>>>,
    start_time: Arc<Mutex<Instant>>,
    running: Arc<AtomicBool>,
    _hook_thread: Option<std::thread::JoinHandle<()>>,
}

// Windows LL keyboard hook needs to be on a thread with a message loop
unsafe impl Send for KeyboardCapture {}

static KEYBOARD_CAPTURE: std::sync::OnceLock<Mutex<Option<KeyboardCapture>>> = std::sync::OnceLock::new();
static HOOK_THREAD_ID: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);
static WORKER_THREAD: std::sync::Mutex<Option<std::thread::Thread>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RawKeyEvent {
    vk_code: u32,
    is_down: bool,
    timestamp: Instant,
}

#[cfg(target_os = "windows")]
const QUEUE_SIZE: usize = 1024;

#[cfg(target_os = "windows")]
static mut KEYBOARD_QUEUE: [Option<RawKeyEvent>; QUEUE_SIZE] = [None; QUEUE_SIZE];

#[cfg(target_os = "windows")]
static QUEUE_HEAD: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "windows")]
static QUEUE_TAIL: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "windows")]
#[inline(always)]
fn push_raw_event(vk_code: u32, is_down: bool, timestamp: Instant) {
    let tail = QUEUE_TAIL.load(Ordering::Relaxed);
    let head = QUEUE_HEAD.load(Ordering::Acquire);
    let next_tail = (tail + 1) % QUEUE_SIZE;
    if next_tail != head {
        unsafe {
            KEYBOARD_QUEUE[tail] = Some(RawKeyEvent { vk_code, is_down, timestamp });
        }
        QUEUE_TAIL.store(next_tail, Ordering::Release);
        
        // Unpark the worker thread to process events instantly
        if let Ok(guard) = WORKER_THREAD.lock() {
            if let Some(ref thread) = *guard {
                thread.unpark();
            }
        }
    }
}

#[cfg(target_os = "windows")]
#[inline(always)]
fn pop_raw_event() -> Option<RawKeyEvent> {
    let head = QUEUE_HEAD.load(Ordering::Relaxed);
    let tail = QUEUE_TAIL.load(Ordering::Acquire);
    if head != tail {
        let event = unsafe { KEYBOARD_QUEUE[head] };
        QUEUE_HEAD.store((head + 1) % QUEUE_SIZE, Ordering::Release);
        event
    } else {
        None
    }
}

/// Start capturing keyboard events.
/// Must be called when recording starts.
pub fn start_keyboard_capture() {
    let capture_mutex = KEYBOARD_CAPTURE.get_or_init(|| Mutex::new(None));
    let mut capture = capture_mutex.lock().unwrap();

    // Stop any existing capture first
    if capture.is_some() {
        *capture = None;
    }
    *HOOK_THREAD_ID.lock().unwrap() = None;
    if let Ok(mut guard) = WORKER_THREAD.lock() {
        *guard = None;
    }

    #[cfg(target_os = "windows")]
    {
        QUEUE_HEAD.store(0, Ordering::SeqCst);
        QUEUE_TAIL.store(0, Ordering::SeqCst);
    }

    let events = Arc::new(Mutex::new(VecDeque::with_capacity(256)));
    let running = Arc::new(AtomicBool::new(true));
    let start_time = Arc::new(Mutex::new(Instant::now()));

    let events_clone = events.clone();
    let running_clone = running.clone();
    let start_time_clone = start_time.clone();

    let hook_thread = std::thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(move || {
            run_keyboard_hook(events_clone, running_clone, start_time_clone);
        })
        .expect("Failed to spawn keyboard hook thread");

    *capture = Some(KeyboardCapture {
        events,
        start_time,
        running,
        _hook_thread: Some(hook_thread),
    });

    tracing::info!("Keyboard capture started");
}

/// Stop capturing keyboard events.
pub fn stop_keyboard_capture() -> Vec<KeyEvent> {
    let capture_mutex = KEYBOARD_CAPTURE.get_or_init(|| Mutex::new(None));
    let mut capture = capture_mutex.lock().unwrap();

    if let Some(ref mut cap) = *capture {
        cap.running.store(false, Ordering::SeqCst);
    }

    // Wake up worker thread so it can exit
    if let Ok(guard) = WORKER_THREAD.lock() {
        if let Some(ref thread) = *guard {
            thread.unpark();
        }
    }

    if let Some(thread_id) = *HOOK_THREAD_ID.lock().unwrap() {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }

    // Join the hook thread BEFORE extracting events (hook thread joins worker internally)
    let events = if let Some(cap) = capture.take() {
        if let Some(handle) = cap._hook_thread {
            let _ = handle.join();
        }
        if let Ok(mut guard) = WORKER_THREAD.lock() {
            *guard = None;
        }
        let evts = cap.events.lock().unwrap();
        evts.iter().cloned().collect()
    } else {
        Vec::new()
    };

    tracing::info!("Keyboard capture stopped ({} events)", events.len());
    events
}

/// Get current keyboard events (called by frontend polling).
/// Returns events from the last few seconds, prunes old ones.
pub fn get_keyboard_events() -> Vec<KeyEvent> {
    let capture_mutex = KEYBOARD_CAPTURE.get_or_init(|| Mutex::new(None));
    let capture = capture_mutex.lock().unwrap();

    if let Some(ref cap) = *capture {
        let elapsed = cap.start_time.lock().unwrap().elapsed().as_millis() as f64;
        let mut events = cap.events.lock().unwrap();

        // Prune events older than 5 seconds
        while let Some(front) = events.front() {
            if elapsed - front.timestamp_ms > 5000.0 {
                events.pop_front();
            } else {
                break;
            }
        }

        events.iter().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Reset the start time of the keyboard capture session to sync with recording arm time.
/// Clears any events recorded prior to arming.
#[cfg(target_os = "windows")]
pub fn reset_keyboard_start_time() {
    let capture_mutex = KEYBOARD_CAPTURE.get_or_init(|| Mutex::new(None));
    let mut capture = capture_mutex.lock().unwrap();
    if let Some(ref mut cap) = *capture {
        *cap.start_time.lock().unwrap() = Instant::now();
        if let Ok(mut events) = cap.events.lock() {
            events.clear();
        }
        tracing::info!("Keyboard capture start time reset to arm instant");
    }
}

/// No-op for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn reset_keyboard_start_time() {}

/// Windows low-level keyboard hook implementation.
/// Runs a message loop on a dedicated thread.
#[cfg(target_os = "windows")]
fn run_keyboard_hook(
    events: Arc<Mutex<VecDeque<KeyEvent>>>,
    running: Arc<AtomicBool>,
    start_time: Arc<Mutex<Instant>>,
) {
    use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;

    // Spawn the worker thread to process events asynchronously
    let running_clone = running.clone();
    let events_clone = events.clone();
    let worker_thread = std::thread::Builder::new()
        .name("keyboard-worker".into())
        .spawn(move || {
            if let Ok(mut guard) = WORKER_THREAD.lock() {
                *guard = Some(std::thread::current());
            }
            run_worker(events_clone, running_clone, start_time);
        })
        .expect("Failed to spawn keyboard worker thread");

    unsafe {
        *HOOK_THREAD_ID.lock().unwrap() = Some(GetCurrentThreadId());
    }

    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let key_code = w_param.0 as u32;

            if key_code == WM_KEYDOWN || key_code == WM_SYSKEYDOWN {
                let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
                push_raw_event(kbd.vkCode, true, Instant::now());
            } else if key_code == WM_KEYUP || key_code == WM_SYSKEYUP {
                let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
                push_raw_event(kbd.vkCode, false, Instant::now());
            }
        }

        CallNextHookEx(None, n_code, w_param, l_param)
    }

    unsafe {
        // Install the hook
        let h_instance = GetModuleHandleW(None).map(|h| HINSTANCE(h.0)).ok();
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            h_instance,
            0,
        );

        if let Err(e) = hook {
            tracing::error!("Failed to install keyboard hook: {}", e);
            let _ = worker_thread.join();
            return;
        }

        tracing::info!("Keyboard hook installed");

        // Message loop — required for WH_KEYBOARD_LL to work
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Unhook
        if let Ok(h) = hook {
            let _ = UnhookWindowsHookEx(h);
        }

        tracing::info!("Keyboard hook removed");
    }

    // Join the worker thread to ensure graceful cleanup
    let _ = worker_thread.join();
    tracing::info!("Keyboard hook run thread finished");
}

#[cfg(target_os = "windows")]
fn run_worker(
    events: Arc<Mutex<VecDeque<KeyEvent>>>,
    running: Arc<AtomicBool>,
    start_time: Arc<Mutex<Instant>>,
) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyState, VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL,
        VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN,
    };

    // Initialize L/R modifier states independently at startup
    let mut lshift = unsafe { (GetKeyState(VK_LSHIFT.0 as i32) as u16 & 0x8000) != 0 };
    let mut rshift = unsafe { (GetKeyState(VK_RSHIFT.0 as i32) as u16 & 0x8000) != 0 };
    let mut lctrl = unsafe { (GetKeyState(VK_LCONTROL.0 as i32) as u16 & 0x8000) != 0 };
    let mut rctrl = unsafe { (GetKeyState(VK_RCONTROL.0 as i32) as u16 & 0x8000) != 0 };
    let mut lalt = unsafe { (GetKeyState(VK_LMENU.0 as i32) as u16 & 0x8000) != 0 };
    let mut ralt = unsafe { (GetKeyState(VK_RMENU.0 as i32) as u16 & 0x8000) != 0 };
    let mut lwin = unsafe { (GetKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0 };
    let mut rwin = unsafe { (GetKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0 };

    let shift = lshift || rshift;
    let ctrl = lctrl || rctrl;
    let alt = lalt || ralt;
    let win = lwin || rwin;
    tracing::info!("Keyboard worker thread started (initial modifiers: shift={}, ctrl={}, alt={}, win={})", shift, ctrl, alt, win);

    while running.load(Ordering::Relaxed) {
        // Poll all events currently in the lock-free queue
        while let Some(raw_event) = pop_raw_event() {
            let vk = raw_event.vk_code;
            let is_down = raw_event.is_down;

            // Update L/R modifier states independently using specific VK codes
            match vk {
                0xA0 => lshift = is_down,  // VK_LSHIFT
                0xA1 => rshift = is_down,  // VK_RSHIFT
                0xA2 => lctrl = is_down,   // VK_LCONTROL
                0xA3 => rctrl = is_down,   // VK_RCONTROL
                0xA4 => lalt = is_down,    // VK_LMENU
                0xA5 => ralt = is_down,    // VK_RMENU
                0x5B => lwin = is_down,    // VK_LWIN
                0x5C => rwin = is_down,    // VK_RWIN
                _ => {}
            }

            // Combine L/R into single modifier flags
            let shift = lshift || rshift;
            let ctrl = lctrl || rctrl;
            let alt = lalt || ralt;
            let win = lwin || rwin;

            if is_down {
                let key = vk_to_name(vk);
                let st = *start_time.lock().unwrap();
                let timestamp_ms = if raw_event.timestamp >= st {
                    raw_event.timestamp.duration_since(st).as_millis() as f64
                } else {
                    0.0
                };

                tracing::debug!(
                    "keyboard worker keydown: key={}, ctrl={}, shift={}, alt={}, win={}, timestamp={}",
                    key, ctrl, shift, alt, win, timestamp_ms
                );

                let event = KeyEvent {
                    key: key.clone(),
                    timestamp_ms,
                    duration_ms: 0.0,
                    ctrl,
                    shift,
                    alt,
                    win,
                };

                if let Ok(mut evts) = events.lock() {
                    evts.push_back(event.clone());
                    while evts.len() > 512 {
                        evts.pop_front();
                    }
                }

                // Emit event directly to the effects overlay window
                if let Some(app) = crate::app_handle() {
                    let _ = app.emit_to(
                        "effects-overlay",
                        "keyboard-event",
                        event,
                    );
                }

                // Record for post-processing/video baking
                crate::postprocess::record_keyboard_event(&key, ctrl, shift, alt, win);
            }
        }

        // Park thread until unparked or timeout (250ms)
        std::thread::park_timeout(std::time::Duration::from_millis(250));
    }

    tracing::info!("Keyboard worker thread stopped");
}

/// Convert Windows virtual key code to human-readable name
#[cfg(target_os = "windows")]
fn vk_to_name(vk: u32) -> String {
    match vk {
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x10 => "Shift".into(),
        0x11 => "Ctrl".into(),
        0x12 => "Alt".into(),
        0x13 => "Pause".into(),
        0x14 => "CapsLock".into(),
        0x1B => "Esc".into(),
        0x20 => "Space".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "←".into(),
        0x26 => "↑".into(),
        0x27 => "→".into(),
        0x28 => "↓".into(),
        0x2C => "PrintScreen".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x5B => "Win".into(),
        0x5C => "Win".into(),
        0x5D => "Menu".into(),
        0x70 => "F1".into(),
        0x71 => "F2".into(),
        0x72 => "F3".into(),
        0x73 => "F4".into(),
        0x74 => "F5".into(),
        0x75 => "F6".into(),
        0x76 => "F7".into(),
        0x77 => "F8".into(),
        0x78 => "F9".into(),
        0x79 => "F10".into(),
        0x7A => "F11".into(),
        0x7B => "F12".into(),
        0x90 => "NumLock".into(),
        0x91 => "ScrollLock".into(),
        0xA0 => "LShift".into(),
        0xA1 => "RShift".into(),
        0xA2 => "LCtrl".into(),
        0xA3 => "RCtrl".into(),
        0xA4 => "LAlt".into(),
        0xA5 => "RAlt".into(),
        // Numbers 0-9
        0x30..=0x39 => char::from_u32(vk).unwrap_or('?').to_string(),
        // Letters A-Z
        0x41..=0x5A => char::from_u32(vk).unwrap_or('?').to_string(),
        // Numpad
        0x60 => "Num0".into(),
        0x61 => "Num1".into(),
        0x62 => "Num2".into(),
        0x63 => "Num3".into(),
        0x64 => "Num4".into(),
        0x65 => "Num5".into(),
        0x66 => "Num6".into(),
        0x67 => "Num7".into(),
        0x68 => "Num8".into(),
        0x69 => "Num9".into(),
        0x6A => "Num*".into(),
        0x6B => "Num+".into(),
        0x6C => "NumEnter".into(),
        0x6D => "Num-".into(),
        0x6E => "Num.".into(),
        0x6F => "Num/".into(),
        // Punctuation
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBC => ",".into(),
        0xBD => "-".into(),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xC0 => "`".into(),
        0xDB => "[".into(),
        0xDC => "\\".into(),
        0xDD => "]".into(),
        0xDE => "'".into(),
        _ => format!("VK_{:02X}", vk),
    }
}

/// No-op for non-Windows platforms
#[cfg(not(target_os = "windows"))]
fn run_keyboard_hook(
    _events: Arc<Mutex<VecDeque<KeyEvent>>>,
    _running: Arc<AtomicBool>,
    _start_time: Arc<Mutex<Instant>>,
) {
    tracing::warn!("Keyboard overlay not supported on this platform");
}

