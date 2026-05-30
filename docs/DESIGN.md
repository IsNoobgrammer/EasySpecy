# EasySpecy — Technical Design Document

**Version:** 1.0
**Date:** 2026-05-31

---

## 1. System Overview

EasySpecy is a Tauri 2 desktop application with a Rust backend handling all performance-critical work (screen capture, audio capture, encoding, post-processing) and a React frontend for the GUI. The architecture prioritizes:

1. **Zero-lag capture** — native OS APIs via scap, no FFmpeg for capture
2. **Instant save** — write raw frames to temp, encode on stop
3. **Post-process effects** — auto-zoom/cursor trail applied via FFmpeg filters after recording stops
4. **Hotkey-driven** — system-wide shortcuts via tauri-plugin-global-shortcut

---

## 2. Component Breakdown

### 2.1 Rust Backend (src-tauri/)

| Component | Responsibility | Key Crates |
|-----------|---------------|------------|
| `capture` | Screen/frame capture via native APIs | `scap`, `windows-capture` |
| `audio` | Mic + system audio capture | `cpal`, `hound` |
| `encoder` | Frame encoding to H.264/VP9 | `ffmpeg-next` or bundled FFmpeg CLI |
| `postprocess` | Auto-zoom, cursor trail, cursor smoothing | FFmpeg filter chains |
| `config` | Settings persistence | `serde`, `toml` |
| `hotkeys` | System-wide key bindings | `tauri-plugin-global-shortcut` |
| `tray` | System tray icon + menu | `tauri-plugin-tray` |
| `commands` | Tauri IPC commands exposed to frontend | `tauri::command` |

### 2.2 React Frontend (src/)

| Component | Responsibility |
|-----------|---------------|
| `Dashboard` | Main window — record button, settings, recent recordings |
| `Settings` | Resolution, FPS, audio sample rate, hotkeys, output dir |
| `RecordingOverlay` | Minimal floating overlay during recording (timer, stop button) |
| `WebcamPreview` | Webcam feed preview + position/size controls |
| `RegionSelector` | Fullscreen transparent overlay for drag-to-select |
| `TrayMenu` | System tray context menu (managed by Rust) |

---

## 3. Data Flows

### 3.1 Recording Flow

```
User presses hotkey
  → Rust hotkey handler fires
  → capture::start() begins scap frame capture loop
  → Each frame → ring buffer → raw .yuv temp file on disk
  → Audio stream → cpal callback → .wav temp file
  → Tray icon changes to "recording" state
  → Frontend overlay shows timer

User presses stop hotkey
  → capture::stop() signals frame loop to exit
  → encoder::encode() reads raw .yuv + .wav
  → FFmpeg encodes to .mp4 with configured resolution/FPS/codec
  → Auto-zoom filter chain applied if enabled (zoom metadata from click events)
  → Final .mp4 written to output directory
  → File path copied to clipboard
  → Tray notification: "Recording saved: /path/to/file.mp4"
```

### 3.2 Auto-Zoom Post-Processing

```
During recording:
  → Click events captured via OS hooks (not from scap frames)
  → Each click stored as: {timestamp, x, y}
  → Saved to .zoommeta JSON alongside raw frames

On stop (if auto-zoom enabled):
  → Read .zoommeta
  → Generate FFmpeg zoompan filter chain:
    - For each click event, create a zoom keyframe
    - Smooth easing (ease-in-out) between keyframes
    - Configurable zoom level (1.5x, 2x, 3x)
    - Configurable dwell time at zoomed position
  → Apply filter during encode pass
  → Result: video with cinematic auto-zoom baked in
```

### 3.3 Cursor Trail Effect

```
During recording:
  → Cursor position sampled at 60Hz via OS hooks
  → Stored as: {timestamp, x, y} in .cursortrail file

On stop (if cursor trail enabled):
  → Read .cursortrail
  → Generate FFmpeg drawtext/drawbox filter or overlay compositing
  → Trail rendered as fading circles along cursor path
  → Color, opacity, fade duration configurable
```

---

## 4. Key Design Decisions

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Screen capture | scap (native OS APIs) | FFmpeg capture, OBS libs | Native APIs give zero-copy GPU frames, lowest latency |
| Encoding | FFmpeg (bundled binary) | Pure Rust (openh264) | FFmpeg supports all codecs, battle-tested, openh264 has quality issues |
| Auto-zoom timing | Post-process | Real-time | Real-time requires GPU shaders + complex state machine. Post-process is simpler, reliable, and adds <5s delay |
| Audio library | cpal | ALSA/CoreAudio directly | Cross-platform, well-maintained, async callbacks |
| Frontend framework | React + Tailwind | Vue, Svelte, Solid | Largest ecosystem, user familiarity, fast prototyping |
| Config format | TOML | JSON, YAML | Rust-idiomatic, human-readable, serde support |
| Hotkey library | tauri-plugin-global-shortcut | rdev, device_query | Official Tauri plugin, system-wide, maintained |
| Temp storage | Raw YUV + WAV on disk | In-memory buffer | Disk avoids OOM on long recordings, SSD is fast enough |

---

## 5. Error Handling Strategy

| Error | Client (Frontend) | Server (Rust) |
|-------|-------------------|---------------|
| Camera not found | Show "No webcam detected" in overlay settings | Return error code, log to stderr |
| Mic not found | Show "No microphone" warning, record video-only | Fall back to no-audio capture |
| Disk full | Show notification, stop recording gracefully | Monitor free space, auto-stop at 100MB remaining |
| FFmpeg crash | Show "Encoding failed" with retry option | Log stderr, offer raw .yuv file as fallback |
| Hotkey conflict | Show "Hotkey already in use" in settings | Catch registration error, prompt for new key |

---

## 6. Security Considerations

- **No network requests** — fully offline, no telemetry, no analytics
- **No credentials** — no auth, no accounts, no tokens
- **File permissions** — output directory must be writable, check on startup
- **Hotkey safety** — don't register hotkeys that conflict with OS defaults (Cmd+Q, Alt+F4)
- **Webcam indicator** — show clear visual when webcam is active (privacy)
- **Update mechanism** — Tauri's built-in updater with signature verification (if enabled)

---

## 7. Scalability Notes

| Concern | Current Target | Scaling Path |
|---------|---------------|--------------|
| Recording length | Up to 2 hours | Ring buffer + disk streaming handles arbitrarily long recordings |
| Resolution | Up to 1080p | scap supports 4K, FFmpeg handles any resolution |
| FPS | Up to 60fps | scap supports higher, limited by display refresh rate |
| Concurrent captures | 1 screen | Multi-monitor support via scap's monitor enumeration |
| Post-process time | <5s for 10min video | FFmpeg is fast, GPU encoding (NVENC/AMF) as future optimization |
