# EasySpecy — Product Requirements Document

**Version:** 1.0
**Date:** 2026-05-31
**Status:** Draft

---

## 1. Product Vision

**One-liner:** A free, open-source, cross-platform screen recorder with cinematic auto-zoom and cursor effects — built for devs and content creators who refuse to pay $89/yr for Screen Studio.

**Problem:** Screen Studio charges $89/yr and is macOS-only. OBS is complex and has no auto-zoom. Loom charges $15/mo. ShareX is Windows-only. No free tool combines native performance + auto-zoom + cursor effects + cross-platform.

**Solution:** EasySpecy — a Tauri + Rust desktop app that records screens at configurable resolution/FPS with audio, auto-zooms on click events, animates cursor trails, overlays webcam, and saves directly to file. Free, fast, cross-platform.

---

## 2. User Personas

### Persona 1: Dev Demo Dan
- **Role:** Software developer recording product demos and code walkthroughs
- **Goals:** Record clean 1080p60 demos with auto-zoom on code sections, share with team
- **Pains:** OBS is overconfigured, Screen Studio costs money, Loom requires internet
- **Success:** Hotkey → record → stop → file saved. No post-processing needed.

### Persona 2: Creator Casey
- **Role:** Content creator making tutorials and YouTube videos
- **Goals:** Polished recordings with cursor effects, webcam overlay, cinematic zoom
- **Pains:** Hours spent in post-production adding zoom effects manually
- **Success:** Record once, auto-zoom + cursor trail applied automatically on export.

### Persona 3: Power User Pat
- **Role:** Developer who records bug reports and shares with team
- **Goals:** Quick region capture, system tray recording, hotkey-driven workflow
- **Pains:** ShareX is Windows-only, no Linux/Mac equivalent with same feature set
- **Success:** Hotkey triggers region select → records → saves → copies path to clipboard.

---

## 3. Feature Requirements

| ID  | Feature                        | Priority | Phase | Acceptance Criteria |
|-----|--------------------------------|----------|-------|---------------------|
| F01 | Screen capture (full/region/window) | P0 | 1 | Can capture at 480/720/1080p, 24/30/60fps |
| F02 | Audio recording (mic + system) | P0 | 1 | Configurable sample rate (22050/44100/48000 Hz) |
| F03 | Resolution & FPS picker       | P0 | 1 | Dropdown in GUI, persisted in settings |
| F04 | Hotkey start/stop/pause       | P0 | 1 | System-wide hotkey works when app unfocused |
| F05 | System tray minimize          | P0 | 1 | App minimizes to tray, recording continues |
| F06 | Direct file save              | P0 | 1 | MP4 saved immediately on stop, no re-encode wait |
| F07 | Webcam overlay (PiP)          | P0 | 1 | Webcam feed overlaid, resizable, repositionable |
| F08 | Settings persistence          | P0 | 1 | All prefs saved to config file, restored on launch |
| F09 | Auto-zoom on click            | P1 | 2 | Zooms toward click position, smooth easing |
| F10 | Cursor trail effect           | P1 | 2 | Glowing trail follows cursor path during recording |
| F11 | Cursor smoothing              | P1 | 2 | Jittery cursor movements smoothed in output |
| F12 | Region select                 | P1 | 2 | Drag-to-select recording area with pixel coordinates |
| F13 | Window capture                | P1 | 2 | Record specific window, crop to window bounds |
| F14 | Custom cursor size/color      | P1 | 2 | User can enlarge cursor, change color in settings |
| F15 | Tab switching capture         | P2 | 3 | Record one window, auto-switch to another via hotkey |
| F16 | Key-triggered zoom            | P2 | 3 | Press hotkey + drag to zoom into specific area |
| F17 | Export presets                | P2 | 3 | GIF, MP4, WebM with quality presets |
| F18 | AI transcription              | P3 | 4 | Whisper-based subtitle generation |
| F19 | Annotations during recording  | P3 | 4 | Draw on screen while recording |
| F20 | Scheduled recording           | P3 | 4 | Timer-based start/stop |

---

## 4. User Stories

**US-01: Quick Record**
> GIVEN the app is running in the system tray
> WHEN the user presses the record hotkey (e.g. Ctrl+Shift+R)
> THEN recording starts at the configured resolution/FPS/audio settings
> AND a tray icon change indicates active recording

**US-02: Auto-Zoom Recording**
> GIVEN auto-zoom is enabled in settings
> WHEN the user records and clicks on different areas of the screen
> THEN the output video smoothly zooms toward each click position
> AND zooms back out after a configurable dwell time

**US-03: Webcam PiP**
> GIVEN webcam overlay is enabled
> WHEN the user starts recording
> THEN the webcam feed appears as a small overlay in the chosen corner
> AND the user can drag to reposition or resize the overlay

**US-04: Region Capture**
> GIVEN the user presses the region-select hotkey
> WHEN the user drags a rectangle on screen
> THEN only that region is captured
> AND the selection dimensions are shown in real-time

**US-05: Hotkey-Driven Workflow**
> GIVEN the app is in the system tray
> WHEN the user presses Ctrl+Shift+R to start, Ctrl+Shift+S to stop
> THEN the file is saved to the configured output directory
> AND the file path is copied to clipboard

---

## 5. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Recording latency | <50ms frame capture overhead | Profiling with scap benchmarks |
| Output file available | <2s after stop | Time from stop press to file on disk |
| App startup time | <1s cold start | Tauri window ready time |
| Memory usage while recording | <200MB | Task manager / Activity Monitor |
| GitHub stars (6 months) | 500+ | GitHub analytics |
| Cross-platform parity | Feature-complete on Win/Mac/Linux | Manual testing checklist |

---

## 6. Non-Goals (Explicit)

- **No cloud sync or sharing** — EasySpecy is a local-first tool. No accounts, no servers.
- **No built-in video editor** — Users who want editing can use DaVinci Resolve, CapCut, etc. The file is ready immediately.
- **No streaming** — OBS handles live streaming. EasySpecy is recording-only.
- **No AI post-processing in MVP** — AI transcription is P3, not blocking launch.
- **No mobile support** — Desktop only (Windows, macOS, Linux).

---

## 7. Technical Constraints

- **Platform:** Windows 10+, macOS 12+, Linux (X11/Wayland)
- **Bundle size target:** <15MB installer (Tauri + FFmpeg binary)
- **License:** MIT (open source)
- **Dependencies:** scap (Rust screen capture), FFmpeg (bundled binary), cpal (audio)
- **Build:** Tauri 2.x, Rust 1.75+, Node.js 18+ for frontend

---

## 8. Open Questions

1. Should we bundle FFmpeg or use a lighter encoder (e.g. pure Rust H.264 viaopenh264)?
2. Auto-zoom: apply in real-time during recording, or as a post-processing step?
   - Real-time: harder, but instant output
   - Post-process: easier, but adds delay after stop
   - **Recommendation:** Post-process with fast FFmpeg filter, show progress bar
3. Webcam overlay: use native camera APIs or go through FFmpeg's v4l2/avfoundation?
4. Should settings be JSON or TOML? (TOML is more Rust-idiomatic)

---

## 9. Context for AI Agents

```
Project: EasySpecy
Type: Desktop screen recorder
Stack: Tauri 2 + Rust + React + FFmpeg
Capture: scap (native OS APIs)
Audio: cpal
License: MIT
Priority: Performance > Features > UI polish
Pattern: Hotkey-driven workflow, system tray, direct save
Anti-patterns: No cloud dependency, no post-processing delay, no Electron
```
