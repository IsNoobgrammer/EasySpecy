use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

static SELECTED_REGION: Mutex<Option<CaptureRegion>> = Mutex::new(None);

pub fn set_region(region: CaptureRegion) {
    *SELECTED_REGION.lock().unwrap() = Some(region);
}

pub fn get_region() -> Option<CaptureRegion> {
    SELECTED_REGION.lock().unwrap().clone()
}

pub fn clear_region() {
    *SELECTED_REGION.lock().unwrap() = None;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub hwnd: isize,
}

/// Get all visible windows using Windows EnumWindows API
#[cfg(target_os = "windows")]
pub fn get_windows() -> Vec<WindowInfo> {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible,
    };

    static WINDOW_RESULTS: Mutex<Vec<WindowInfo>> = Mutex::new(Vec::new());

    // Clear previous results
    WINDOW_RESULTS.lock().unwrap().clear();

    unsafe extern "system" fn enum_callback(hwnd: HWND, _lparam: LPARAM) -> windows::core::BOOL {
        unsafe {
            if IsWindowVisible(hwnd).as_bool() {
                let mut text = [0u16; 512];
                let len = GetWindowTextW(hwnd, &mut text);
                if len > 0 {
                    let title = String::from_utf16_lossy(&text[..len as usize]);
                    if !title.is_empty()
                        && title != "EasySpecy"
                        && title != "Region Select"
                        && title != "Default IME"
                        && title != "MSCTFIME UI"
                        && title.len() > 1
                    {
                        let mut rect = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut rect);
                        let w = rect.right - rect.left;
                        let h = rect.bottom - rect.top;
                        if w > 80 && h > 80 {
                            WINDOW_RESULTS.lock().unwrap().push(WindowInfo {
                                title,
                                x: rect.left,
                                y: rect.top,
                                width: w,
                                height: h,
                                hwnd: hwnd.0 as isize,
                            });
                        }
                    }
                }
            }
        }
        windows::core::BOOL(1) // continue enumeration
    }

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(0));
    }

    WINDOW_RESULTS.lock().unwrap().clone()
}

#[cfg(not(target_os = "windows"))]
pub fn get_windows() -> Vec<WindowInfo> {
    Vec::new()
}
