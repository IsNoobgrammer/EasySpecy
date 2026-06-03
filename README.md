<div align="center">
  <img src="public/logo.png" alt="EasySpecy" width="80" height="80">
  
  <h1>EasySpecy</h1>
  <p><em>Record like a pro. Pay like it's 2005.</em></p>
  
  <p style="color: #9a9eb5; max-width: 600px; margin: 0 auto;">
    Free, open-source screen recorder with cinematic auto-zoom, cursor effects, and webcam overlay.<br>
    Built with Rust + Tauri for native performance.
  </p>
</div>

<div align="center" style="margin-top: 24px;">
  <img src="https://img.shields.io/badge/Tauri-2-FFC131?style=flat-square&logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/Rust-2021-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-5.8-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/License-MIT-00E88A?style=flat-square" alt="License">
</div>

---

<div align="center" style="padding: 24px 0;">
  <a href="#features" style="margin: 0 16px; color: #00e88a; text-decoration: none;">Features</a>
  <a href="#installation" style="margin: 0 16px; color: #00e88a; text-decoration: none;">Installation</a>
  <a href="#quick-start" style="margin: 0 16px; color: #00e88a; text-decoration: none;">Quick Start</a>
  <a href="https://isnoobgrammer.github.io/EasySpecy/" style="margin: 0 16px; color: #00e88a; text-decoration: none;">Documentation</a>
</div>

---

## Features

<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 16px;">

**Auto-Zoom**
Cinematic zoom toward clicks. Five detection methods, zero performance impact.

**Cursor Effects**
5 cursor packs, smooth trails, click animations. Post-processed for quality.

**Keyboard Overlay**
Real-time keypress visualization. 4 themes, game capture support.

**Webcam Overlay**
PiP webcam with positioning, shapes, and real-time enhancements.

**Advanced Audio**
Multi-source capture with RNN noise reduction. Fine-tuned controls.

**Native Performance**
Sub-50ms latency. Hardware encoding. Parallel processing.

</div>

---

## Installation

### Windows

Download the latest release from [Releases](https://github.com/IsNoobgrammer/EasySpecy/releases)

### Build from Source

```bash
git clone https://github.com/IsNoobgrammer/EasySpecy.git
cd EasySpecy
npm install
npm run tauri dev
```

See [Installation Guide](https://isnoobgrammer.github.io/EasySpecy/guide/installation) for prerequisites and Linux/macOS builds.

---

## Quick Start

1. **Configure settings** → output directory, resolution, audio source
2. **Enable effects** (optional) → auto-zoom, cursor trails, webcam, keyboard overlay
3. **Press `Ctrl+Shift+R`** → recording starts
4. **Press `Ctrl+Shift+S`** → file saved to output directory

Full guide: [Quick Start](https://isnoobgrammer.github.io/EasySpecy/guide/quick-start)

---

## Platform Support

EasySpecy is **Windows-first**. macOS and Linux have experimental basic capture.

| Feature | Windows | macOS | Linux |
|---------|:-------:|:-----:|:-----:|
| Full-screen capture | ✅ | ✅ | ✅ |
| Region capture | ✅ | ⚠️ | ⚠️ |
| Auto-zoom | ✅ | ❌ | ❌ |
| Cursor effects | ✅ | ❌ | ❌ |
| Keyboard overlay | ✅ | ❌ | ❌ |
| Webcam overlay | ✅ | ✅ | ✅ |
| Hardware encoding | ✅ | ❌ | ❌ |

Details: [Platform Support](https://isnoobgrammer.github.io/EasySpecy/guide/installation#platform-support)

---

## Documentation

Comprehensive guides for every feature:

- 📖 [Full Documentation Site](https://isnoobgrammer.github.io/EasySpecy/)
- 🎬 [Screen Capture Guide](https://isnoobgrammer.github.io/EasySpecy/guide/screen-capture)
- 🔍 [Auto-Zoom Guide](https://isnoobgrammer.github.io/EasySpecy/guide/auto-zoom)
- 🖱️ [Cursor Settings](https://isnoobgrammer.github.io/EasySpecy/guide/cursor-settings)
- ⌨️ [Keyboard Overlay](https://isnoobgrammer.github.io/EasySpecy/guide/keyboard-settings)
- 🎥 [Webcam Settings](https://isnoobgrammer.github.io/EasySpecy/guide/webcam-settings)
- 🔊 [Audio Recording](https://isnoobgrammer.github.io/EasySpecy/guide/audio-recording)
- 🔧 [Configuration Reference](https://isnoobgrammer.github.io/EasySpecy/config/overview)

---

## Development

```bash
# Start Tauri dev mode
npm run tauri dev

# Frontend-only (faster for UI changes)
npm run dev

# Run tests
cd src-tauri && cargo test

# Lint
cargo clippy -- -D warnings
```

See [Contributing Guide](https://isnoobgrammer.github.io/EasySpecy/guide/contributing) for code standards and workflow.

---

<div align="center" style="margin-top: 48px; padding: 24px 0; border-top: 1px solid #2a2d42;">
  <p style="color: #5c6078; font-size: 14px;">
    Built with Rust + Tauri • <a href="https://github.com/IsNoobgrammer/EasySpecy" style="color: #00e88a; text-decoration: none;">GitHub</a> • <a href="https://isnoobgrammer.github.io/EasySpecy/" style="color: #00e88a; text-decoration: none;">Docs</a>
  </p>
  <p style="color: #5c6078; font-size: 12px; margin-top: 8px;">
    MIT License
  </p>
</div>
