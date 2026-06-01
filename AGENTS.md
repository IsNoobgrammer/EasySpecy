# EasySpecy — AI Agent Configuration

**Project:** EasySpecy — Free, cross-platform screen recorder
**Stack:** Tauri 2 + Rust + React + FFmpeg
**Current Phase:** 3 — Final Polish (Auto-zoom + Webcam remaining)

---

## Project Context

EasySpecy is a desktop screen recorder built with Tauri. The Rust backend handles all performance-critical work (capture, encoding, effects). The React frontend is a thin GUI layer. FFmpeg is bundled for encoding and post-processing.

---

## Tech Conventions

### Rust (src-tauri/)
- **Module structure:** `capture/`, `audio/`, `config/`, `commands/`, `postprocess/`
- **Error handling:** `Result<T, String>` for Tauri commands, `anyhow::Result` internally
- **Async:** Use `tokio` for async I/O, `std::thread` for capture loops
- **Logging:** `tracing` crate, not `println!`
- **Config:** TOML via `serde` + `toml` crate, stored in `~/.config/easyspecy/config.toml`

### React (src/)
- **Framework:** React 18 + TypeScript + Tailwind CSS
- **State:** Zustand for global state (recording state, config)
- **IPC:** `@tauri-apps/api` invoke() for all Rust communication
- **Components:** Functional components only, no class components
- **Styling:** Tailwind utility classes, no CSS modules

### FFmpeg
- **Invocation:** Shell out to bundled FFmpeg binary, NOT ffmpeg-next crate (simpler, more reliable)
- **Path:** `resource_dir/ffmpeg` (platform-appropriate binary)
- **Filters:** Use `-vf` for zoompan, drawtext; `-af` for audio processing
- **Release:** FFmpeg binary too large for Git. Use GitHub Releases with attached assets, or download at first run (see Release Strategy below)

---

## Critical Learnings (Battle-Tested)

### Synchronization Architecture
ALL subsystems MUST sync to the same instant (`CAPTURE_ARMED` = first video frame):
- **Video:** `CAPTURE_ARMED` fires on first frame → `START_TIME = Instant::now()`
- **Audio:** `set_armed(true)` called at same instant → samples start recording
- **Cursor:** `reset_session_start()` called from mouse thread after detecting `CAPTURE_ARMED` → cursor timestamp 0 = video frame 0
- **NEVER start collecting data before CAPTURE_ARMED** — any pre-arm data has wrong timestamps

### Cursor Trail — What Works
- **Post-processing approach** — render trail onto video AFTER recording (not live overlay)
- **Pre-smooth entire path** with Catmull-Rom spline BEFORE rendering any frames
- **Per-frame rendering** at video FPS with parallel batch processing (rayon)
- **Comet model:** bright head at cursor position, fading tail behind
- **Coordinates:** `GetCursorPos` returns screen coords = video coords (capture is at monitor resolution). NO SCALING.
- **Parallel rendering:** Pre-compute trail segments with `rayon::par_iter`, batch-render frames in parallel

### Cursor Trail — What DOESN'T Work
- **Transparent overlay window** — WebView2 on Windows cannot do transparent click-through properly. White screen, can't close, doesn't receive events. DEAD END.
- **FFmpeg drawbox filters** — produces ugly square boxes with gaps. Not smooth.
- **Scaling cursor coords** — video is captured at monitor native resolution, not config resolution. Don't scale.
- **Starting cursor collection before CAPTURE_ARMED** — creates 200-500ms timing offset

### Audio
- `cpal` uses WASAPI shared mode by default — does NOT lock mic exclusively
- Other apps can access mic simultaneously (shared mode)
- Noise gate + spectral subtraction configurable via settings
- Mic gain, system volume, noise reduction all user-adjustable

### Performance
- Frame rendering is CPU-bound → use `rayon` parallel iterators
- Batch-render frames (2x CPU cores) then pipe to FFmpeg in order
- Pre-compute trail segments in parallel before rendering
- Cache config/screen metrics outside hot loops

---

## File Structure

```
EasySpecy/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs          # App setup, global AppHandle
│   │   ├── capture/        # windows-capture, frame capture, mouse tracking
│   │   ├── audio/          # cpal WASAPI, noise processing
│   │   ├── config/         # TOML config read/write
│   │   ├── commands/       # Tauri IPC commands
│   │   ├── postprocess/    # Trail rendering, FFmpeg compositing
│   │   ├── sync_verifier.rs # A/V sync verification
│   │   ├── cursors/        # Cursor pack system
│   │   ├── region.rs       # Region capture
│   │   └── tray/           # System tray setup
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── Dashboard.tsx
│   │   ├── Settings.tsx
│   │   ├── Customization.tsx
│   │   ├── RecordingOverlay.tsx
│   │   └── RegionSelector.tsx
│   ├── stores/
│   │   └── recording.ts    # Zustand store
│   └── lib/
│       ├── effects.ts      # Canvas trail renderer (preview only)
│       └── theme.ts
├── brand/                   # Logo, brand assets
├── public/                  # Static assets, favicons
├── AGENTS.md               # This file
├── README.md
├── LICENSE
└── .github/
    └── workflows/
        └── build.yml       # CI for all 3 platforms
```

---

## Release Strategy (FFmpeg)

FFmpeg binary is ~80-130MB — too large for Git. Options:
1. **GitHub Releases:** Attach FFmpeg as a release asset. CI downloads it during build.
2. **First-run download:** App downloads FFmpeg on first launch (like yt-dlp does).
3. **Tauri sidecar:** Use `externalBin` in tauri.conf.json — Tauri bundles it in the installer but not in Git.
4. **Git LFS:** Store in LFS (not recommended — complicates cloning).

**Recommended:** Option 3 (Tauri sidecar) for releases + `.gitignore` the binary in dev. CI downloads it.

---

## Remaining Features

- [ ] **Auto-zoom** — zoom toward click positions (post-processing)
- [ ] **Webcam overlay** — PiP webcam on recording

---

## Testing Expectations

- **Unit tests:** Config parsing, filter chain generation, metadata serialization
- **Integration tests:** Capture → encode → verify output file exists and is valid MP4
- **Manual tests:** Hotkey on all 3 platforms, tray behavior, cursor trail smoothness
- **Sync verification:** Automated A/V sync check runs after every recording

---

## Git Workflow

- **Branch:** `master` for stable, feature branches for dev
- **Commits:** Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`)
- **Releases:** Tag `v0.1.0`, `v0.2.0` etc., GitHub Actions builds installers

---

## Explicit Boundaries

- **Never use Electron** — Tauri only
- **Never use ffmpeg-next crate** — shell out to bundled FFmpeg binary
- **Never block the UI thread** — all Rust commands must be async or spawn threads
- **Never store recordings in app directory** — use user-configured output dir
- **Never add telemetry or analytics** — fully offline
- **Never hardcode hotkeys** — always read from config
- **Never use transparent overlay windows** — WebView2 can't do it on Windows
- **Never scale cursor coordinates** — video is at monitor resolution, coords match
- **Never start data collection before CAPTURE_ARMED** — sync everything to first frame
