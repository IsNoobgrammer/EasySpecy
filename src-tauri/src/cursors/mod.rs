//! Cursor customization module
//! 3 packs: macOS (bundled .cur), Posy (bundled .cur), EasySpecy (glassmorphic, generated)
//! Uses Win32 SetSystemCursor to swap during recording, restores on stop.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::*;

const OCR_NORMAL: u32 = 32512;
const OCR_IBEAM: u32 = 32513;
const OCR_WAIT: u32 = 32514;
const OCR_CROSS: u32 = 32515;
const OCR_SIZENWSE: u32 = 32642;
const OCR_SIZENESW: u32 = 32643;
const OCR_SIZEWE: u32 = 32644;
const OCR_SIZENS: u32 = 32645;
const OCR_SIZEALL: u32 = 32646;
const OCR_NO: u32 = 32648;
const OCR_HAND: u32 = 32649;

static CURSORS_APPLIED: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub is_builtin: bool,
}

pub fn list_cursor_packs() -> Vec<CursorPackInfo> {
    let mut packs = vec![
        CursorPackInfo { id: "default".into(), name: "System Default".into(), description: "No change — uses your Windows cursor".into(), author: "System".into(), is_builtin: true },
        CursorPackInfo { id: "macos".into(), name: "macOS Tahoe".into(), description: "Apple's latest cursor with shadow — clean and iconic".into(), author: "Community (MIT)".into(), is_builtin: true },
        CursorPackInfo { id: "posy".into(), name: "Posy's Improved".into(), description: "The internet's favorite cursor — crisp black with white border".into(), author: "Michiel de Boer (CC0)".into(), is_builtin: true },
        CursorPackInfo { id: "easyspecy".into(), name: "Specy's Glasses".into(), description: "Premium glassmorphic translucent cursor with brand green glow — our signature".into(), author: "EasySpecy".into(), is_builtin: true },
    ];
    // Scan user custom packs
    if let Some(dir) = get_user_cursors_dir() {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        packs.push(CursorPackInfo { id: format!("custom_{}", name), name: name.clone(), description: "User-installed pack".into(), author: "User".into(), is_builtin: false });
                    }
                }
            }
        }
    }
    packs
}

fn get_user_cursors_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("easyspecy").join("cursors"))
}

#[cfg(target_os = "windows")]
pub fn apply_cursor_pack(pack_id: &str) -> Result<(), String> {
    if pack_id == "default" { return Ok(()); }
    let cursors = match pack_id {
        "macos" | "posy" => load_bundled_pack(pack_id)?,
        "easyspecy" => generate_specy_glass()?,
        id if id.starts_with("custom_") => load_custom_pack(&id[7..])?,
        _ => return Err(format!("Unknown pack: {}", pack_id)),
    };
    for (id, data) in &cursors {
        apply_cur(id, data)?;
    }
    *CURSORS_APPLIED.lock().unwrap() = true;
    tracing::info!("Applied cursor pack '{}' ({} types)", pack_id, cursors.len());
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_cursor_pack(_: &str) -> Result<(), String> { Ok(()) }

#[cfg(target_os = "windows")]
pub fn restore_cursors() -> Result<(), String> {
    let mut applied = CURSORS_APPLIED.lock().unwrap();
    if !*applied { return Ok(()); }
    unsafe {
        SystemParametersInfoW(SPI_SETCURSORS, 0, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
            .map_err(|e| format!("Restore: {}", e))?;
    }
    *applied = false;
    tracing::info!("Cursors restored to system default");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn restore_cursors() -> Result<(), String> { Ok(()) }

#[cfg(target_os = "windows")]
fn apply_cur(cursor_id: &u32, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = std::env::temp_dir().join("easyspecy_cur");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let p = tmp.join(format!("{}.cur", cursor_id));
    std::fs::File::create(&p).and_then(|mut f| f.write_all(data)).map_err(|e| e.to_string())?;
    unsafe {
        let w: Vec<u16> = p.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        let h = LoadImageW(None, windows::core::PCWSTR(w.as_ptr()), IMAGE_CURSOR, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE)
            .map_err(|e| format!("LoadImage {}: {}", cursor_id, e))?;
        let c = CopyIcon(HICON(h.0)).map_err(|e| format!("CopyIcon {}: {}", cursor_id, e))?;
        SetSystemCursor(HCURSOR(c.0), SYSTEM_CURSOR_ID(*cursor_id)).map_err(|e| format!("Set {}: {}", cursor_id, e))?;
    }
    Ok(())
}

fn load_bundled_pack(pack_id: &str) -> Result<HashMap<u32, Vec<u8>>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe.parent().ok_or("no parent")?.to_path_buf();
    let candidates = [
        exe_dir.join("resources").join("cursors").join(pack_id),
        exe_dir.parent().unwrap_or(&exe_dir).join("resources").join("cursors").join(pack_id),
        PathBuf::from("src-tauri").join("resources").join("cursors").join(pack_id),
    ];
    let dir = candidates.iter().find(|p| p.exists())
        .ok_or(format!("Bundled pack '{}' not found", pack_id))?;
    let map: &[(u32, &str)] = &[
        (OCR_NORMAL,"arrow.cur"),(OCR_IBEAM,"ibeam.cur"),(OCR_HAND,"hand.cur"),
        (OCR_CROSS,"cross.cur"),(OCR_SIZEWE,"sizewe.cur"),(OCR_SIZENS,"sizens.cur"),
        (OCR_SIZENWSE,"sizenwse.cur"),(OCR_SIZENESW,"sizenesw.cur"),
        (OCR_SIZEALL,"sizeall.cur"),(OCR_NO,"no.cur"),
    ];
    let mut m = HashMap::new();
    for (id, f) in map { let p = dir.join(f); if p.exists() { if let Ok(d) = std::fs::read(&p) { m.insert(*id, d); } } }
    if m.is_empty() { return Err(format!("No files in {}", dir.display())); }
    Ok(m)
}

fn load_custom_pack(name: &str) -> Result<HashMap<u32, Vec<u8>>, String> {
    let dir = get_user_cursors_dir().ok_or("no dir")?.join(name);
    if !dir.exists() { return Err(format!("Not found: {}", dir.display())); }
    let map: &[(u32, &[&str])] = &[
        (OCR_NORMAL,&["arrow.cur","normal.cur","default.cur"]),
        (OCR_IBEAM,&["ibeam.cur","text.cur","beam.cur"]),
        (OCR_HAND,&["hand.cur","link.cur","pointer.cur"]),
        (OCR_CROSS,&["cross.cur","crosshair.cur"]),
        (OCR_SIZEWE,&["sizewe.cur","ew-resize.cur"]),
        (OCR_SIZENS,&["sizens.cur","ns-resize.cur"]),
        (OCR_SIZENWSE,&["sizenwse.cur","nwse-resize.cur"]),
        (OCR_SIZENESW,&["sizenesw.cur","nesw-resize.cur"]),
        (OCR_SIZEALL,&["sizeall.cur","move.cur"]),
        (OCR_NO,&["no.cur","not-allowed.cur"]),
    ];
    let mut m = HashMap::new();
    for (id, names) in map { for n in *names { let p = dir.join(n); if p.exists() { if let Ok(d) = std::fs::read(&p) { m.insert(*id, d); break; } } } }
    if m.is_empty() { return Err("No .cur files".into()); }
    Ok(m)
}

// ═══════════════════════════════════════════════════════════════
// SPECY GLASS — Glassmorphic cursor set
// Design: translucent body, green inner glow, white edge highlight,
// dark outline for universal contrast. Each type = distinct shape.
// ═══════════════════════════════════════════════════════════════

fn generate_specy_glass() -> Result<HashMap<u32, Vec<u8>>, String> {
    let mut m = HashMap::new();
    let sz: u32 = 32;
    m.insert(OCR_NORMAL, render_sdf_cursor(sz, 0, 0, sdf_arrow, standard_color_func));
    m.insert(OCR_IBEAM, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_ibeam, standard_color_func));
    m.insert(OCR_HAND, render_sdf_cursor(sz, 13, 4, sdf_hand, standard_color_func));
    m.insert(OCR_CROSS, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_crosshair, standard_color_func));
    m.insert(OCR_SIZEWE, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_resize_h, standard_color_func));
    m.insert(OCR_SIZENS, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_resize_v, standard_color_func));
    m.insert(OCR_SIZENWSE, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_resize_nwse, standard_color_func));
    m.insert(OCR_SIZENESW, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_resize_nesw, standard_color_func));
    m.insert(OCR_SIZEALL, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_move, standard_color_func));
    m.insert(OCR_NO, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_forbidden, forbidden_color_func));
    m.insert(OCR_WAIT, render_sdf_cursor(sz, sz / 2, sz / 2, sdf_hourglass, wait_color_func));
    Ok(m)
}

// ===============================================================
// SDF HELPER MATH FUNCTIONS
// ===============================================================

fn sdf_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let pa_x = p.0 - a.0;
    let pa_y = p.1 - a.1;
    let ba_x = b.0 - a.0;
    let ba_y = b.1 - a.1;
    let h = (pa_x * ba_x + pa_y * ba_y) / (ba_x * ba_x + ba_y * ba_y);
    let h = h.clamp(0.0, 1.0);
    let dx = pa_x - ba_x * h;
    let dy = pa_y - ba_y * h;
    (dx * dx + dy * dy).sqrt()
}

fn sdf_triangle(p: (f32, f32), p0: (f32, f32), p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let d0 = sdf_segment(p, p0, p1);
    let d1 = sdf_segment(p, p1, p2);
    let d2 = sdf_segment(p, p2, p0);
    
    let sign0 = (p1.0 - p0.0) * (p.1 - p0.1) - (p1.1 - p0.1) * (p.0 - p0.0);
    let sign1 = (p2.0 - p1.0) * (p.1 - p1.1) - (p2.1 - p1.1) * (p.0 - p1.0);
    let sign2 = (p0.0 - p2.0) * (p.1 - p2.1) - (p0.1 - p2.1) * (p.0 - p2.0);
    
    let has_neg = (sign0 < 0.0) || (sign1 < 0.0) || (sign2 < 0.0);
    let has_pos = (sign0 > 0.0) || (sign1 > 0.0) || (sign2 > 0.0);
    
    let dist = d0.min(d1).min(d2);
    if !(has_neg && has_pos) {
        -dist
    } else {
        dist
    }
}

fn sdf_circle(p: (f32, f32), center: (f32, f32), r: f32) -> f32 {
    let dx = p.0 - center.0;
    let dy = p.1 - center.1;
    (dx * dx + dy * dy).sqrt() - r
}

fn sdf_capsule(p: (f32, f32), a: (f32, f32), b: (f32, f32), r: f32) -> f32 {
    sdf_segment(p, a, b) - r
}

// ===============================================================
// SPECIFIC CURSOR SDF SHAPES
// ===============================================================

fn sdf_arrow(p: (f32, f32)) -> f32 {
    let d_head1 = sdf_triangle(p, (6.0, 4.0), (20.0, 18.0), (13.0, 17.0));
    let d_head2 = sdf_triangle(p, (6.0, 4.0), (13.0, 17.0), (6.0, 20.0));
    let d_head = d_head1.min(d_head2);
    let d_stem = sdf_capsule(p, (11.0, 15.0), (17.5, 23.5), 1.4);
    d_head.min(d_stem)
}

fn sdf_ibeam(p: (f32, f32)) -> f32 {
    let d1 = sdf_capsule(p, (16.0, 6.0), (16.0, 26.0), 1.2);
    let d2 = sdf_capsule(p, (11.0, 6.0), (21.0, 6.0), 1.0);
    let d3 = sdf_capsule(p, (11.0, 26.0), (21.0, 26.0), 1.0);
    d1.min(d2).min(d3)
}

fn sdf_hand(p: (f32, f32)) -> f32 {
    let index = sdf_capsule(p, (13.0, 4.0), (13.0, 13.0), 1.8);
    let middle = sdf_capsule(p, (17.5, 9.0), (17.5, 17.0), 1.6);
    let ring = sdf_capsule(p, (21.5, 10.0), (21.5, 17.0), 1.6);
    let pinky = sdf_capsule(p, (25.5, 11.5), (25.5, 17.0), 1.6);
    let thumb = sdf_capsule(p, (7.5, 13.0), (10.5, 17.0), 1.6);
    let palm = sdf_capsule(p, (11.0, 16.0), (22.0, 23.0), 2.2);
    index.min(middle).min(ring).min(pinky).min(thumb).min(palm)
}

fn sdf_crosshair(p: (f32, f32)) -> f32 {
    let d_ring = (sdf_circle(p, (16.0, 16.0), 4.5)).abs() - 1.2;
    let d_center = sdf_circle(p, (16.0, 16.0), 1.0);
    let d_top = sdf_capsule(p, (16.0, 4.0), (16.0, 8.0), 0.8);
    let d_bottom = sdf_capsule(p, (16.0, 24.0), (16.0, 28.0), 0.8);
    let d_left = sdf_capsule(p, (4.0, 16.0), (8.0, 16.0), 0.8);
    let d_right = sdf_capsule(p, (24.0, 16.0), (28.0, 16.0), 0.8);
    d_ring.min(d_center).min(d_top).min(d_bottom).min(d_left).min(d_right)
}

fn sdf_resize_h(p: (f32, f32)) -> f32 {
    let d_line = sdf_capsule(p, (8.0, 16.0), (24.0, 16.0), 1.2);
    let d_arrow_l = sdf_triangle(p, (3.0, 16.0), (9.0, 11.0), (9.0, 21.0));
    let d_arrow_r = sdf_triangle(p, (29.0, 16.0), (23.0, 11.0), (23.0, 21.0));
    d_line.min(d_arrow_l).min(d_arrow_r)
}

fn sdf_resize_v(p: (f32, f32)) -> f32 {
    let d_line = sdf_capsule(p, (16.0, 8.0), (16.0, 24.0), 1.2);
    let d_arrow_t = sdf_triangle(p, (16.0, 3.0), (11.0, 9.0), (21.0, 9.0));
    let d_arrow_b = sdf_triangle(p, (16.0, 29.0), (11.0, 23.0), (21.0, 23.0));
    d_line.min(d_arrow_t).min(d_arrow_b)
}

fn sdf_resize_nwse(p: (f32, f32)) -> f32 {
    let d_line = sdf_capsule(p, (8.0, 8.0), (24.0, 24.0), 1.2);
    let d_arrow_tl = sdf_triangle(p, (4.0, 4.0), (11.0, 4.0), (4.0, 11.0));
    let d_arrow_br = sdf_triangle(p, (28.0, 28.0), (21.0, 28.0), (28.0, 21.0));
    d_line.min(d_arrow_tl).min(d_arrow_br)
}

fn sdf_resize_nesw(p: (f32, f32)) -> f32 {
    let d_line = sdf_capsule(p, (24.0, 8.0), (8.0, 24.0), 1.2);
    let d_arrow_tr = sdf_triangle(p, (28.0, 4.0), (21.0, 4.0), (28.0, 12.0));
    let d_arrow_bl = sdf_triangle(p, (4.0, 28.0), (4.0, 21.0), (12.0, 28.0));
    d_line.min(d_arrow_tr).min(d_arrow_bl)
}

fn sdf_move(p: (f32, f32)) -> f32 {
    let d_vert = sdf_capsule(p, (16.0, 7.0), (16.0, 25.0), 1.0);
    let d_horiz = sdf_capsule(p, (7.0, 16.0), (25.0, 16.0), 1.0);
    let d_arrow_t = sdf_triangle(p, (16.0, 3.0), (12.0, 8.0), (20.0, 8.0));
    let d_arrow_b = sdf_triangle(p, (16.0, 29.0), (12.0, 24.0), (20.0, 24.0));
    let d_arrow_l = sdf_triangle(p, (3.0, 16.0), (8.0, 12.0), (8.0, 20.0));
    let d_arrow_r = sdf_triangle(p, (29.0, 16.0), (24.0, 12.0), (24.0, 20.0));
    let d_hub = sdf_circle(p, (16.0, 16.0), 3.5);
    d_vert.min(d_horiz).min(d_arrow_t).min(d_arrow_b).min(d_arrow_l).min(d_arrow_r).min(d_hub)
}

fn sdf_forbidden(p: (f32, f32)) -> f32 {
    let d_ring = (sdf_circle(p, (16.0, 16.0), 10.0)).abs() - 1.8;
    let d_slash = sdf_capsule(p, (9.0, 9.0), (23.0, 23.0), 1.5);
    d_ring.min(d_slash)
}

fn sdf_hourglass(p: (f32, f32)) -> f32 {
    let d_top = sdf_triangle(p, (16.0, 16.0), (7.0, 6.0), (25.0, 6.0));
    let d_bottom = sdf_triangle(p, (16.0, 16.0), (7.0, 26.0), (25.0, 26.0));
    let d_plate_t = sdf_capsule(p, (9.0, 5.0), (23.0, 5.0), 1.2);
    let d_plate_b = sdf_capsule(p, (9.0, 27.0), (23.0, 27.0), 1.2);
    d_top.min(d_bottom).min(d_plate_t).min(d_plate_b)
}

// ===============================================================
// RENDERING & COLORING & SHADOW PIPELINE
// ===============================================================

fn blend(base: (u8, u8, u8, u8), top: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    let alpha_top = top.3 as f32 / 255.0;
    let alpha_base = base.3 as f32 / 255.0;
    
    let out_a = alpha_top + alpha_base * (1.0 - alpha_top);
    if out_a == 0.0 {
        return (0, 0, 0, 0);
    }
    
    let out_r = (top.0 as f32 * alpha_top + base.0 as f32 * alpha_base * (1.0 - alpha_top)) / out_a;
    let out_g = (top.1 as f32 * alpha_top + base.1 as f32 * alpha_base * (1.0 - alpha_top)) / out_a;
    let out_b = (top.2 as f32 * alpha_top + base.2 as f32 * alpha_base * (1.0 - alpha_top)) / out_a;
    
    (
        out_r.round().min(255.0) as u8,
        out_g.round().min(255.0) as u8,
        out_b.round().min(255.0) as u8,
        (out_a * 255.0).round().min(255.0) as u8,
    )
}

fn standard_color_func(p: (f32, f32), d: f32) -> (u8, u8, u8, u8) {
    let x = p.0;
    let y = p.1;
    
    let shape_opacity = (0.5 - d).clamp(0.0, 1.0);
    if shape_opacity <= 0.0 {
        let outline_dist = (d - 0.5).abs();
        let outline_mask = (1.0 - outline_dist / 0.5).clamp(0.0, 1.0);
        if outline_mask > 0.0 {
            return (13, 15, 26, (245.0 * outline_mask) as u8);
        }
        return (0, 0, 0, 0);
    }
    
    let diag = x + y;
    let hl = (1.0 - (diag - 18.0).abs() / 4.0).clamp(0.0, 1.0) * 0.45;
    let body_r = (21.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_g = (24.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_b = (40.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_a = (140.0 + (255.0 - 140.0) * hl) as u8;
    let mut color = (body_r, body_g, body_b, (body_a as f32 * shape_opacity) as u8);
    
    let border_dist = (d + 0.5).abs();
    let border_mask = (1.0 - border_dist / 0.8).clamp(0.0, 1.0);
    if border_mask > 0.0 {
        let glow_color = (0, 232, 138, (255.0 * border_mask) as u8);
        color = blend(color, glow_color);
    }
    
    let edge_dist = d.abs();
    let is_top_left = x < 15.0 && y < 15.0;
    let edge_mask = if is_top_left {
        (1.0 - edge_dist / 0.8).clamp(0.0, 1.0) * 0.65
    } else {
        0.0
    };
    if edge_mask > 0.0 {
        let edge_color = (238, 240, 246, (255.0 * edge_mask) as u8);
        color = blend(color, edge_color);
    }
    
    let outline_dist = (d - 0.5).abs();
    let outline_mask = (1.0 - outline_dist / 0.5).clamp(0.0, 1.0);
    if outline_mask > 0.0 {
        let out_color = (13, 15, 26, (245.0 * outline_mask) as u8);
        color = blend(color, out_color);
    }
    
    color
}

fn forbidden_color_func(p: (f32, f32), d: f32) -> (u8, u8, u8, u8) {
    let x = p.0;
    let y = p.1;
    
    let shape_opacity = (0.5 - d).clamp(0.0, 1.0);
    if shape_opacity <= 0.0 {
        let outline_dist = (d - 0.5).abs();
        let outline_mask = (1.0 - outline_dist / 0.5).clamp(0.0, 1.0);
        if outline_mask > 0.0 {
            return (40, 10, 10, (245.0 * outline_mask) as u8);
        }
        return (0, 0, 0, 0);
    }
    
    let diag = x + y;
    let hl = (1.0 - (diag - 18.0).abs() / 4.0).clamp(0.0, 1.0) * 0.45;
    let body_r = (240.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_g = (64.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_b = (64.0 * (1.0 - hl) + 255.0 * hl) as u8;
    let body_a = (145.0 + (255.0 - 145.0) * hl) as u8;
    let mut color = (body_r, body_g, body_b, (body_a as f32 * shape_opacity) as u8);
    
    let border_dist = (d + 0.5).abs();
    let border_mask = (1.0 - border_dist / 0.8).clamp(0.0, 1.0);
    if border_mask > 0.0 {
        let glow_color = (255, 110, 110, (255.0 * border_mask) as u8);
        color = blend(color, glow_color);
    }
    
    let outline_dist = (d - 0.5).abs();
    let outline_mask = (1.0 - outline_dist / 0.5).clamp(0.0, 1.0);
    if outline_mask > 0.0 {
        let out_color = (40, 10, 10, (245.0 * outline_mask) as u8);
        color = blend(color, out_color);
    }
    
    color
}

fn wait_color_func(p: (f32, f32), d: f32) -> (u8, u8, u8, u8) {
    let mut color = standard_color_func(p, d);
    
    let d_sand = sdf_triangle(p, (16.0, 19.5), (10.5, 25.5), (21.5, 25.5));
    let sand_opacity = (0.5 - d_sand).clamp(0.0, 1.0);
    if sand_opacity > 0.0 {
        let sand_color = (0, 232, 138, (255.0 * sand_opacity) as u8);
        color = blend(color, sand_color);
    }
    
    let d_stream = sdf_capsule(p, (16.0, 13.0), (16.0, 19.0), 0.6);
    let stream_opacity = (0.5 - d_stream).clamp(0.0, 1.0);
    if stream_opacity > 0.0 {
        let stream_color = (0, 232, 138, (200.0 * stream_opacity) as u8);
        color = blend(color, stream_color);
    }
    
    color
}

fn render_sdf_cursor(
    sz: u32,
    hx: u32,
    hy: u32,
    sdf_func: impl Fn((f32, f32)) -> f32,
    color_func: impl Fn((f32, f32), f32) -> (u8, u8, u8, u8),
) -> Vec<u8> {
    let mut px = vec![0u8; (sz * sz * 4) as usize];
    let s = sz as i32;
    
    let mut mask = vec![0.0f32; (sz * sz) as usize];
    for y in 0..s {
        for x in 0..s {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let d = sdf_func(p);
            mask[(y * s + x) as usize] = (0.5 - d).clamp(0.0, 1.0);
        }
    }
    
    let mut shadow = vec![0.0f32; (sz * sz) as usize];
    let ox = 1;
    let oy = 2;
    for y in 0..s {
        for x in 0..s {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let sx = x - ox + dx;
                    let sy = y - oy + dy;
                    if sx >= 0 && sx < s && sy >= 0 && sy < s {
                        let weight = if dx == 0 && dy == 0 { 2.0 } else { 1.0 };
                        sum += mask[(sy * s + sx) as usize] * weight;
                        count += weight;
                    }
                }
            }
            shadow[(y * s + x) as usize] = sum / count;
        }
    }
    
    for y in 0..s {
        for x in 0..s {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let d = sdf_func(p);
            
            let sh_val = shadow[(y * s + x) as usize];
            let sh_alpha = (sh_val * 0.40 * 255.0) as u8;
            let mut pixel_color = (0, 0, 0, sh_alpha);
            
            let fg_color = color_func(p, d);
            pixel_color = blend(pixel_color, fg_color);
            
            set4(&mut px, s, x, y, pixel_color);
        }
    }
    
    cur_file(&px, sz, hx, hy)
}

// ===== Utilities =====

#[inline]
fn set4(px: &mut [u8], stride: i32, x: i32, y: i32, rgba: (u8, u8, u8, u8)) {
    if x < 0 || y < 0 || x >= stride || y >= stride { return; }
    let i = ((y * stride + x) * 4) as usize;
    px[i] = rgba.2;   // B
    px[i+1] = rgba.1; // G
    px[i+2] = rgba.0; // R
    px[i+3] = rgba.3; // A
}

fn cur_file(pixels: &[u8], sz: u32, hx: u32, hy: u32) -> Vec<u8> {
    let mask_row = ((sz+31)/32)*4;
    let img_sz = 40 + sz*sz*4 + mask_row*sz;
    let mut d = Vec::with_capacity((22 + img_sz) as usize);
    // ICO header
    d.extend_from_slice(&[0,0,2,0,1,0]);
    // Directory entry
    d.push(sz as u8); d.push(sz as u8); d.push(0); d.push(0);
    d.extend_from_slice(&(hx as u16).to_le_bytes());
    d.extend_from_slice(&(hy as u16).to_le_bytes());
    // BITMAPINFOHEADER
    d.extend_from_slice(&img_sz.to_le_bytes());
    d.extend_from_slice(&22u32.to_le_bytes());
    // BITMAPINFOHEADER
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(sz as i32).to_le_bytes());
    d.extend_from_slice(&((sz*2) as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&32u16.to_le_bytes());
    d.extend_from_slice(&[0u8;4]); // compression
    d.extend_from_slice(&(sz*sz*4 + mask_row*sz).to_le_bytes());
    d.extend_from_slice(&[0u8;16]); // ppm, colors
    // Pixels bottom-up
    for y in (0..sz).rev() {
        let s = (y*sz*4) as usize;
        d.extend_from_slice(&pixels[s..s+(sz*4) as usize]);
    }
    // AND mask (zeros = use alpha)
    d.extend(vec![0u8; (mask_row*sz) as usize]);
    d
}
