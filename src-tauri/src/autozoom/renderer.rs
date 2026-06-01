//! Zoom viewport rendering — crop and scale source frames
//!
//! Integrates with the existing postprocess pipeline by providing
//! crop+scale operations and coordinate transformation for cursor trail.

use super::camera::{CameraState, CropRect};

/// Crop a region from an RGBA frame buffer and scale to output size
/// Uses bilinear interpolation for smooth scaling
pub fn crop_and_scale(
    source: &[u8],
    source_w: u32,
    source_h: u32,
    crop: &CropRect,
    output_w: u32,
    output_h: u32,
) -> Vec<u8> {
    let out_size = (output_w * output_h * 4) as usize;
    let mut output = vec![0u8; out_size];

    if crop.width == 0 || crop.height == 0 {
        return output;
    }

    let scale_x = crop.width as f64 / output_w as f64;
    let scale_y = crop.height as f64 / output_h as f64;

    for oy in 0..output_h {
        for ox in 0..output_w {
            // Map output pixel to source coordinates
            let src_x = crop.x as f64 + ox as f64 * scale_x;
            let src_y = crop.y as f64 + oy as f64 * scale_y;

            // Bilinear interpolation
            let x0 = src_x.floor() as u32;
            let y0 = src_y.floor() as u32;
            let x1 = (x0 + 1).min(source_w - 1);
            let y1 = (y0 + 1).min(source_h - 1);

            let fx = src_x - x0 as f64;
            let fy = src_y - y0 as f64;

            // Get 4 source pixels
            let idx00 = ((y0 * source_w + x0) * 4) as usize;
            let idx10 = ((y0 * source_w + x1) * 4) as usize;
            let idx01 = ((y1 * source_w + x0) * 4) as usize;
            let idx11 = ((y1 * source_w + x1) * 4) as usize;

            let out_idx = ((oy * output_w + ox) * 4) as usize;

            if idx11 + 3 < source.len() && out_idx + 3 < output.len() {
                for c in 0..4 {
                    let v00 = source[idx00 + c] as f64;
                    let v10 = source[idx10 + c] as f64;
                    let v01 = source[idx01 + c] as f64;
                    let v11 = source[idx11 + c] as f64;

                    let v = v00 * (1.0 - fx) * (1.0 - fy)
                        + v10 * fx * (1.0 - fy)
                        + v01 * (1.0 - fx) * fy
                        + v11 * fx * fy;

                    output[out_idx + c] = v.round() as u8;
                }
            }
        }
    }

    output
}

/// Transform a screen-space coordinate to viewport space
/// Returns None if the point is outside the viewport
pub fn screen_to_viewport(
    screen_x: f32,
    screen_y: f32,
    crop: &CropRect,
    output_w: u32,
    output_h: u32,
) -> Option<(f32, f32)> {
    let local_x = screen_x - crop.x as f32;
    let local_y = screen_y - crop.y as f32;

    if local_x < 0.0 || local_x >= crop.width as f32
        || local_y < 0.0 || local_y >= crop.height as f32
    {
        return None;
    }

    let scale_x = output_w as f32 / crop.width as f32;
    let scale_y = output_h as f32 / crop.height as f32;

    Some((local_x * scale_x, local_y * scale_y))
}

/// Transform a screen-space coordinate to viewport space, clamping to edges
pub fn screen_to_viewport_clamped(
    screen_x: f32,
    screen_y: f32,
    crop: &CropRect,
    output_w: u32,
    output_h: u32,
) -> (f32, f32) {
    let local_x = (screen_x - crop.x as f32).max(0.0).min(crop.width as f32 - 1.0);
    let local_y = (screen_y - crop.y as f32).max(0.0).min(crop.height as f32 - 1.0);

    let scale_x = output_w as f32 / crop.width as f32;
    let scale_y = output_h as f32 / crop.height as f32;

    (local_x * scale_x, local_y * scale_y)
}

/// Check if a camera state represents "no zoom" (effectively 1.0x)
pub fn is_no_zoom(state: &CameraState) -> bool {
    state.zoom < 1.01
}
