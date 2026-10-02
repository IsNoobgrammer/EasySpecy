# Changelog

All notable changes to EasySpecy will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Security
- **Passwords no longer appear in the keyboard overlay** — Keys typed into a password field show as `•`. This covers browser `<input type="password">`, Win32 password edits and UWP PasswordBox, detected via UI Automation `IsPassword`. A focus-changed handler switches on Chromium/Electron accessibility up front, so the first keystrokes aren't missed. Shortcuts and Enter/Tab/Backspace stay visible.

### Added
- **Window recording** — Pick an open window from a list. The screen is cropped to the window's visible bounds (DWM extended frame, no invisible border), so the webcam, trail and keyboard overlays stay in the recording.

### Fixed
- **Region selector showed the app instead of the desktop** — It now takes a screenshot first and selects on top of it, on the primary monitor (the one being captured).
- **Region crop wrong on scaled displays** — CSS px are now converted to physical px.
- **Odd region sizes failed the encode** — Regions are clamped to the frame and rounded to even sizes.
- **Full-screen recordings stayed cropped after a region recording** — The region is now cleared, and is only applied in Region/Window mode.
- **A capture error lost the whole recording** — Stop now works after a capture-thread error. The in-flight segment and audio are salvaged, and the UI auto-stops and saves.
- **A failed start left things behind** — Cursors, the keyboard hook and the overlay were left in place, and the next start reported "already in progress".
- **Concurrent starts** — The session is claimed atomically.
- **Tray stuck on "recording" when stop failed.**
- **Re-encode path output 25 fps** — It now uses the configured FPS.
- **"Capture failed to start" on Windows 10 / older GPUs (#2)** — The live capture encoder was hardcoded to HEVC, which isn't available on GPUs like Intel HD 4000 / GT 630M or on stock Windows 10 (`0xC00D5212`). Live capture now uses H.264 unless an H.265 output is selected, and it falls back to H.264 automatically if HEVC can't be created.

- **End of recording chopped off** — When system audio was shorter than the video (nothing playing near the end), `-shortest` trimmed the video to the audio length. 143 s recordings lost about 9 s. Audio is now padded to the video length.
- **Recordings with no audio track** — If audio was enabled but nothing played, the file was saved without an audio stream. A silent track is now written.
- **Video after resume lost** — About 1 in 3 resumed segments came out as a single frame. windows-capture's frame pool has one buffer, and a cold encoder created on the resume frame could pin it and starve capture. The next segment's encoder is now created at pause time. Empty segments are no longer queued for concat.
- **Webcam bubble blank or "NotReadableError"** — The overlay opened the system default camera, which is often an idle virtual camera (NVIDIA Broadcast, phone mirroring). It now tries cameras until one delivers a real frame, physical cameras first, at 640×480.
- **Pause hotkey did nothing** — `hotkey_pause` is now registered and toggles pause/resume.
- **Old hotkeys stayed bound after changing them** — They were unregistered using the *new* config.
- **Cursor effects toggle ignored** — The trail and click effects drew even with effects turned off.
- **Mixed-codec segments** — Later segments reuse the codec that segment 0 actually got, so a settings change during a pause can't break concat.

### Changed
- **Stop → file ready: 111 s → 1.8 s** for a 60 s recording with pause, webcam, trail and keyboard overlay (i5-12450H / RTX 3050).
- **Single-encode pipeline** — When the live-encoded codec matches the selected output, the final pass only muxes audio (`-c:v copy`) and no longer re-encodes the whole video. Region crop now runs inside the one final encode instead of a separate libx264 pass.
- **Live encoder respects settings** — Bitrate and frame rate now follow the quality and FPS settings instead of a fixed 15 Mbps @ 60 fps.
- **No redundant FFmpeg probes on stop** — The single-segment pre-check and the duration probe on the copy path are skipped; each was a ~1 s cold start of the bundled ffmpeg.
- **Noise gate is O(N)** — It used a sliding-window sum instead of re-summing a 10 ms window for every sample.
- **Effects overlay always created** — It now always runs during recording, with an invisible capture heartbeat.
- **Default encoder is now H.264** for new installs, which keeps the fast copy path on every machine.

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
