# EasySpecy

A free, open-source, cross-platform screen recorder with cinematic auto-zoom and cursor effects.

## Features

- **Screen Recording** — Full screen, region, or window capture
- **Configurable Quality** — 480p/720p/1080p, 24/30/60 fps
- **Audio Recording** — Mic + system audio, configurable sample rate (22050/44100/48000 Hz)
- **Auto-Zoom** — Cinematic zoom on click events (like Screen Studio, but free)
- **Cursor Effects** — Cursor trail, smoothing, custom size/color
- **Webcam Overlay** — Picture-in-picture webcam, resizable & repositionable
- **Hotkeys** — System-wide shortcuts for start/stop/pause
- **System Tray** — Minimize to tray, record in background
- **Direct Save** — MP4 ready immediately on stop, no post-processing delay

## Tech Stack

- **Framework:** Tauri 2 (~3MB bundle, native perf)
- **Backend:** Rust (screen capture, audio, encoding)
- **Frontend:** React + TypeScript + Tailwind CSS
- **Capture:** Native OS APIs (Windows Graphics Capture / ScreenCaptureKit / PipeWire)
- **Audio:** cpal (cross-platform audio I/O)
- **Encoding:** FFmpeg (bundled)

## Install

### Download

Check the [Releases](https://github.com/IsNoobgrammer/EasySpecy/releases) page for pre-built installers:
- Windows: `.msi` or `.exe`
- macOS: `.dmg`
- Linux: `.AppImage` or `.deb`

### Build from Source

```bash
# Prerequisites
# - Rust (https://rustup.rs)
# - Node.js 18+
# - MSVC Build Tools (Windows) / Xcode (macOS) / build-essential (Linux)

git clone https://github.com/IsNoobgrammer/EasySpecy.git
cd EasySpecy
npm install
npm run tauri dev    # Development
npm run tauri build  # Production build
```

## Default Hotkeys

| Action  | Shortcut     |
|---------|-------------|
| Start   | Ctrl+Shift+R |
| Stop    | Ctrl+Shift+S |
| Pause   | Ctrl+Shift+P |

All hotkeys are configurable in Settings.

## License

MIT
