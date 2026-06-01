//! Spring-physics camera animation
//!
//! Damped harmonic oscillator for smooth camera position and zoom transitions.
//! Zoom-in is faster than zoom-out (asymmetric damping).

use super::{ZoomConfig, ZoomTimeline};

/// Camera state at a specific frame
#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
}

impl CameraState {
    /// Get the crop rectangle for this camera state
    pub fn crop_rect(&self, source_w: u32, source_h: u32) -> CropRect {
        let viewport_w = source_w as f32 / self.zoom;
        let viewport_h = source_h as f32 / self.zoom;

        // Clamp center so viewport stays within source bounds
        let half_w = viewport_w / 2.0;
        let half_h = viewport_h / 2.0;
        let cx = self.center_x.max(half_w).min(source_w as f32 - half_w);
        let cy = self.center_y.max(half_h).min(source_h as f32 - half_h);

        let x = (cx - half_w).max(0.0) as u32;
        let y = (cy - half_h).max(0.0) as u32;
        let w = (viewport_w as u32).min(source_w - x);
        let h = (viewport_h as u32).min(source_h - y);

        CropRect { x, y, width: w, height: h }
    }
}

/// Crop rectangle in source frame coordinates
#[derive(Debug, Clone, Copy)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Spring-physics camera animator
pub struct SpringCamera {
    // Current state
    pos_x: f64,
    pos_y: f64,
    zoom: f64,

    // Velocity
    vel_x: f64,
    vel_y: f64,
    vel_zoom: f64,

    // Target
    target_x: f64,
    target_y: f64,
    target_zoom: f64,

    // Spring parameters
    stiffness: f64,
    damping_in: f64,   // damping for zoom-in (lower = faster)
    damping_out: f64,  // damping for zoom-out (higher = slower)

    // Screen bounds
    screen_w: f64,
    screen_h: f64,
}

impl SpringCamera {
    pub fn new(screen_w: u32, screen_h: u32, config: &ZoomConfig) -> Self {
        let center_x = screen_w as f64 / 2.0;
        let center_y = screen_h as f64 / 2.0;

        // High stiffness + high damping = fast, critically-damped response (no oscillation)
        // Target: reach target in ~200-300ms (snappy, not sluggish)
        let stiffness = 400.0 * config.zoom_speed as f64;  // Much stiffer = faster response

        Self {
            pos_x: center_x,
            pos_y: center_y,
            zoom: 1.0,
            vel_x: 0.0,
            vel_y: 0.0,
            vel_zoom: 0.0,
            target_x: center_x,
            target_y: center_y,
            target_zoom: 1.0,
            stiffness,
            damping_in: 40.0,   // critically damped — fast zoom-in, no overshoot
            damping_out: 45.0,  // slightly more damped zoom-out — gentle but still quick
            screen_w: screen_w as f64,
            screen_h: screen_h as f64,
        }
    }

    /// Set the target position and zoom level
    pub fn set_target(&mut self, x: f32, y: f32, zoom: f32) {
        self.target_x = x as f64;
        self.target_y = y as f64;
        self.target_zoom = zoom as f64;
    }

    /// Advance the spring simulation by dt seconds
    pub fn step(&mut self, dt: f64) {
        // Choose damping based on whether we're zooming in or out
        let damping = if self.target_zoom > self.zoom {
            self.damping_in
        } else {
            self.damping_out
        };

        // Position spring (x)
        let dx = self.target_x - self.pos_x;
        let force_x = self.stiffness * dx - damping * self.vel_x;
        self.vel_x += force_x * dt;
        self.pos_x += self.vel_x * dt;

        // Position spring (y)
        let dy = self.target_y - self.pos_y;
        let force_y = self.stiffness * dy - damping * self.vel_y;
        self.vel_y += force_y * dt;
        self.pos_y += self.vel_y * dt;

        // Zoom spring
        let dz = self.target_zoom - self.zoom;
        let force_z = self.stiffness * dz - damping * self.vel_zoom;
        self.vel_zoom += force_z * dt;
        self.zoom += self.vel_zoom * dt;

        // Clamp zoom to valid range
        self.zoom = self.zoom.max(1.0).min(4.0);

        // Clamp position to screen bounds
        let half_w = self.screen_w / self.zoom / 2.0;
        let half_h = self.screen_h / self.zoom / 2.0;
        self.pos_x = self.pos_x.max(half_w).min(self.screen_w - half_w);
        self.pos_y = self.pos_y.max(half_h).min(self.screen_h - half_h);
    }

    /// Get current camera state
    pub fn state(&self) -> CameraState {
        CameraState {
            center_x: self.pos_x as f32,
            center_y: self.pos_y as f32,
            zoom: self.zoom as f32,
        }
    }

    /// Check if camera has settled (velocity near zero)
    pub fn is_settled(&self) -> bool {
        self.vel_x.abs() < 0.1
            && self.vel_y.abs() < 0.1
            && self.vel_zoom.abs() < 0.001
    }
}

/// Pre-compute the entire camera timeline (one CameraState per frame)
pub fn compute_camera_timeline(
    timeline: &ZoomTimeline,
    config: &ZoomConfig,
    total_frames: u32,
    fps: u32,
) -> Vec<CameraState> {
    let mut camera = SpringCamera::new(timeline.source_width, timeline.source_height, config);
    let dt = 1.0 / fps as f64;
    let ms_per_frame = 1000.0 / fps as f64;

    let center_x = timeline.source_width as f32 / 2.0;
    let center_y = timeline.source_height as f32 / 2.0;

    let mut states = Vec::with_capacity(total_frames as usize);

    for frame_idx in 0..total_frames {
        let frame_time_ms = frame_idx as f64 * ms_per_frame;

        // Find active zoom region for this frame
        let active_region = timeline.regions.iter().find(|r| {
            frame_time_ms >= r.start_ms as f64 && frame_time_ms <= r.end_ms as f64
        });

        // Set target based on active region (or default to full screen)
        match active_region {
            Some(region) => {
                camera.set_target(region.center_x, region.center_y, region.zoom_level);
            }
            None => {
                camera.set_target(center_x, center_y, 1.0);
            }
        }

        // Step the spring simulation
        camera.step(dt);

        states.push(camera.state());
    }

    states
}
