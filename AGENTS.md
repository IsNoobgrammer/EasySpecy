# EasySpecy — AI Agent Configuration

**Project:** EasySpecy — Free, cross-platform screen recorder
**Stack:** Tauri 2 + Rust + React + FFmpeg
**Current Phase:** 0 — Foundation

---

## Project Context

EasySpecy is a desktop screen recorder built with Tauri. The Rust backend handles all performance-critical work (capture, encoding, effects). The React frontend is a thin GUI layer. FFmpeg is bundled for encoding and post-processing.

**Docs:** See `docs/PRD.md`, `docs/DESIGN.md`, `docs/ARCH.md`, `docs/IMPL-PLAN.md`

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

---

## File Structure

```
EasySpecy/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── capture/        # scap wrapper, frame capture loop
│   │   ├── audio/          # cpal wrapper, audio capture loop
│   │   ├── config/         # TOML config read/write
│   │   ├── commands/       # Tauri IPC commands
│   │   ├── postprocess/    # FFmpeg invocation, zoom/trail filters
│   │   └── tray/           # System tray setup
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── Dashboard.tsx
│   │   ├── Settings.tsx
│   │   ├── RecordingOverlay.tsx
│   │   ├── RegionSelector.tsx
│   │   └── WebcamPreview.tsx
│   ├── stores/
│   │   └── recording.ts    # Zustand store
│   └── lib/
│       └── tauri.ts        # IPC wrappers
├── resources/
│   ├── ffmpeg.exe          # Windows
│   ├── ffmpeg              # macOS/Linux
│   └── icon.ico
├── docs/
│   ├── PRD.md
│   ├── DESIGN.md
│   ├── ARCH.md
│   ├── IMPL-PLAN.md
│   └── FLOW.md
├── AGENTS.md               # This file
├── README.md
├── LICENSE
└── .github/
    └── workflows/
        └── build.yml       # CI for all 3 platforms
```

---

## Testing Expectations

- **Unit tests:** Config parsing, filter chain generation, metadata serialization
- **Integration tests:** Capture → encode → verify output file exists and is valid MP4
- **Manual tests:** Hotkey on all 3 platforms, tray behavior, webcam on/off
- **No E2E framework yet** — manual testing is fine for MVP

---

## Git Workflow

- **Branch:** `main` for stable, feature branches for dev
- **Commits:** Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`)
- **PRs:** Self-review OK for solo dev, require CI pass
- **Releases:** Tag `v0.1.0`, `v0.2.0` etc., GitHub Actions builds installers

---

## Explicit Boundaries

- **Never use Electron** — Tauri only
- **Never use ffmpeg-next crate** — shell out to bundled FFmpeg binary
- **Never block the UI thread** — all Rust commands must be async or spawn threads
- **Never store recordings in app directory** — use user-configured output dir
- **Never add telemetry or analytics** — fully offline
- **Never hardcode hotkeys** — always read from config
