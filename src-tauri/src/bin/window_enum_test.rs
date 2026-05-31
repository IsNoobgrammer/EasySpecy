use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible,
};
use std::sync::Mutex;

static RESULTS: Mutex<Vec<(String, i32, i32, i32, i32)>> = Mutex::new(Vec::new());

unsafe extern "system" fn callback(hwnd: HWND, _lparam: LPARAM) -> windows::core::BOOL {
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            let mut text = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut text);
            if len > 0 {
                let title = String::from_utf16_lossy(&text[..len as usize]);
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                if w > 50 && h > 50 && !title.is_empty() && title.len() > 1 {
                    RESULTS.lock().unwrap().push((title, rect.left, rect.top, w, h));
                }
            }
        }
    }
    windows::core::BOOL(1)
}

fn main() {
    println!("Enumerating windows...");
    
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(0));
    }
    
    let windows = RESULTS.lock().unwrap();
    println!("Found {} windows:", windows.len());
    for (i, (title, x, y, w, h)) in windows.iter().enumerate() {
        println!("  {}: '{}' at ({}, {}) size {}x{}", i + 1, title, x, y, w, h);
    }
}
