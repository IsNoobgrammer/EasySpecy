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

#[cfg(target_os = "windows")]
pub fn get_windows() -> Vec<WindowInfo> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindowVisible, GetWindowRect,
    };

    let mut windows = Vec::new();

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let windows_vec = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        if IsWindowVisible(hwnd).as_bool() {
            let mut text = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut text);
            if len > 0 {
                let title = String::from_utf16_lossy(&text[..len as usize]);
                if !title.is_empty() && title != "EasySpecy" {
                    let mut rect = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rect);
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 100 && h > 100 {
                        windows_vec.push(WindowInfo {
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
        BOOL(1)
    }

    unsafe {
        let parameter = LPARAM(&mut windows as *mut Vec<WindowInfo> as isize);
        let _ = EnumWindows(Some(enum_windows_callback), parameter);
    }

    windows
}

#[cfg(not(target_os = "windows"))]
pub fn get_windows() -> Vec<WindowInfo> {
    Vec::new()
}
