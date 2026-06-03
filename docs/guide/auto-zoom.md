# Auto-Zoom Guide

Complete guide to cinematic auto-zoom detection, configuration, and tuning in EasySpecy.

---

## Overview

EasySpecy's auto-zoom automatically detects meaningful moments during your recording and applies smooth, cinematic zoom effects. All processing happens in post-processing—zero performance impact during recording.

---

## Detection Methods

Auto-zoom uses 5 independent signals to detect zoom regions:

### 1. Click Clusters (Primary Signal)

Detects rapid clicks in the same area and zooms to that region.

**Detection logic:**
- 2+ clicks within 3 seconds (adjustable via sensitivity)
- Clicks within 300px of each other (adjustable via sensitivity)
- Creates zoom region centered on cluster

**Configuration:**
- Time window scales with sensitivity: `3000ms / sensitivity`
- Distance threshold scales with sensitivity: `300px / sensitivity`
- Minimum clicks: 2 (high sensitivity > 0.7) or 3 (lower sensitivity)

**Zoom level calculation:**
```rust
let zoom_x = screen_width / bbox_width;
let zoom_y = screen_height / bbox_height;
let zoom_level = min(zoom_x, zoom_y).min(max_zoom).max(1.0);
```

**Example:**
- 3 clicks in a 400×300 area → zoom to 1.5×
- 5 clicks in a 200×150 area → zoom to 2.0× (capped at max_zoom)

### 2. Window Dwell

Detects when cursor stays within a window boundary with multiple clicks.

**Detection logic:**
- Tracks window bounds changes (via `GetForegroundWindow`)
- Groups clicks by window session
- If 2+ clicks in same window → zoom to window region

**Use case:**
- Demonstrating features in a specific application window
- Switching between apps during a tutorial

### 3. Typing Detection

Subtle zoom to text input areas when you're typing.

**Detection logic:**
- Detects rapid keyboard events (5+ keys in 2 seconds)
- Finds cursor position at typing start
- Creates small zoom region around cursor

**Use case:**
- Code editors, IDE demonstrations
- Form filling, data entry tutorials

### 4. Circle Gesture

Recognizes circular cursor movements and zooms to the circled area.

**Detection logic:**
- Tracks cursor path for circular patterns
- Detects closed loops with radius > 50px
- Zooms to center of circle

**Use case:**
- Highlighting specific UI elements
- Emphasizing regions without clicking

### 5. Text Selection

Detects text selection (drag with left button) and zooms to selected text.

**Detection logic:**
- Detects mouse down + move + mouse up pattern
- Calculates selection bounding box
- Zooms to text region

**Use case:**
- Code review demonstrations
- Document editing tutorials

---

## Configuration

### Basic Settings

```toml
# Enable auto-zoom
auto_zoom_enabled = true

# Maximum zoom level (1.0 = no zoom, 2.0 = 2× magnification)
zoom_level = 1.5

# How long to hold zoom (milliseconds)
zoom_dwell_ms = 1500

# Detection sensitivity (0.0 = insensitive, 1.0 = very sensitive)
zoom_sensitivity = 0.5

# Zoom animation speed (1.0 = normal, 2.0 = fast)
zoom_speed = 1.0
```

### Sensitivity Tuning

**Low sensitivity (0.2-0.4):**
- Only obvious click clusters trigger zoom
- Fewer false positives
- Best for: Professional presentations, polished content

**Medium sensitivity (0.5-0.7):**
- Balanced detection (default)
- Catches most meaningful moments
- Best for: Tutorials, general use

**High sensitivity (0.8-1.0):**
- Aggressive detection, many zoom regions
- May produce false positives
- Best for: Fast-paced content, interactive demos

### Zoom Level

**Subtle zoom (1.2-1.5×):**
- Professional, non-distracting
- Best for: Corporate training, formal content

**Moderate zoom (1.5-1.8×):**
- Noticeable but not overwhelming
- Best for: Tutorials, YouTube content

**Strong zoom (1.8-2.0×):**
- Very noticeable, cinematic
- Best for: Social media, engaging content

### Dwell Time

**Short dwell (500-1000ms):**
- Quick zoom in/out
- Best for: Fast-paced editing, TikTok/Reels

**Medium dwell (1500-2000ms):**
- Balanced visibility (default)
- Best for: YouTube tutorials, presentations

**Long dwell (2500-4000ms):**
- Extended focus on region
- Best for: Detailed explanations, code walkthroughs

---

## Post-Processing Pipeline

Auto-zoom applies zoom effects using FFmpeg's `zoompan` filter:

### Step 1: Detect Zoom Regions

After recording stops, EasySpecy analyzes:
- Cursor trail (positions + timestamps)
- Click events (position + timestamp + type)
- Window bounds events (window rectangle + timestamp)
- Keyboard events (key + timestamp + modifiers)

### Step 2: Generate Zoom Regions

Detection heuristics produce a list of `ZoomRegion`:
```rust
struct ZoomRegion {
    start_ms: u64,        // When zoom starts
    end_ms: u64,          // When zoom ends
    center_x: f32,        // Zoom center X
    center_y: f32,        // Zoom center Y
    zoom_level: f32,      // Zoom multiplier
    trigger: ZoomTrigger, // What caused this zoom
    priority: u32,        // Priority for conflict resolution
}
```

### Step 3: Merge Overlapping Regions

If multiple zoom regions overlap in time:
- Higher priority regions win (click cluster > window dwell > typing)
- Overlapping regions are merged into a single zoom
- Conflicts are resolved by priority and temporal proximity

### Step 4: Generate FFmpeg Filter

EasySpecy generates a `zoompan` filter expression:
```
zoompan=z='if(lte(time,1.5),1.5,1)':x='if(lte(time,1.5),(iw-iw/1.5)/2,0)':y='if(lte(time,1.5),(ih-ih/1.5)/2,0)':d=1:s=1920x1080:fps=60
```

This filter:
- Zooms to 1.5× from 0-1.5 seconds
- Centers zoom on detected region
- Returns to 1.0× (no zoom) after dwell time
- Outputs at original resolution (1920×1080) and frame rate (60fps)

### Step 5: Apply Filter

FFmpeg applies the filter to the video:
```bash
ffmpeg -i input.mp4 -vf "zoompan=..." -c:v libx264 output.mp4
```

---

## FFmpeg zoompan Filter

### Filter Syntax

```
zoompan=z=EXPR:x=EXPR:y=EXPR:d=EXPR:s=SIZE:fps=FPS
```

### Expressions

EasySpecy generates expressions dynamically:

**Zoom level (z):**
```
z='if(between(time, START, END), ZOOM_LEVEL, 1)'
```

**Center X (x):**
```
x='if(between(time, START, END), (iw - iw/ZOOM_LEVEL) * CENTER_X / iw, 0)'
```

**Center Y (y):**
```
y='if(between(time, START, END), (ih - ih/ZOOM_LEVEL) * CENTER_Y / ih, 0)'
```

**Duration (d):**
- `1` = apply to every frame
- EasySpecy uses time-based control instead of frame count

### Smooth Transitions

EasySpecy adds smooth zoom in/out by using easing functions:
```rust
// Ease-in-out cubic
fn ease(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - powf(-2.0 * t + 2.0, 3.0) / 2.0
    }
}
```

This produces natural, non-jarring zoom transitions.

---

## Conflict Resolution

When multiple detection methods trigger simultaneously, EasySpecy resolves conflicts using priority:

| Trigger | Priority | Description |
|---------|----------|-------------|
| **ClickCluster** | 100 (highest) | Most intentional signal |
| **CircleGesture** | 80 | Deliberate gesture |
| **WindowDwell** | 60 | Moderate intention |
| **TextSelection** | 40 | Passive signal |
| **Typing** | 20 (lowest) | Weakest signal |

### Merge Logic

1. Sort regions by start time
2. For overlapping regions:
   - If priorities differ → higher priority wins
   - If priorities equal → merge centers, average zoom levels
3. Ensure minimum gap between regions (500ms)
4. Clamp zoom levels to `zoom_level` config value

---

## Performance Impact

### During Recording
- **Event logging**: Minimal (appending to vectors)
- **Window tracking**: Low (polling `GetForegroundWindow` every 100ms)
- **Total overhead**: < 1% CPU

### Post-Processing
- **Detection analysis**: 100-500ms (depends on recording length)
- **FFmpeg filter generation**: < 100ms
- **FFmpeg encoding**: 1-2× real-time (depends on video length and hardware)
- **Total post-processing time**: 1-3× recording duration

### Optimization Tips
- Reduce `zoom_level` for faster FFmpeg processing
- Use shorter recordings if post-processing is too slow
- Enable hardware encoding (NVENC/QuickSync) for faster output

---

## Troubleshooting

### Auto-zoom not triggering

**Check:**
1. `auto_zoom_enabled = true` in config
2. Recording has detectable signals (clicks, typing, etc.)
3. Sensitivity is appropriate for your content

**Solutions:**
- Increase `zoom_sensitivity` (try 0.7-0.8)
- Decrease `zoom_dwell_ms` if zooms are too short
- Check logs at `%APPDATA%\easyspecy\logs\easyspecy.log`

### Zoom level too strong/weak

**Adjust:**
- Decrease `zoom_level` for subtler zoom (1.2-1.5)
- Increase `zoom_level` for stronger zoom (1.8-2.0)
- Check if `zoom_level` is capped by `max_zoom` in detection

### False positives (zooming to wrong areas)

**Solutions:**
- Decrease `zoom_sensitivity` (try 0.3-0.4)
- Increase minimum click threshold (requires code change)
- Disable specific detection methods (future feature)

### Zoom transitions are jarring

**Check:**
- `zoom_speed` is not too high (keep at 1.0-1.5)
- FFmpeg is using hardware encoding (software encoding can cause stutter)
- Video frame rate is consistent (variable FPS can cause issues)

### Post-processing is too slow

**Solutions:**
- Enable hardware encoding in config (`video_encoder = "AV1_NVENC"` or `"H264_NVENC"`)
- Reduce output resolution
- Use shorter recordings
- Close other CPU-intensive apps during post-processing

---

## Advanced: Detection Heuristics

### Click Cluster Algorithm

```rust
fn detect_click_clusters(clicks: &[ClickEvent], config: &ZoomConfig) -> Vec<ZoomRegion> {
    let time_window_ms = (3000.0 / config.sensitivity.max(0.1)) as u64;
    let distance_threshold = (300.0 / config.sensitivity.max(0.1)) as f32;
    let min_clicks = if config.sensitivity > 0.7 { 2 } else { 3 };

    let mut i = 0;
    while i < clicks.len() {
        let mut cluster = vec![&clicks[i]];
        let mut j = i + 1;

        // Grow cluster
        while j < clicks.len() {
            let time_diff = clicks[j].timestamp_ms - clicks[i].timestamp_ms;
            if time_diff > time_window_ms {
                break;
            }

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
            // Create zoom region
            regions.push(ZoomRegion { ... });
            i = j; // Skip past this cluster
        } else {
            i += 1;
        }
    }

    regions
}
```

### Circle Gesture Detection

```rust
fn detect_circle_gestures(trail: &[CursorSample], config: &ZoomConfig) -> Vec<ZoomRegion> {
    // Track cursor path segments
    // Detect closed loops with radius > 50px
    // Calculate center of circle
    // Create zoom region at center
}
```

---

## Platform Support

### Windows (Full Support)
- ✅ All detection methods
- ✅ Window bounds tracking
- ✅ Keyboard event detection
- ✅ Hardware-accelerated encoding

### macOS (Not Yet Implemented)
- ❌ Auto-zoom detection (not yet implemented)
- ❌ Window bounds tracking (requires different API)
- ❌ Keyboard event detection (not yet implemented)

### Linux (Not Yet Implemented)
- ❌ Auto-zoom detection (not yet implemented)
- ❌ Window bounds tracking (requires X11/Wayland API)
- ❌ Keyboard event detection (not yet implemented)

**Recommendation**: Auto-zoom is currently **Windows-only**. macOS and Linux support requires platform-specific implementations that are not yet complete.

---

## Future Improvements

Planned enhancements:
- [ ] Manual zoom region editing (add/remove/adjust in UI)
- [ ] Custom detection rules (user-defined triggers)
- [ ] Zoom presets (tutorial, gaming, presentation modes)
- [ ] Zoom preview before final export
- [ ] Per-region zoom level adjustment
- [ ] macOS and Linux support
