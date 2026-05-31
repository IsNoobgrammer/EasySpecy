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

/// Capture a window thumbnail as base64 PNG using BitBlt from screen
#[cfg(target_os = "windows")]
pub fn capture_window_thumbnail(hwnd: isize, max_width: u32, max_height: u32) -> Result<String, String> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        GetDIBits, GetDC, ReleaseDC, SelectObject, StretchBlt, SetStretchBltMode,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY, HALFTONE,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    unsafe {
        let hwnd_win = HWND(hwnd as *mut _);

        let mut rect = RECT::default();
        if GetWindowRect(hwnd_win, &mut rect).is_err() {
            return Err("Failed to get window rect".to_string());
        }
        let win_w = (rect.right - rect.left) as u32;
        let win_h = (rect.bottom - rect.top) as u32;
        if win_w == 0 || win_h == 0 {
            return Err("Window has zero size".to_string());
        }

        let scale = f64::min(max_width as f64 / win_w as f64, max_height as f64 / win_h as f64).min(1.0);
        let thumb_w = ((win_w as f64 * scale) as u32).max(1);
        let thumb_h = ((win_h as f64 * scale) as u32).max(1);

        // Get screen DC (None = entire screen)
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("Failed to get screen DC".to_string());
        }

        // Create memory DC at full window size
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, win_w as i32, win_h as i32);
        let old_bm = SelectObject(hdc_mem, hbm.into());

        // Get the window's own DC and BitBlt from it (captures even if occluded)
        let hdc_window = GetDC(Some(hwnd_win));
        if !hdc_window.is_invalid() {
            let _ = BitBlt(hdc_mem, 0, 0, win_w as i32, win_h as i32, Some(hdc_window), 0, 0, SRCCOPY);
            ReleaseDC(Some(hwnd_win), hdc_window);
        } else {
            // Fallback: capture from screen position
            let _ = BitBlt(hdc_mem, 0, 0, win_w as i32, win_h as i32, Some(hdc_screen), rect.left, rect.top, SRCCOPY);
        }

        // Create thumbnail DC
        let hdc_thumb = CreateCompatibleDC(Some(hdc_screen));
        let hbm_thumb = CreateCompatibleBitmap(hdc_screen, thumb_w as i32, thumb_h as i32);
        let old_thumb = SelectObject(hdc_thumb, hbm_thumb.into());

        let _ = SetStretchBltMode(hdc_thumb, HALFTONE);
        let _ = StretchBlt(hdc_thumb, 0, 0, thumb_w as i32, thumb_h as i32, Some(hdc_mem), 0, 0, win_w as i32, win_h as i32, SRCCOPY);

        // Extract pixels
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: thumb_w as i32,
                biHeight: -(thumb_h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (thumb_w * thumb_h * 4) as usize];
        GetDIBits(hdc_thumb, hbm_thumb, 0, thumb_h, Some(pixels.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);

        // Cleanup
        SelectObject(hdc_thumb, old_thumb);
        let _ = DeleteObject(hbm_thumb.into());
        let _ = DeleteDC(hdc_thumb);
        SelectObject(hdc_mem, old_bm);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        // BGRA -> RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        // Encode PNG
        let mut png_data = Vec::new();
        png_data.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&thumb_w.to_be_bytes());
        ihdr.extend_from_slice(&thumb_h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit RGBA
        write_png_chunk(&mut png_data, b"IHDR", &ihdr);

        let mut raw = Vec::with_capacity((thumb_w * 4 + 1) as usize * thumb_h as usize);
        for y in 0..thumb_h {
            raw.push(0); // filter None
            let start = (y * thumb_w * 4) as usize;
            raw.extend_from_slice(&pixels[start..start + (thumb_w * 4) as usize]);
        }
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        write_png_chunk(&mut png_data, b"IDAT", &compressed);
        write_png_chunk(&mut png_data, b"IEND", &[]);

        use base64::Engine;
        Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png_data)))
    }
}

#[cfg(target_os = "windows")]
fn write_png_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32fast::hash(&crc_input).to_be_bytes());
}

#[cfg(not(target_os = "windows"))]
pub fn capture_window_thumbnail(_hwnd: isize, _max_width: u32, _max_height: u32) -> Result<String, String> {
    Err("Window thumbnails not supported on this platform".to_string())
}
