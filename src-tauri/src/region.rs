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
/// Desktop origin of the monitor the region selector is covering — the selector reports
/// coordinates relative to its own window.
static SELECTOR_ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

/// A monitor's bounds in virtual-desktop physical pixels (the primary monitor starts at 0,0;
/// others can be at negative or offset coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub hmonitor: isize,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The monitor containing a desktop point (primary if the point is off every screen).
#[cfg(target_os = "windows")]
pub fn monitor_at(x: i32, y: i32) -> MonitorRect {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
    unsafe {
        let h = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(h, &mut mi);
        let r = mi.rcMonitor;
        MonitorRect { hmonitor: h.0 as isize, x: r.left, y: r.top, width: r.right - r.left, height: r.bottom - r.top }
    }
}

#[cfg(target_os = "windows")]
pub fn monitor_under_cursor() -> MonitorRect {
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = windows::Win32::Foundation::POINT::default();
    unsafe { let _ = GetCursorPos(&mut p); }
    monitor_at(p.x, p.y)
}

/// Which monitor a recording captures: the primary for full screen; for Region / Window mode
/// the one holding the centre of the selection (it used to always be the primary, so a region
/// on a second screen recorded the wrong picture).
#[cfg(target_os = "windows")]
pub fn recording_monitor() -> MonitorRect {
    let mode = crate::config::AppConfig::load().recording_mode;
    match (mode, get_region()) {
        (crate::config::RecordingMode::FullScreen, _) | (_, None) => monitor_at(0, 0),
        (_, Some(r)) => monitor_at(r.x + r.width / 2, r.y + r.height / 2),
    }
}

pub fn set_selector_origin(x: i32, y: i32) {
    *SELECTOR_ORIGIN.lock().unwrap() = (x, y);
}

pub fn selector_origin() -> (i32, i32) {
    *SELECTOR_ORIGIN.lock().unwrap()
}

pub fn set_region(region: CaptureRegion) {
    *SELECTED_REGION.lock().unwrap() = Some(region);
}

pub fn get_region() -> Option<CaptureRegion> {
    SELECTED_REGION.lock().unwrap().clone()
}

pub fn clear_region() {
    *SELECTED_REGION.lock().unwrap() = None;
}

/// Move a desktop-coordinate region into the captured monitor's own pixel space.
pub fn to_monitor_local(r: CaptureRegion, (ox, oy): (i32, i32)) -> CaptureRegion {
    CaptureRegion { x: r.x - ox, y: r.y - oy, ..r }
}

/// Clamp a region (physical px) to the captured frame and round to even numbers —
/// yuv420p encoders reject odd crop sizes, which used to fail the encode and lose the file.
/// Returns None for a region too small to be useful, or one that covers the whole frame.
pub fn sanitize(r: CaptureRegion, (fw, fh): (i32, i32)) -> Option<CaptureRegion> {
    let x0 = r.x.clamp(0, fw) & !1;
    let y0 = r.y.clamp(0, fh) & !1;
    let x1 = (r.x + r.width).clamp(0, fw);
    let y1 = (r.y + r.height).clamp(0, fh);
    let (w, h) = ((x1 - x0) & !1, (y1 - y0) & !1);
    if w < 32 || h < 32 || (w >= fw - 1 && h >= fh - 1) {
        return None;
    }
    Some(CaptureRegion { x: x0, y: y0, width: w, height: h })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(x: i32, y: i32, width: i32, height: i32) -> CaptureRegion { CaptureRegion { x, y, width, height } }

    #[test]
    fn sanitize_region() {
        let s = sanitize(r(101, 51, 301, 201), (1920, 1080)).unwrap();
        assert_eq!((s.x, s.y, s.width % 2, s.height % 2), (100, 50, 0, 0));
        assert!(s.x + s.width <= 402 && s.y + s.height <= 252);
        // off-screen parts clipped
        let s = sanitize(r(1800, 1000, 500, 500), (1920, 1080)).unwrap();
        assert_eq!((s.x + s.width, s.y + s.height), (1920, 1080));
        // negative origin clipped, whole-frame → no crop, tiny → none
        assert_eq!(sanitize(r(-10, -10, 400, 300), (1920, 1080)).unwrap().x, 0);
        assert!(sanitize(r(0, 0, 1920, 1080), (1920, 1080)).is_none());
        assert!(sanitize(r(10, 10, 20, 20), (1920, 1080)).is_none());
        // a region on a second monitor to the left (x = -1920) maps into that monitor's pixels
        let m = MonitorRect { hmonitor: 0, x: -1920, y: 0, width: 1920, height: 1080 };
        let s = sanitize(to_monitor_local(r(-1500, 100, 640, 360), (m.x, m.y)), (m.width, m.height)).unwrap();
        assert_eq!((s.x, s.y, s.width, s.height), (420, 100, 640, 360));
    }
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

/// Visible, top-level, non-cloaked windows with their *visible* bounds in physical px.
/// GetWindowRect includes Win10/11's invisible ~7px resize border, so a crop to it would
/// show a strip of whatever is behind — DWMWA_EXTENDED_FRAME_BOUNDS is the real frame.
#[cfg(target_os = "windows")]
pub fn get_windows() -> Vec<WindowInfo> {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsIconic, IsWindowVisible};

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let out = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return true.into();
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        let mut text = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut text);
        let title = String::from_utf16_lossy(&text[..len.max(0) as usize]);
        if cloaked != 0 || title.chars().count() < 2 || title.starts_with("EasySpecy")
            || matches!(title.as_str(), "Program Manager" | "Default IME" | "MSCTFIME UI")
        {
            return true.into();
        }
        let mut r = RECT::default();
        if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32).is_ok() {
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            if w > 80 && h > 80 {
                out.push(WindowInfo { title, x: r.left, y: r.top, width: w, height: h, hwnd: hwnd.0 as isize });
            }
        }
        true.into()
    }

    let mut out: Vec<WindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

#[cfg(not(target_os = "windows"))]
pub fn get_windows() -> Vec<WindowInfo> {
    Vec::new()
}

/// Bring the window being recorded to the front (restoring it if minimised).
#[cfg(target_os = "windows")]
pub fn focus_window(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
    unsafe {
        let h = HWND(hwnd as *mut _);
        let _ = ShowWindow(h, SW_RESTORE);
        let _ = SetForegroundWindow(h);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn focus_window(_hwnd: isize) {}

/// Screenshot of a monitor (physical px) as a PNG data URL — the backdrop the region
/// selector draws on, so the user sees their real desktop instead of the app.
#[cfg(target_os = "windows")]
pub fn capture_monitor_png(m: &MonitorRect) -> Result<String, String> {
    use base64::Engine;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(capture_monitor_png_bytes(m)?)))
}

/// Primary-monitor screenshot as PNG bytes (physical px).
#[cfg(target_os = "windows")]
pub fn capture_screen_png_bytes() -> Result<Vec<u8>, String> {
    capture_monitor_png_bytes(&monitor_at(0, 0))
}

/// Screenshot of one monitor as PNG bytes (physical px).
#[cfg(target_os = "windows")]
pub fn capture_monitor_png_bytes(m: &MonitorRect) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Gdi::*;
    let (w, h) = (m.width, m.height);
    let mut px = vec![0u8; (w * h * 4) as usize];
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let _ = BitBlt(mem, 0, 0, w, h, Some(screen), m.x, m.y, SRCCOPY);
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        GetDIBits(mem, bmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
    }
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2); // BGRA → RGBA
        p[3] = 255;
    }
    use image::ImageEncoder;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(&mut png, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::NoFilter)
        .write_image(&px, w as u32, h as u32, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

#[cfg(not(target_os = "windows"))]
pub fn capture_screen_png_bytes() -> Result<Vec<u8>, String> {
    Err("Not supported on this platform".into())
}
