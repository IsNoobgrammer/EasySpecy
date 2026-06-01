//! Zoom trigger detection — heuristic analysis of recording metadata
//!
//! Analyzes cursor trail, click events, window bounds, and keyboard events
//! to produce a list of ZoomRegions.

use super::{
    KeyboardEvent, WindowBoundsEvent, ZoomConfig, ZoomRegion, ZoomTrigger,
};
use crate::postprocess::{ClickEvent, CursorSample};

/// Detect all zoom regions from recording metadata
pub fn detect_zoom_regions(
    cursor_trail: &[CursorSample],
    click_events: &[ClickEvent],
    window_events: &[WindowBoundsEvent],
    keyboard_events: &[KeyboardEvent],
    config: &ZoomConfig,
    screen_width: u32,
    screen_height: u32,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();

    // Signal 1: Click clusters (primary — highest value)
    regions.extend(detect_click_clusters(click_events, config, screen_width, screen_height));

    // Signal 2: Window dwell (zoom to focused window)
    regions.extend(detect_window_dwell(
        cursor_trail, click_events, window_events, config, screen_width, screen_height,
    ));

    // Signal 3: Typing detection (subtle zoom to input area)
    regions.extend(detect_typing(cursor_trail, keyboard_events, config));

    // Signal 4: Circle gesture detection
    regions.extend(detect_circle_gestures(cursor_trail, config, screen_width, screen_height));

    // Signal 5: Text selection / underline
    regions.extend(detect_text_selection(cursor_trail, click_events, config));

    // Merge overlapping regions and resolve conflicts
    merge_and_resolve(&mut regions, config);

    regions
}

/// Detect click clusters: 2+ clicks within 3s and 300px of each other
fn detect_click_clusters(
    clicks: &[ClickEvent],
    config: &ZoomConfig,
    screen_w: u32,
    screen_h: u32,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    if clicks.is_empty() {
        return regions;
    }

    // Sensitivity scales the time window and distance threshold
    let time_window_ms = (3000.0 / config.sensitivity.max(0.1)) as u64;
    let distance_threshold = (300.0 / config.sensitivity.max(0.1)) as f32;
    let min_clicks = if config.sensitivity > 0.7 { 2 } else { 3 };

    let mut i = 0;
    while i < clicks.len() {
        let mut cluster = vec![&clicks[i]];
        let mut j = i + 1;

        // Grow cluster: find all clicks within time window and distance
        while j < clicks.len() {
            let time_diff = clicks[j].timestamp_ms - clicks[i].timestamp_ms;
            if time_diff > time_window_ms {
                break;
            }

            // Check distance to any click in cluster
            let close_enough = cluster.iter().any(|c| {
                let dx = clicks[j].x - c.x;
                let dy = clicks[j].y - c.y;
                (dx * dx + dy * dy).sqrt() < distance_threshold
            });

            if close_enough {
                cluster.push(&clicks[j]);
            }
            j += 1;
        }

        if cluster.len() >= min_clicks {
            // Calculate bounding box of cluster
            let min_x = cluster.iter().map(|c| c.x).fold(f32::MAX, f32::min);
            let max_x = cluster.iter().map(|c| c.x).fold(f32::MIN, f32::max);
            let min_y = cluster.iter().map(|c| c.y).fold(f32::MAX, f32::min);
            let max_y = cluster.iter().map(|c| c.y).fold(f32::MIN, f32::max);

            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

            // Add 30% padding (generous — don't crop too tight)
            let bbox_w = (max_x - min_x).max(200.0) * 1.3;
            let bbox_h = (max_y - min_y).max(200.0) * 1.3;

            // Calculate zoom level — SUBTLE, capped at max_zoom (1.8x)
            let zoom_x = screen_w as f32 / bbox_w;
            let zoom_y = screen_h as f32 / bbox_h;
            let zoom_level = zoom_x.min(zoom_y).min(config.max_zoom).max(1.0);

            if zoom_level >= config.min_zoom {
                let start_ms = cluster.first().unwrap().timestamp_ms.saturating_sub(200);
                // SHORT hold — only while clicks are happening + 500ms after
                let end_ms = cluster.last().unwrap().timestamp_ms + 500;

                regions.push(ZoomRegion {
                    start_ms,
                    end_ms: end_ms.max(start_ms + config.min_duration_ms),
                    center_x,
                    center_y,
                    zoom_level,
                    trigger: ZoomTrigger::ClickCluster,
                    priority: ZoomTrigger::ClickCluster.priority(),
                });
            }

            i = j; // skip past this cluster
        } else {
            i += 1;
        }
    }

    regions
}

/// Detect window dwell: cursor stays in a window with 2+ clicks
fn detect_window_dwell(
    cursor_trail: &[CursorSample],
    clicks: &[ClickEvent],
    window_events: &[WindowBoundsEvent],
    config: &ZoomConfig,
    screen_w: u32,
    screen_h: u32,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    if window_events.is_empty() || clicks.len() < 2 {
        return regions;
    }

    // Group clicks by window (using window bounds at click time)
    // A "window session" = consecutive clicks in the same window bounds
    let mut sessions: Vec<(WindowBoundsEvent, Vec<&ClickEvent>)> = Vec::new();

    for we in window_events {
        // Find clicks that occurred near this window event's timestamp
        // and are within the window bounds
        let window_clicks: Vec<&ClickEvent> = clicks
            .iter()
            .filter(|c| {
                c.x >= we.x as f32
                    && c.x <= (we.x + we.width) as f32
                    && c.y >= we.y as f32
                    && c.y <= (we.y + we.height) as f32
            })
            .collect();

        if window_clicks.len() >= 2 {
            sessions.push((we.clone(), window_clicks));
        }
    }

    for (window, clicks_in_window) in &sessions {
        // Skip if window is nearly full-screen (>85% of screen area)
        let window_area = (window.width * window.height) as f64;
        let screen_area = (screen_w * screen_h) as f64;
        if window_area / screen_area > 0.85 {
            continue;
        }

        // Calculate zoom to fill viewport with window + 5% padding
        let padded_w = window.width as f32 * 1.1;
        let padded_h = window.height as f32 * 1.1;
        let zoom_x = screen_w as f32 / padded_w;
        let zoom_y = screen_h as f32 / padded_h;
        let zoom_level = zoom_x.min(zoom_y).min(config.max_zoom).max(1.0);

        if zoom_level < config.min_zoom {
            continue;
        }

        let center_x = window.x as f32 + window.width as f32 / 2.0;
        let center_y = window.y as f32 + window.height as f32 / 2.0;

        // Use look-ahead: find when cursor first enters this window
        let first_click_ms = clicks_in_window.first().unwrap().timestamp_ms;
        let last_click_ms = clicks_in_window.last().unwrap().timestamp_ms;

        // Find cursor entry time (when cursor first enters window bounds)
        let entry_ms = cursor_trail
            .iter()
            .find(|s| {
                s.timestamp_ms <= first_click_ms
                    && s.x >= window.x as f32
                    && s.x <= (window.x + window.width) as f32
                    && s.y >= window.y as f32
                    && s.y <= (window.y + window.height) as f32
            })
            .map(|s| s.timestamp_ms)
            .unwrap_or(first_click_ms.saturating_sub(300));

        // End when last click happens + short buffer (NOT long hold)
        regions.push(ZoomRegion {
            start_ms: entry_ms,
            end_ms: last_click_ms + 800, // only 800ms after last click — responsive zoom-out
            center_x,
            center_y,
            zoom_level,
            trigger: ZoomTrigger::WindowDwell,
            priority: ZoomTrigger::WindowDwell.priority(),
        });
    }

    regions
}

/// Detect typing sessions: rapid keyboard activity with stationary cursor
fn detect_typing(
    cursor_trail: &[CursorSample],
    keyboard_events: &[KeyboardEvent],
    config: &ZoomConfig,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    if keyboard_events.len() < 3 || cursor_trail.is_empty() {
        return regions;
    }

    let mut session_start: Option<usize> = None;

    for i in 0..keyboard_events.len() {
        if session_start.is_none() {
            // Look for 3+ keystrokes within 1.5s
            let mut count = 1;
            let mut end_idx = i;
            for j in (i + 1)..keyboard_events.len() {
                if keyboard_events[j].timestamp_ms - keyboard_events[i].timestamp_ms > 1500 {
                    break;
                }
                count += 1;
                end_idx = j;
            }

            if count >= 3 {
                // Check if cursor is stationary during this period
                let start_time = keyboard_events[i].timestamp_ms;
                let cursor_at_start = cursor_trail
                    .iter()
                    .filter(|s| s.timestamp_ms >= start_time.saturating_sub(100) && s.timestamp_ms <= start_time + 100)
                    .next();

                if let Some(start_pos) = cursor_at_start {
                    let cursor_moved = cursor_trail
                        .iter()
                        .filter(|s| {
                            s.timestamp_ms >= start_time
                                && s.timestamp_ms <= keyboard_events[end_idx].timestamp_ms
                        })
                        .any(|s| {
                            let dx = s.x - start_pos.x;
                            let dy = s.y - start_pos.y;
                            (dx * dx + dy * dy).sqrt() > 20.0
                        });

                    if !cursor_moved {
                        session_start = Some(i);
                    }
                }
            }
        } else {
            // Check if session should end (>2s gap between keystrokes)
            let gap = if i > 0 {
                keyboard_events[i].timestamp_ms - keyboard_events[i - 1].timestamp_ms
            } else {
                0
            };

            if gap > 2000 || i == keyboard_events.len() - 1 {
                // End session
                let start_idx = session_start.unwrap();
                let start_ms = keyboard_events[start_idx].timestamp_ms;
                let end_ms = keyboard_events[i.saturating_sub(1).max(start_idx)].timestamp_ms;

                // Get cursor position at session start
                let cursor_pos = cursor_trail
                    .iter()
                    .filter(|s| s.timestamp_ms >= start_ms.saturating_sub(100))
                    .next();

                if let Some(pos) = cursor_pos {
                    if end_ms - start_ms >= config.min_duration_ms {
                        regions.push(ZoomRegion {
                            start_ms: start_ms.saturating_sub(200),
                            end_ms: end_ms + 500,
                            center_x: pos.x,
                            center_y: pos.y,
                            zoom_level: 1.25,
                            trigger: ZoomTrigger::Typing,
                            priority: ZoomTrigger::Typing.priority(),
                        });
                    }
                }

                session_start = None;
            }
        }
    }

    regions
}

/// Detect circle gestures: closed cursor loops
fn detect_circle_gestures(
    cursor_trail: &[CursorSample],
    config: &ZoomConfig,
    screen_w: u32,
    screen_h: u32,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    if cursor_trail.len() < 20 {
        return regions;
    }

    // Sliding window: look for closed loops within 3 seconds
    let window_ms = 3000u64;

    let mut i = 0;
    while i < cursor_trail.len() {
        let start = &cursor_trail[i];

        // Find the end of the time window
        let mut j = i + 1;
        while j < cursor_trail.len()
            && cursor_trail[j].timestamp_ms - start.timestamp_ms < window_ms
        {
            j += 1;
        }

        // Check if any point in [i+10..j] is close to start (closed loop)
        let min_points = 15; // need enough points for a real circle
        if j - i >= min_points {
            for k in (i + min_points)..j {
                let end = &cursor_trail[k];
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let close_distance = (dx * dx + dy * dy).sqrt();

                if close_distance < 50.0 {
                    // Potential closed loop! Calculate path length and bounding box
                    let path_slice = &cursor_trail[i..=k];
                    let path_length: f32 = path_slice
                        .windows(2)
                        .map(|w| {
                            let ddx = w[1].x - w[0].x;
                            let ddy = w[1].y - w[0].y;
                            (ddx * ddx + ddy * ddy).sqrt()
                        })
                        .sum();

                    if path_length > 200.0 {
                        // Valid circle gesture!
                        let min_x = path_slice.iter().map(|s| s.x).fold(f32::MAX, f32::min);
                        let max_x = path_slice.iter().map(|s| s.x).fold(f32::MIN, f32::max);
                        let min_y = path_slice.iter().map(|s| s.y).fold(f32::MAX, f32::min);
                        let max_y = path_slice.iter().map(|s| s.y).fold(f32::MIN, f32::max);

                        let bbox_w = max_x - min_x;
                        let bbox_h = max_y - min_y;

                        // Skip if too small
                        if bbox_w < 50.0 || bbox_h < 50.0 {
                            i = k + 1;
                            break;
                        }

                        // Add 10% padding
                        let padded_w = bbox_w * 1.2;
                        let padded_h = bbox_h * 1.2;
                        let center_x = (min_x + max_x) / 2.0;
                        let center_y = (min_y + max_y) / 2.0;

                        let zoom_x = screen_w as f32 / padded_w;
                        let zoom_y = screen_h as f32 / padded_h;
                        let zoom_level = zoom_x.min(zoom_y).min(config.max_zoom).max(1.0);

                        if zoom_level >= config.min_zoom {
                            regions.push(ZoomRegion {
                                start_ms: start.timestamp_ms.saturating_sub(200),
                                end_ms: end.timestamp_ms + 1000, // hold 1s after circle — responsive
                                center_x,
                                center_y,
                                zoom_level,
                                trigger: ZoomTrigger::CircleGesture,
                                priority: ZoomTrigger::CircleGesture.priority(),
                            });
                        }

                        i = k + 1;
                        break;
                    }
                }
            }
        }
        i += 1;
    }

    regions
}

/// Detect text selection: horizontal cursor sweep with button held
fn detect_text_selection(
    cursor_trail: &[CursorSample],
    clicks: &[ClickEvent],
    config: &ZoomConfig,
) -> Vec<ZoomRegion> {
    let mut regions = Vec::new();
    if cursor_trail.len() < 10 || clicks.is_empty() {
        return regions;
    }

    // Look for horizontal sweeps between click-down and click-up patterns
    // Heuristic: find segments where horizontal displacement >> vertical displacement
    let window_size = 30; // samples to analyze at once

    for window in cursor_trail.windows(window_size) {
        let first = &window[0];
        let last = &window[window_size - 1];

        let dx = (last.x - first.x).abs();
        let dy = (last.y - first.y).abs();

        // Horizontal displacement > 5x vertical AND > 100px
        if dx > 100.0 && dx > dy * 5.0 {
            let duration_ms = last.timestamp_ms - first.timestamp_ms;
            if duration_ms < 50 || duration_ms > 5000 {
                continue; // too fast or too slow
            }

            let center_x = (first.x + last.x) / 2.0;
            let center_y = (first.y + last.y) / 2.0;

            regions.push(ZoomRegion {
                start_ms: first.timestamp_ms.saturating_sub(200),
                end_ms: last.timestamp_ms + config.min_duration_ms,
                center_x,
                center_y,
                zoom_level: 1.25, // subtle zoom for text selection
                trigger: ZoomTrigger::TextSelection,
                priority: ZoomTrigger::TextSelection.priority(),
            });
        }
    }

    regions
}

/// Merge overlapping regions and resolve priority conflicts
fn merge_and_resolve(regions: &mut Vec<ZoomRegion>, _config: &ZoomConfig) {
    if regions.is_empty() {
        return;
    }

    // Sort by start time
    regions.sort_by_key(|r| r.start_ms);

    // Remove overlapping lower-priority regions
    let mut resolved: Vec<ZoomRegion> = Vec::new();

    for region in regions.iter() {
        if let Some(last) = resolved.last() {
            // Check overlap
            if region.start_ms < last.end_ms {
                // Overlap! Keep higher priority
                if region.priority > last.priority {
                    // Replace last with this one
                    resolved.pop();
                    resolved.push(region.clone());
                }
                // Otherwise skip this region (lower priority)
                continue;
            }

            // Enforce minimum gap between zooms (500ms — responsive, not sluggish)
            if region.start_ms < last.end_ms + 500 {
                continue; // too close to previous zoom
            }
        }
        resolved.push(region.clone());
    }

    *regions = resolved;
}
