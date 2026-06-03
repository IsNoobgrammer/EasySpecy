<!-- ═══════════════════════════════════════════════════════
     EASYSPECY — PREMIUM README
     Branded Hero + Bento Layout + System Architecture HLD
     ═══════════════════════════════════════════════════════ -->

<div align="center">
  <img src="docs/public/logo.png" alt="EasySpecy Logo" width="100" height="100">
  
  <h1>EasySpecy</h1>
  <p><strong>Record like a pro. Pay like it's 2005.</strong></p>
  
  <p style="max-width: 600px; color: #9a9eb5; line-height: 1.6;">
    A free, open-source, high-performance screen recorder for developers, creators, and power users. Captures high-framerate desktop feeds with cinematic auto-zoom, customizable cursor trails, keyboard overlays, and live webcam PIP.
  </p>
</div>

<div align="center" style="margin: 20px 0;">
  <a href="https://github.com/IsNoobgrammer/EasySpecy/releases/latest">
    <img src="https://img.shields.io/badge/Download_for_Windows-00E88A?style=for-the-badge&logo=windows&logoColor=11131e&labelColor=11131e" alt="Download Installer" height="40">
  </a>
  <a href="https://github.com/IsNoobgrammer/EasySpecy/releases/latest">
    <img src="https://img.shields.io/badge/Portable_ZIP-1d1f2b?style=for-the-badge&logo=archive&logoColor=00E88A&labelColor=1d1f2b" alt="Download Portable ZIP" height="40">
  </a>
</div>

<div align="center" style="margin-top: 15px; margin-bottom: 30px;">
  <img src="https://img.shields.io/badge/Tauri-v2-FFC131?style=flat-square&logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/Rust-2021-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/React-v19-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-v5.8-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript">
  <a href="https://isnoobgrammer.github.io/EasySpecy/">
    <img src="https://img.shields.io/badge/Docs-Live-00E88A?style=flat-square" alt="Documentation">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-00E88A?style=flat-square" alt="License">
</div>

---

## 📸 Interface Preview

Here is how EasySpecy looks in action, adapting automatically to your operating system's theme settings:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/public/docs-main-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="docs/public/docs-inner-light.png">
  <img src="docs/public/docs-main-dark.png" alt="EasySpecy Dashboard View" width="100%" style="border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.08); box-shadow: 0 20px 40px rgba(0,0,0,0.35);">
</picture>

---

## ⚡ Bento Feature Showcase

<table width="100%">
  <tr>
    <td width="50%" valign="top" style="padding: 16px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; background: rgba(21, 24, 40, 0.4);">
      <h3>🔍 Cinematic Auto-Zoom</h3>
      <p style="color: #9a9eb5; font-size: 13.5px; line-height: 1.5;">
        Intelligent camera zoom that tracks your cursor and clicks, applying smooth cubic bezier transitions to mimic professional post-production. Configurable zoom scales, speeds, and focus offsets.
      </p>
    </td>
    <td width="50%" valign="top" style="padding: 16px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; background: rgba(21, 24, 40, 0.4);">
      <h3>⌨️ Keyboard Overlay</h3>
      <p style="color: #9a9eb5; font-size: 13.5px; line-height: 1.5;">
        Display real-time keypresses on screen during tutorials or gameplay. Choose from 4 beautiful themes (including Glassmorphism and Neon), with dynamic keycap bounce animations and latency graphs.
      </p>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top" style="padding: 16px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; background: rgba(21, 24, 40, 0.4);">
      <h3>🖱️ Customizable Cursor Trails</h3>
      <p style="color: #9a9eb5; font-size: 13.5px; line-height: 1.5;">
        Make your cursor highly visible with vector-interpolated smooth trails, click wave ripples, and custom colors. Support for multiple cursor asset packs for a unique presentation.
      </p>
    </td>
    <td width="50%" valign="top" style="padding: 16px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; background: rgba(21, 24, 40, 0.4);">
      <h3>🎥 Pro Webcam PIP</h3>
      <p style="color: #9a9eb5; font-size: 13.5px; line-height: 1.5;">
        Embed your facecam inside a highly customizable, hardware-accelerated picture-in-picture window. Adjust border sizing, shape presets, opacity, and position on the fly without drop frames.
      </p>
    </td>
  </tr>
  <tr>
    <td colspan="2" valign="top" style="padding: 16px; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; background: rgba(21, 24, 40, 0.4);">
      <h3>🔊 Dual Audio Noise Gate</h3>
      <p style="color: #9a9eb5; font-size: 13.5px; line-height: 1.5;">
        Record both system audio and microphone streams concurrently. Includes integrated RNN noise suppression (via RNNoise) to dynamically filter mouse clicks, keyboard clacks, and background fan hums.
      </p>
    </td>
  </tr>
</table>

---

## 🏗️ High-Level Design (HLD) & System Architecture

EasySpecy is built on a split-architecture model that divides tasks between a web-standard frontend UI and a highly optimized native systems backend.

### Subsystem Flowchart

```mermaid
graph TD
    %% Styling and layout
    classDef frontend fill:#1e2030,stroke:#61dafb,stroke-width:2px,color:#fff;
    classDef tauri fill:#16192b,stroke:#ffc131,stroke-width:2px,color:#fff;
    classDef backend fill:#11131e,stroke:#00e88a,stroke-width:2px,color:#fff;
    classDef external fill:#0c0e18,stroke:#f04040,stroke-width:2px,color:#fff;

    %% Subgraph layers
    subgraph UI_Layer ["UI Layer (React + Vite)"]
        UI[Settings Panel & Recording Controls]:::frontend
        KeyOverlay[Keypress Visualizer Overlay]:::frontend
    end

    subgraph Bridge_Layer ["Bridge Layer (Tauri v2)"]
        IPC[Tauri IPC / Command Router]:::tauri
    end

    subgraph Backend_Layer ["Backend Engine (Rust Core)"]
        VideoCapture[Windows Graphics Capture / SCK]:::backend
        AudioCapture[CPAL Audio Capture: System & Mic]:::backend
        RNNoise[RNN Noise Gate Filter]:::backend
        FrameProcessor[Frame Post-Processor: Easing, Rails, Zoom]:::backend
        Rayon[Rayon Parallel Thread Pool]:::backend
        FFmpegPipe[FFmpeg Sub-process Pipeline]:::backend
    end

    subgraph OS_Layer ["Hardware & OS Layer"]
        OS_Video[OS Display Surface Buffer]:::external
        OS_Audio[DirectSound / WASAPI Streams]:::external
    end

    %% Connections
    UI -->|IPC Settings & State| IPC
    IPC -->|Command Dispatch| FrameProcessor
    IPC -->|Capture Parameters| VideoCapture

    OS_Video -->|DXGI / SCK Frames| VideoCapture
    OS_Audio -->|cpal Host stream| AudioCapture

    VideoCapture -->|Raw BGRA Frames| FrameProcessor
    AudioCapture -->|Raw PCM Buffers| RNNoise
    RNNoise -->|Clean PCM| FFmpegPipe

    FrameProcessor -->|Parallel Matrix Transforms| Rayon
    Rayon -->|RGBA Frames| FrameProcessor
    FrameProcessor -->|Encoded Frame Stream| FFmpegPipe

    FFmpegPipe -->|Write Container| OutFile[(Output MP4 File)]:::external
```

### Core Architecture Highlights

1. **Zero-Copy Display Frame Capture**:
   - Rather than scanning memory buffers periodically, the Rust core queries frame updates directly from GPU display surfaces using native system APIs (e.g. `Windows Graphics Capture` on Windows, `ScreenCaptureKit` on macOS).
   - This provides hardware-assisted, sub-millisecond capturing performance, ensuring screen captures remain locked at `60 FPS` even under heavy gaming or CPU rendering loads.

2. **Parallel Frame Post-Processor**:
   - The captured display frames undergo a processing pipeline that overlays vector-interpolated cursor trails, webcam video PIP nodes, and click ripple effects.
   - These compute-heavy operations are chunked and executed in parallel across a CPU thread pool using `Rayon`. It prevents CPU thread bottlenecks and maintains consistent framerates.

3. **High-Performance Audio Pipeline & RNN Filter**:
   - Audio inputs are handled using the `cpal` systems audio interface. The engine captures microphone inputs and loopbacks system audio, converting them to clean single-format PCM audio buffers.
   - The microphone stream is piped directly through `nnnoiseless` (a Rust implementation of Mozilla's `RNNoise` Recurrent Neural Network). The neural network isolates vocal signals and strips out keyboard typing, clicks, and background ambient sounds.

4. **FFmpeg Sub-Process Streaming**:
   - processed frames (RGBA) and clean audio bytes (PCM) are written directly into an active, low-overhead FFmpeg subprocess pipe.
   - The frames are encoded on the fly (leveraging hardware encoders like H.264 NVENC/AMF/QSV when available) and written to the output file wrapper (`.mp4`), ensuring the video is ready immediately on stop with **zero post-processing delay**.

---

## 🔧 Platform Support Matrix

| Feature | Windows | macOS | Linux |
|---------|:-------:|:-----:|:-----:|
| **Display Capture (60 FPS)** | ✅ (WGC) | ✅ (SCK) | ⚠️ (PipeWire) |
| **Cinematic Auto-Zoom** | ✅ (Direct) | ❌ | ❌ |
| **Vector Cursor Trails** | ✅ (Direct) | ❌ | ❌ |
| **Keyboard Overlay** | ✅ (Tauri Win Hook) | ❌ | ❌ |
| **RNN Audio Noise Gate** | ✅ (RNNoise) | ✅ (RNNoise) | ✅ (RNNoise) |
| **Hardware Encoding** | ✅ (NVENC/AMF) | ✅ (VideoToolbox) | ⚠️ (VAAPI) |

---

## 🛠️ Build from Source

### Prerequisites

Ensure you have the following installed on your machine:
- **Rust Toolchain** (via [rustup](https://rustup.rs/))
- **Node.js v20+** (with `npm`)
- **FFmpeg** (installed and added to your system `PATH`)
- **MSVC Build Tools** (for compiling native Windows bindings)

### Build Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/IsNoobgrammer/EasySpecy.git
   cd EasySpecy
   ```

2. **Install frontend dependencies**:
   ```bash
   npm install
   ```

3. **Start the development server (Live Hot Reloading)**:
   ```bash
   npm run tauri dev
   ```

4. **Build the production installer**:
   ```bash
   npm run tauri build
   ```
   The built installer `.exe` will be located under `src-tauri/target/release/bundle/nsis/`.

---

## 📖 Additional Resources

- [Full Documentation Site](https://isnoobgrammer.github.io/EasySpecy/) — Detailed configurations and advanced guides.
- [Auto-Zoom Guide](https://isnoobgrammer.github.io/EasySpecy/guide/auto-zoom) — Learn how to tweak easing curves.
- [Contributing Guidelines](https://isnoobgrammer.github.io/EasySpecy/guide/contributing) — Help us make EasySpecy better!

---

<div align="center" style="margin-top: 48px; padding: 24px 0; border-top: 1px solid #2a2d42;">
  <p style="color: #5c6078; font-size: 13px;">
    EasySpecy is built with Rust + Tauri • Maintained by <a href="https://github.com/IsNoobgrammer" style="color: #00e88a; text-decoration: none;">Shaurya</a> • <a href="https://isnoobgrammer.github.io/EasySpecy/" style="color: #00e88a; text-decoration: none;">Documentation</a>
  </p>
  <p style="color: #5c6078; font-size: 11px; margin-top: 4px;">
    Released under the MIT License
  </p>
</div>
