//! Keyboard overlay — captures keypresses during recording for display.
//!
//! Uses Windows WH_KEYBOARD_LL (low-level keyboard hook) to capture
//! global keypresses system-wide. Keypresses are stored with timestamps
//! relative to the recording start, then exposed to the frontend for
//! rendering as an overlay.
//!
//! Architecture:
//! - Hook runs on a dedicated thread (Windows requires message loop for LL hooks)
//! - Events stored in a lock-free ring buffer (flume channel)
//! - Frontend polls events via Tauri command
//! - Old events (>5s) are automatically pruned

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A single keypress event with timing info
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyEvent {
    /// Key display name (e.g. "Ctrl", "A", "F5", "Space")
    pub key: String,
    /// Timestamp in milliseconds since recording start
    pub timestamp_ms: f64,
    /// Duration the key was held in milliseconds (0 if just pressed)
    pub duration_ms: f64,
}

/// Keyboard capture state
struct KeyboardCapture {
    events: Arc<Mutex<VecDeque<KeyEvent>>>,
    start_time: Instant,
    running: Arc<AtomicBool>,
    _hook_thread: Option<std::thread::JoinHandle<()>>,
}

// Windows LL keyboard hook needs to be on a thread with a message loop
unsafe impl Send for KeyboardCapture {}

static KEYBOARD_CAPTURE: std::sync::OnceLock<Mutex<Option<KeyboardCapture>>> = std::sync::OnceLock::new();

/// Start capturing keyboard events.
/// Must be called when recording starts.
pub fn start_keyboard_capture() {
    let capture_mutex = KEYBOARD_CAPTURE.get_or_init(|| Mutex::new(None));
    let mut capture = capture_mutex.lock().unwrap();

    // Stop any existing capture first
    if capture.is_some() {
        *capture = None;
    }

    let events = Arc::new(Mutex::new(VecDeque::with_capacity(256)));
    let running = Arc::new(AtomicBool::new(true));
    let start_time = Instant::now();

    let events_clone = events.clone();
    let running_clone = running.clone();

    let hook_thread = std::thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(move || {
            run_keyboard_hook(events_clone, running_clone, start_time);
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

    // Extract remaining events
    let events = if let Some(cap) = capture.take() {
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
        let elapsed = cap.start_time.elapsed().as_millis() as f64;
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

/// Windows low-level keyboard hook implementation.
/// Runs a message loop on a dedicated thread.
#[cfg(target_os = "windows")]
fn run_keyboard_hook(
    events: Arc<Mutex<VecDeque<KeyEvent>>>,
    running: Arc<AtomicBool>,
    start_time: Instant,
) {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    // Shared state for the hook callback
    // We use a raw pointer because the hook callback can't capture environment
    struct HookState {
        events: Arc<Mutex<VecDeque<KeyEvent>>>,
        start_time: Instant,
        running: Arc<AtomicBool>,
    }

    static mut HOOK_STATE: Option<Box<HookState>> = None;

    unsafe {
        HOOK_STATE = Some(Box::new(HookState {
            events: events.clone(),
            start_time,
            running: running.clone(),
        }));
    }

    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
            let key_code = w_param.0 as u32;

            // Only capture keydown events (not keyup)
            if key_code == WM_KEYDOWN || key_code == WM_SYSKEYDOWN {
                if let Some(ref state) = HOOK_STATE {
                    if state.running.load(Ordering::Relaxed) {
                        let key = vk_to_name(kbd.vkCode);
                        let timestamp_ms = state.start_time.elapsed().as_millis() as f64;

                        let event = KeyEvent {
                            key,
                            timestamp_ms,
                            duration_ms: 0.0,
                        };

                        if let Ok(mut evts) = state.events.try_lock() {
                            evts.push_back(event);
                            // Cap at 512 events to prevent memory growth
                            while evts.len() > 512 {
                                evts.pop_front();
                            }
                        }
                    }
                }
            }
        }

        CallNextHookEx(None, n_code, w_param, l_param)
    }

    unsafe {
        // Install the hook
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            None,
            0,
        );

        if let Err(e) = hook {
            tracing::error!("Failed to install keyboard hook: {}", e);
            return;
        }

        tracing::info!("Keyboard hook installed");

        // Message loop — required for WH_KEYBOARD_LL to work
        let mut msg = MSG::default();
        while running.load(Ordering::Relaxed) {
            // PeekMessage with PM_REMOVE + short timeout so we can check `running`
            if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        // Unhook
        if let Ok(h) = hook {
            let _ = UnhookWindowsHookEx(h);
        }

        tracing::info!("Keyboard hook removed");
    }
}

/// Convert Windows virtual key code to human-readable name
#[cfg(target_os = "windows")]
fn vk_to_name(vk: u32) -> String {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

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
    _start_time: Instant,
) {
    tracing::warn!("Keyboard overlay not supported on this platform");
}
