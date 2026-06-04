# Changelog

All notable changes to EasySpecy will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
