# Changelog

All notable changes to EasySpecy will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.5] - 2026-06-05

### Added
- **Single-instance enforcement** — Prevents multiple EasySpecy windows; re-launching restores and focuses the existing window.
- **Tray menu state management** — Menu items now enable/disable based on recording state (Start/Stop/Pause/Resume).
- **Tray pause/resume controls** — Added "Pause Recording" and "Resume Recording" options to system tray menu.
- **Tray "Open" command** — Renamed "Show Window" to "Open" for clarity; opens and focuses main app window.

### Fixed
- **"Reveal in Explorer" context menu** — Now correctly opens file explorer with recording highlighted instead of playing the video.
- **Toast notifications blocked on Settings page** — Increased z-index from 50 to 10000 to prevent Settings header from blocking notifications.
- **Tray menu static state** — Menu items now dynamically update enabled/disabled state during recording lifecycle.

### Changed
- **Tray module architecture** — Rewritten with state-aware menu item management using `MenuItem::set_enabled()`.

---

## [0.1.4] - 2026-06-04

### Added
- **"Mobile Shareable" video encoder** — New `MobileShareable` preset using libx264 + yuv420p for maximum mobile device compatibility.
- **GPU encoder toggle** — Settings now has an "Enable GPU Encoders" toggle; NVENC options are hidden when disabled, with automatic fallback to CPU encoders.
- **Left/right modifier key tracking** — Keyboard capture distinguishes LShift/RShift, LCtrl/RCtrl, LAlt/RAlt, LWin/RWin independently.
- **Markdown release notes** — Update dialog renders release notes with basic markdown (headers, bold, code, lists).
- **Vite code-splitting** — Manual chunks for motion, UI libs, and Tauri APIs to improve load performance.

### Changed
- **Keyboard overlay bake pipeline rewritten** — Full parity with live JS overlay logic: shortcut combos, shift+key mapping, backspace deletion, arrow keys, punctuation merging. Now matches overlay.html behavior exactly.
- **Recording filename format** — Changed from `recording_YYYY-MM-DD_HH-MM-SS.mp4` to `EasySpecy_DD_Month_YYYY_HH_MM_SS.mp4`.
- **Stop recording order** — Keyboard capture stopped before destroying overlay to preserve final key events.
- **Keyboard overlay positioning** — Config values now treated as physical pixels (removed DPI scale factor multiplication).
- **Sync verifier tolerance** — Frame count check relaxed from 5% to 20% with effective FPS diagnostics for VFR capture.
- **Canvas context** — Added `{ desynchronized: true }` hint for lower-latency overlay rendering; removed `backdrop-filter` for GPU performance.
- **Punctuation-append parity** — After punctuation, Rust bake pipeline now extends existing bubble (matches JS overlay behavior instead of creating new bubble).
- **Render loop cleanup** — Added one extra clear frame after effects end to prevent stale pixel artifacts.

### Removed
- **Password field detection & masking** — Entire feature removed due to high overhead (Win32 API + MSAA + cursor-point fallback + COM initialization). Will revisit in future with optimized approach.

### Fixed
- **Keyboard character case** — Use `e.key` directly instead of manual lowercase conversion for proper CapsLock + Shift handling.
- **VP9 FFmpeg args** — Fixed argument ordering for `-b:v 0` and `-pix_fmt yuv420p`.
- **vite.config.ts** — Removed unused `@ts-expect-error` directive.
- **GPU encoder config IPC** — Added `gpu_encoders_enabled` to `update_config_field` match arms (now works via `updateField()` IPC calls, not just full-save).
- **Worker thread leak** — Re-added `WORKER_THREAD` clear in `stop_keyboard_capture` to prevent stale thread handles.
- **Mutex poisoning safety** — Changed `WORKER_THREAD.lock().unwrap()` to defensive `if let Ok(mut guard)` pattern in 3 locations to prevent hook thread crash if mutex becomes poisoned.

---

## [0.1.3] - 2026-06-04

### Added
- **Auto-update system** — Built-in updater using Tauri's `@tauri-apps/plugin-updater` with minisign signature verification.
- **NSIS install path memory** — Installer automatically detects and uses the previous install location via Windows registry.
- **Portable update support** — Portable mode detects `.portable` marker and downloads new ZIP, replaces the exe, and relaunches.
- **Update UI** — Settings page shows available updates with download progress, and handles both NSIS and portable update flows.
- **CI/CD signing** — Release workflow signs NSIS installer and portable ZIP with minisign, auto-generates `latest.json` update manifest.

---

## [0.1.2] - 2026-06-04

### Added
- **Pause/Resume support** — Real-time segment-based recording. Finalizes segment on pause and starts a new segment on resume (stitched via FFmpeg copy on stop), avoiding Graphic Capture API reinitialization delay.
- **Dual-Timer UI** — Added real-time tracking of both active recording duration and pause duration on the dashboard.

### Fixed
- **Stop when paused** — Moved stop detection to the front of the frame processing loop, allowing users to stop recording directly from a paused state without hanging or crashing.
- **Recording thread leak** — Fixed recording threads remaining active after stop when paused by correctly signaling exit and conditionally finalizing segments.
- **Right-click secondary color** — Added click-effect secondary color (e.g. red) support on right-clicks for Settings/Customizer previews, the live recording overlay, and the post-processing pipeline.

---

## [0.1.1] - 2026-06-04

### Fixed
- **GPU encoder toggle** — GPU encoders (NVENC/AMF/QSV) now correctly fall back to CPU equivalents when the GPU encoders toggle is disabled. Previously the saved encoder selection could persist across restarts even after the toggle was turned off.
- **`.meta.json` cleanup** — The temporary cursor/click metadata file (`.meta.json`) is no longer left behind in the recordings folder after post-processing completes. It is written during recording for use by the effects and autozoom pipeline, then automatically deleted once all post-processing is done. Only the final `.mp4` remains.

---

## [0.1.0] - 2026-06-03

### Added
- Initial release
- Screen recording with H264 / H265 / AV1 / VP9 and NVENC GPU-accelerated encoders
- System audio + microphone capture with noise gate, spectral subtraction, and RNN denoising
- Cursor trail effects (glow, comet, neon) and click ripple/burst overlays rendered live via transparent WebView2 overlay
- Keyboard overlay with customisable key bubbles
- Webcam picture-in-picture (circle / rounded / squircle masks, composited by FFmpeg)
- Region capture with pixel-perfect crop
- Auto-zoom engine (click cluster, typing, circle gesture, text selection detection)
- System tray integration with global hotkeys
- Recording history with file size, duration, and resolution tracking
- Sync verifier to catch audio/video desync on stop
