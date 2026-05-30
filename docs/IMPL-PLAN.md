# EasySpecy — Implementation Plan

**Version:** 1.0
**Date:** 2026-05-31

---

## Overview

Vertical-slice phases. Each phase ships end-to-end functionality. Prototype-first approach — get recording working ASAP, then layer effects.

---

## Phase 0: Foundation (Day 1-2)

**Goal:** Project scaffolding, build pipeline, empty Tauri app running on all platforms.

- [ ] **T0.1** `cargo create-tauri-app` with React + TypeScript template [XS]
- [ ] **T0.2** Configure `tauri.conf.json` — app name, bundle settings, window config [XS]
- [ ] **T0.3** Set up project structure: `src-tauri/src/{capture,audio,config,commands,postprocess}/` [S]
- [ ] **T0.4** Add `scap` dependency, verify compilation on Windows [S]
- [ ] **T0.5** Add `cpal` dependency, verify audio device enumeration [S]
- [ ] **T0.6** Implement `config` module — TOML read/write with serde, default settings [S]
- [ ] **T0.7** Set up React frontend with Tailwind, basic dashboard layout [S]
- [ ] **T0.8** Tauri IPC command: `get_config` / `save_config` [XS]
- [ ] **T0.9** GitHub repo setup: `.gitignore`, `README.md`, MIT `LICENSE` [XS]
- [ ] **T0.10** CI: GitHub Actions — build on Windows + macOS + Linux [S]

**Acceptance:** App launches on all 3 platforms, shows dashboard, reads/writes config.

---

## Phase 1: Core Recording MVP (Day 3-7)

**Goal:** Record screen + audio, save to MP4, hotkey control, system tray.

- [ ] **T1.1** `capture` module: `start_capture(monitor, width, height, fps)` using scap [M]
- [ ] **T1.2** `audio` module: `start_audio(device, sample_rate)` using cpal [M]
- [ ] **T1.3** Ring buffer: raw frames → `.yuv` temp file on disk [M]
- [ ] **T1.4** Audio writer: samples → `.wav` temp file [S]
- [ ] **T1.5** `encoder` module: FFmpeg CLI invocation to encode `.yuv` + `.wav` → `.mp4` [M]
- [ ] **T1.6** Tauri commands: `start_recording`, `stop_recording`, `pause_recording` [S]
- [ ] **T1.7** Global hotkeys: `tauri-plugin-global-shortcut` for start/stop/pause [S]
- [ ] **T1.8** System tray: icon, tooltip, menu (Start/Stop/Settings/Quit) [S]
- [ ] **T1.9** Minimize-to-tray: closing window hides to tray if recording active [S]
- [ ] **T1.10** Frontend: Record button, resolution/FPS/audio dropdowns [S]
- [ ] **T1.11** Frontend: Recording overlay — floating timer + stop button [S]
- [ ] **T1.12** Copy output path to clipboard on save [XS]
- [ ] **T1.13** Tray notification on save [XS]

**Acceptance:** Press Ctrl+Shift+R → records screen + audio → Ctrl+Shift+S → MP4 saved, path in clipboard.

---

## Phase 2: Effects & Polish (Day 8-14)

**Goal:** Auto-zoom, cursor effects, webcam overlay, region capture.

- [ ] **T2.1** Click event collector: OS-level mouse click hook, `{timestamp, x, y}` → `.zoommeta` [M]
- [ ] **T2.2** Cursor position sampler: 60Hz polling → `.cursortrail` [M]
- [ ] **T2.3** Auto-zoom post-process: FFmpeg `zoompan` filter chain from `.zoommeta` [L]
- [ ] **T2.4** Cursor trail post-process: FFmpeg overlay circles from `.cursortrail` [M]
- [ ] **T2.5** Cursor smoothing: FFmpeg smoothing filter on cursor path [S]
- [ ] **T2.6** Frontend: Auto-zoom settings toggle, zoom level, dwell time [S]
- [ ] **T2.7** Frontend: Cursor trail settings toggle, color, size [S]
- [ ] **T2.8** Webcam overlay: capture from camera via cpal/scap, overlay as PiP [L]
- [ ] **T2.9** Webcam position/size: draggable overlay in preview, configurable corners [M]
- [ ] **T2.10** Region select: fullscreen transparent overlay, drag-to-select, show dimensions [M]
- [ ] **T2.11** Window capture: enumerate windows via scap, crop to window bounds [M]
- [ ] **T2.12** Custom cursor size/color in post-process [S]

**Acceptance:** Record with auto-zoom enabled → click during recording → output video has cinematic zoom. Cursor trail visible as glowing path.

---

## Phase 3: Advanced Features (Day 15-21)

**Goal:** Tab switching, key-triggered zoom, export presets, polish.

- [ ] **T3.1** Tab switching: hotkey to switch captured window mid-recording [L]
- [ ] **T3.2** Key-triggered zoom: hold hotkey + drag to define zoom area during recording [L]
- [ ] **T3.3** Export presets: GIF (via FFmpeg palette), MP4, WebM with quality tiers [M]
- [ ] **T3.4** Recording history: list past recordings in dashboard with metadata [S]
- [ ] **T3.5** Recent recordings: click to open file or folder [S]
- [ ] **T3.6** Multi-monitor support: monitor picker dropdown [S]
- [ ] **T3.7** Error handling: disk full auto-stop, device disconnect warnings [M]
- [ ] **T3.8** Settings import/export (JSON) [S]
- [ ] **T3.9** App icon design + installer branding [S]
- [ ] **T3.10** README with screenshots, install instructions, feature list [S]

**Acceptance:** Full feature set working, polished UI, ready for release.

---

## Phase 4: Future (Post-MVP)

- [ ] AI transcription via whisper-rs
- [ ] Annotations during recording (draw on screen)
- [ ] Scheduled recording (timer-based)
- [ ] Cloud upload + share links
- [ ] GPU-accelerated encoding (NVENC/AMF/QSV)
- [ ] Plugin system for custom effects

---

## Task Dependency Graph

```
Phase 0 (Foundation) ─── all independent, do in order T0.1 → T0.10
    │
    ▼
Phase 1 (MVP) ─── T1.1 + T1.2 parallel → T1.3 + T1.4 → T1.5 → T1.6
                   T1.7 + T1.8 + T1.9 parallel (need T1.6)
                   T1.10 + T1.11 + T1.12 + T1.13 parallel (need T1.6)
    │
    ▼
Phase 2 (Effects) ─── T2.1 + T2.2 parallel → T2.3 + T2.4 + T2.5 parallel
                       T2.6 + T2.7 parallel (need T2.3, T2.4)
                       T2.8 → T2.9
                       T2.10 → T2.11
    │
    ▼
Phase 3 (Advanced) ─── T3.1, T3.2 need Phase 2 complete
                        T3.3 → T3.4 → T3.5
                        T3.6 + T3.7 + T3.8 + T3.9 + T3.10 parallel
```

---

## Complexity Estimates

| Phase | Total Tasks | Estimated Time | Critical Path |
|-------|------------|---------------|---------------|
| 0: Foundation | 10 | 1-2 days | T0.1 → T0.4 → T0.7 |
| 1: MVP | 13 | 3-5 days | T1.1 → T1.3 → T1.5 → T1.6 |
| 2: Effects | 12 | 5-7 days | T2.1 → T2.3 → T2.6 → T2.8 |
| 3: Advanced | 10 | 3-5 days | T3.1, T3.2 are blockers |
| **Total** | **45** | **12-19 days** | — |
