# EasySpecy — Architecture Document

**Version:** 1.0
**Date:** 2026-05-31

---

## 1. System Diagram

```mermaid
graph TD
    subgraph "Tauri App"
        subgraph "Frontend (React + Tailwind)"
            UI[Dashboard / Settings / Overlay]
            RegionSel[Region Selector Overlay]
            WebcamPreview[Webcam Preview]
        end

        subgraph "Backend (Rust)"
            IPC[Tauri IPC Commands]
            Hotkey[Global Hotkey Handler]
            Tray[System Tray Manager]

            subgraph "Capture Pipeline"
                SCAP[scap — Native Screen Capture]
                CPAL[cpal — Audio Capture]
                RingBuf[Ring Buffer → Disk]
            end

            subgraph "Post-Processing"
                ClickMeta[Click Event Collector]
                CursorMeta[Cursor Trail Collector]
                FFmpeg[FFmpeg Encoder + Filters]
            end

            Config[Config Manager — TOML]
        end
    end

    subgraph "OS Layer"
        WinAPI[Windows Graphics Capture]
        SCK[macOS ScreenCaptureKit]
        PW[Linux PipeWire]
        Webcam[Camera / V4L2 / AVFoundation]
    end

    subgraph "Output"
        MP4[.mp4 file]
        WAV[.wav temp]
        YUV[.yuv temp]
        META[.zoommeta / .cursortrail]
    end

    UI -->|IPC: start/stop/settings| IPC
    IPC --> SCAP
    IPC --> CPAL
    Hotkey -->|start/stop/pause| IPC
    Tray -->|menu actions| IPC

    SCAP -->|native API| WinAPI
    SCAP -->|native API| SCK
    SCAP -->|native API| PW
    WebcamPreview --> Webcam

    SCAP -->|raw frames| RingBuf
    CPAL -->|audio samples| WAV
    RingBuf --> YUV

    ClickMeta --> META
    CursorMeta --> META
    YUV --> FFmpeg
    WAV --> FFmpeg
    META -->|zoom/trail params| FFmpeg
    FFmpeg --> MP4

    Config -->|read/write| IPC
```

---

## 2. Component Responsibilities

| Component | Owns | Does NOT Own |
|-----------|------|-------------|
| scap | Screen frame capture, monitor enumeration | Encoding, file I/O |
| cpal | Audio stream capture, sample rate conversion | File writing |
| FFmpeg | Video encoding, filter application | Frame capture, UI |
| Hotkey | System-wide key registration | Recording logic |
| Tray | Icon, menu, notification | Recording state |
| Config | Settings read/write, defaults | Validation (frontend does this) |
| Post-Process | Auto-zoom metadata, cursor trail data, FFmpeg invocation | Frame capture |
| Dashboard | User interaction, settings UI | Recording logic |
| Overlay | Minimal recording UI (timer, stop) | Full settings |

---

## 3. Critical User Flow — Record with Auto-Zoom

```mermaid
sequenceDiagram
    participant User
    participant Hotkey
    participant Backend
    participant scap
    participant cpal
    participant Disk
    participant ClickHook
    participant FFmpeg

    User->>Hotkey: Ctrl+Shift+R
    Hotkey->>Backend: start_recording()
    Backend->>scap: start_capture(monitor, resolution, fps)
    Backend->>cpal: start_audio(device, sample_rate)
    Backend->>Disk: open .yuv temp + .wav temp
    Backend->>ClickHook: start_click_listener()
    Backend-->>User: Tray icon = "recording"

    loop Every frame (~16ms at 60fps)
        scap->>Disk: write raw frame to .yuv
    end

    loop Every audio buffer
        cpal->>Disk: write samples to .wav
    end

    loop On each mouse click
        ClickHook->>Disk: append {timestamp, x, y} to .zoommeta
    end

    User->>Hotkey: Ctrl+Shift+S
    Hotkey->>Backend: stop_recording()
    Backend->>scap: stop_capture()
    Backend->>cpal: stop_audio()
    Backend->>ClickHook: stop_click_listener()

    alt Auto-zoom enabled
        Backend->>FFmpeg: encode(.yuv, .wav, .zoommeta, filters)
    else Auto-zoom disabled
        Backend->>FFmpeg: encode(.yuv, .wav, no filters)
    end

    FFmpeg->>Disk: write final .mp4
    Backend->>Backend: copy path to clipboard
    Backend-->>User: Tray notification: "Saved: /path/file.mp4"
```

---

## 4. Data Model

```mermaid
erDiagram
    CONFIG {
        string output_dir
        int resolution_width
        int resolution_height
        int fps
        int audio_sample_rate
        string audio_device
        string video_codec
        bool auto_zoom
        float zoom_level
        int zoom_dwell_ms
        bool cursor_trail
        string cursor_trail_color
        bool cursor_smoothing
        bool webcam_overlay
        string webcam_device
        string webcam_position
        int webcam_size
        string hotkey_start
        string hotkey_stop
        string hotkey_pause
        bool minimize_to_tray
        bool copy_path_on_save
    }

    RECORDING {
        string id
        datetime started_at
        datetime stopped_at
        string output_path
        int duration_ms
        int resolution_w
        int resolution_h
        int fps
        bool has_auto_zoom
        bool has_cursor_trail
    }

    CLICK_EVENT {
        string recording_id
        int timestamp_ms
        int x
        int y
    }

    CURSOR_POSITION {
        string recording_id
        int timestamp_ms
        int x
        int y
    }

    CONFIG ||--o{ RECORDING : "generates"
    RECORDING ||--o{ CLICK_EVENT : "tracks"
    RECORDING ||--o{ CURSOR_POSITION : "tracks"
```

---

## 5. Deployment Topology

| Platform | Bundle Format | Screen Capture API | Audio API | Camera API |
|----------|--------------|-------------------|-----------|------------|
| Windows 10+ | .msi / .exe (NSIS) | Windows.Graphics.Capture | WASAPI | DirectShow |
| macOS 12+ | .dmg / .app | ScreenCaptureKit | CoreAudio | AVFoundation |
| Linux (X11/Wayland) | .AppImage / .deb | PipeWire | ALSA/PulseAudio | V4L2 |

**FFmpeg bundling:**
- Windows: Ship `ffmpeg.exe` in resources/
- macOS: Ship `ffmpeg` universal binary in app bundle
- Linux: Ship `ffmpeg` static build or depend on system package

---

## 6. External Dependencies

| Dependency | Purpose | Fallback if Unavailable |
|------------|---------|------------------------|
| scap | Screen capture | None — core dependency, must compile |
| cpal | Audio capture | Record video-only if no audio device |
| FFmpeg | Encoding + filters | None — core dependency, must bundle |
| tauri-plugin-global-shortcut | Hotkeys | Disable hotkeys, use GUI-only controls |
| tauri-plugin-tray | System tray | Run as normal window, no tray |

---

## 7. Failure Modes & Mitigations

| Failure | Impact | Mitigation |
|---------|--------|------------|
| scap capture drops frames | Video stutter | Log dropped frames, warn user to lower resolution |
| Disk full during recording | Recording lost | Monitor free space, auto-stop at 100MB remaining, save partial |
| FFmpeg not found at runtime | Cannot encode | Check on startup, show download prompt |
| Hotkey registration fails | No keyboard control | Show error in settings, allow reassignment |
| Camera in use by another app | No webcam overlay | Show "camera busy" warning, continue without webcam |
| Audio device disconnected mid-recording | Silent audio | Detect disconnect, switch to no-audio or show warning |
