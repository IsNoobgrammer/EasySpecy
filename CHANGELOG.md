# Changelog

All notable changes to EasySpecy will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

## [1.2.1] - 2026-10-03

### Fixed
- **Recording failed on Windows 10 ("Setting a minimum update interval is not supported").** The frame-rate cap is a Windows 11-only capture setting, and asking for it made capture refuse to start, so every recording and preview failed. It's now only requested where Windows supports it. Elsewhere, extra frames are dropped in software, so 30 fps still records 30 fps.
- **The app said High was the default quality, but Medium is.** The dashboard, Compare quality and README now say Medium. Benchmarks were re-measured at Medium: H.264 ~24 MB/min, H.265 ~18 MB/min, Smaller file ~5.5 MB/min.
- **Compare quality showed the same picture for every bitrate.** Each tile really was encoded at its own bitrate (Low ≈ 150 kb/s → Ultra ≈ 500 kb/s on the crop), but the frame shown was taken 1.5 s in. By then smooth scrolling had let even Low converge to near-lossless (PSNR 42 vs 53 dB). Tiles now show the moment right after the screen changes (0.33 s), where the difference is real and visible (29 vs 46 dB): blocky, smeared text on Low, clean on High and Ultra.

### Changed
- **Faster local builds:** `npm run build:fast` uses a new `fast` Cargo profile (incremental, 256 codegen units for the app crate, dependencies still fully optimized). A Rust rebuild after an edit takes ~22 s instead of ~85 s, and the output goes to `src-tauri/target/fast/`. Release CI is unchanged.

## [1.2.0] - 2026-10-03

### Fixed
- **Noise reduction mostly just lowered the volume.** RNNoise expects 48 kHz audio at 16-bit scale and was fed ±1.0 floats, which it heard as near-silence. It's now fed the right scale, resampled to 48 kHz, with its one-frame (10 ms) delay removed. Tested: voice keeps its level, and fan-like background noise drops by ~45 dB.
- **The microphone chosen in Settings was ignored.** Recording and the level meter now use it, and fall back to the default if it's unplugged.
- **Mic-only recordings skipped gain and noise reduction.** They now get the same processing as Mic + System.
- **Audio drifted on long recordings.** Each source is fitted to the true recording length: sound-card clock error is stretched out, and gaps are filled with silence. Audio also measured its length after a 50 ms flush wait, making it ~100 ms too long.
- **A/V drift after pausing on a still screen.** Windows only sends a frame when the screen changes, so a segment's video ended at its last change and the audio slid later after every pause. Each segment now gets its true timeline length when joined, so its last frame holds as it did on screen. The final file uses the real recording length (`-t`) instead of being cut to the video (`-shortest`), so talking over a still ending is no longer lost.
- **Pause / Stop on a completely still screen.** These are handled on the next frame, so Stop could time out after 15 s and lose the last segment. The overlay now flips one invisible pixel to force a frame, and Stop records that frame so the video runs right up to the Stop press.
- **Stop pressed while a recording was still starting (~1–2 s) was ignored**, so a quick start → stop kept recording. It's now queued and runs as soon as capture is live.
- **Region / window recording only worked on the primary monitor.** The selector opens on the monitor under the cursor. Region and window recordings capture whichever monitor holds the selection, and the effects overlay follows it.
- **Cursor packs stayed applied after a crash.** A marker records that a pack is on, and the next launch restores the system cursors. Exiting the app restores them too.
- **Keyboard capture could hang on a quick start/stop.** Stop could run before the hook thread had a message queue, so its quit message was lost and the join blocked forever. Startup now waits for the hook thread, Stop retries and is capped at 2 s, a failed hook install no longer deadlocks, and restarting a capture shuts the old threads down instead of leaking them.
- **A corrupt or outdated config reset every setting.** Only the invalid values reset, and the original is kept as `config.toml.bad`.
- **Changing any setting while running as admin restarted the app.** It now relaunches only when the game-capture toggle itself changes, and never mid-recording.
- **Leaving Settings without pressing Save lost the changes.** Settings now saves automatically half a second after each change, and when you leave the page.

## [1.1.0] - 2026-10-02

### Added
- **Audio panel on the dashboard** — Separate Microphone / System audio toggles with live level bars. Each source says plainly when it isn't being recorded, and a silent source gets a hint ("No sound from your mic — is it muted?", "Nothing playing — only sound your PC plays gets recorded"). The Saved card says when the audio came out silent, and starting with audio off gives a notice.
- **Preview 10s** — Records 10 s exactly like a real recording (webcam, trail, keys, audio) to one temporary `preview.mp4` and plays it in the app. Each preview replaces the last one. Previews never enter history and are deleted on exit (and on the next launch after a crash).
- **Encoder scan** (Settings → Video) — Benchmarks every encoder on a 1080p clip. It reports live-capture H.264/HEVC support and, per FFmpeg encoder, works or not, speed (fps) and peak RAM. The encoder pickers then list only working encoders, mark which ones save instantly (stream copy) versus re-encode, and suggest a default.
- **"Smaller file" option** (dashboard + Settings) — Re-encodes after stopping with x264 veryfast at the chosen quality, giving 4–5× smaller files (30 s 1080p: ~14.6 → 3.0–3.9 MB) in exchange for a few seconds of save time. It always uses x264: on screen content it matches x265's size at 2.6× the speed, and plays everywhere.
- **Compare quality** — Renders Low / Medium / High / Ultra and Smaller file from a scrolling crop of the user's own screen. It uses the same Media Foundation hardware encoder as live capture, at equal bits-per-pixel, with ≈ MB/min for each. There's a custom-bitrate slider, a click-to-zoom, and "Use this" to apply. Real progress milestones drive a smooth progress bar.

### Changed
- **Dashboard encoder choice is just H.264 / H.265** — AV1, VP9, NVENC and Mobile moved to Settings → Video (advanced). Live capture already uses the GPU's hardware encoder through Media Foundation, so FFmpeg GPU encoders only matter for re-encodes.
- **Quality preview speed: 20 s → ~8 s** — It encodes only the shown crop. All hardware bitrates share one FFmpeg process, because hardware sessions take ~1.2 s each to start and the driver starts them one by one. The CPU sample runs after them rather than starving them.
- **Spawned FFmpeg opts out of Windows 11 power throttling (EcoQoS)**, so windowless encodes aren't parked on efficiency cores.
- **Faster re-encodes** — Measured per minute of 1080p30 screen video:
  - AV1 preset 6 → 10: 38.7 s → 23.0 s, same size.
  - VP9 good/4 → realtime/8: 80.6 s → 12.9 s, ~30% larger.
  - Region crop x264/x265 fast → veryfast: 8.6 s → 6.3 s, and smaller.
- **Post-processing benchmark** — `examples/bench_post.rs` runs the real final pass (`encode_final_with`) over prepared intermediates, so the whole fps × encoder × duration matrix runs without re-recording.
- **Dashboard** — 4 presets (Mode, Frame rate, Encoder, Quality) instead of 6. The no-op Resolution card is gone, and Audio moved into the new panel. The header shows the capture mode instead of the unused config resolution.
- **Settings → Video** — Resolution shows the real native capture size. The GPU toggle and hard-coded encoder list are replaced by the scan-driven picker (choosing NVENC turns GPU encoding on automatically).
- **Lower memory while recording** — The hidden main window's WebView is set to `MemoryUsageTargetLevel = Low` (renderer 105 → 57 MB). Audio buffers are only reserved for sources that are recorded.
- **Capture uses BGRA** (the DWM-native format) instead of RGBA.
- **CPU while recording roughly halved** — Measured with trail + webcam + keyboard overlay: mouse moving 184% → 90% of one core, mouse still 112% → 45%.
  - **Hidden main window stopped animating** — Rust now tells the UI when the window is hidden in the tray. The WebView flags the overlay needs had kept it rendering timers and pulse animations at full speed: 20–26% → ~1%.
  - **Overlay heartbeat removed** — It forced a full-screen re-composite every frame.
  - **Overlay loop** — Sleeps when there's nothing to draw, and is capped at the capture FPS.
  - **Audio monitor pauses while recording** — The recording streams feed the level meter, so system audio is no longer captured twice.
  - **Webcam CSS filter** — Skipped when it's a no-op.
  - **Current state** — All features on: 184% → 65% of a core with the mouse moving, 112% → ~49% still. Of the Rust core's ~20–25%, ~5% is our own threads; the rest is Windows' Media Foundation encode pipeline.
- **Repo language** — `.gitattributes` marks the docs site as documentation, so GitHub reports the Rust engine as the main language.

### Fixed
- **Live bitrate was sized from the Resolution setting** — Output isn't scaled, so a 1280×720 setting starved 1080p captures to 44% of their bitrate. Bitrate now follows the real captured size.
- **Live encoder over-sized files** — It's constant-bitrate, and used a table meant for CRF encoding: "High" 1080p30 was 15 Mbps ≈ 112 MB/min. A screen-content table (bits per pixel per frame; 60 fps ≈ 1.5×, not 2×) gives ~5 Mbps ≈ 35 MB/min (verified on a real recording: 4,933 kb/s). The dashboard size estimate uses the same numbers.
- **Pause/resume concat could fail** — The concat list was written before its folder was guaranteed to exist.
- **"Capture stopped unexpectedly (0xC00D4A44)" on a fast pause → resume → pause** — The second pause tried to finalise a segment that had received no frames. That error killed the capture and cost the last seconds. Empty segments are now kept for the next resume, a segment that fails to finalise no longer stops the recording, and pause cancels a pending resume. Before, capture could resume by itself while the UI showed "paused".

### Benchmarks

**Time to save** is from pressing Stop to the file being ready to share. All runs are **1080p at 30 fps** (the default) with **High** quality, mic + system audio, on an i5-12450H laptop using Intel QSV for live encoding. The test video is real screen content (an editor and a terminal), half still and half scrolling. Scrolling is the hardest case for file size.

**Main choices on the dashboard**

| Output | Time to save (1 min) | Time to save (2 min) | Size per minute |
|---|---|---|---|
| **H.264** (default, plays everywhere) | ~2 s | ~4 s | ~37 MB |
| **H.265** (~25% smaller, newer players) | ~2 s | ~3 s | ~28 MB |
| **H.264 + Smaller file** | ~11 s | ~24 s | **~6.5 MB** |
| **H.265 + Smaller file** | ~12 s | ~22 s | **~6.5 MB** |
| H.264 with a pause/resume | ~2 s | ~4 s | ~37 MB |

H.264 and H.265 save almost instantly because the video is already encoded on the GPU while you record. Saving just copies it into the final file. **Smaller file** re-encodes after you stop: the file is about 5× smaller, but saving takes longer, roughly 11 s per minute of video. It always outputs H.264, which comes out the same size as H.265 for screen content but encodes much faster and plays everywhere.

**Quality presets** (live encoder, video only, per minute at 1080p30)

| Quality | H.264 | H.265 |
|---|---|---|
| Low | ~13 MB | ~10 MB |
| Medium | ~22 MB | ~17 MB |
| **High** (default) | ~36 MB | ~27 MB |
| Ultra | ~53 MB | ~40 MB |

Use **Compare quality** in the app to see each one on your own screen before choosing.

**Advanced encoders** (Settings → Video encoder; these always re-encode)

| Output | Time to save (1 min) | Time to save (2 min) | Size per minute |
|---|---|---|---|
| AV1 (CPU, SVT-AV1) | ~32 s | ~56 s | ~5 MB |
| VP9 (CPU) | ~22 s | ~28 s | ~17 MB |
| Region / window crop 1280×720, H.264 | ~8 s | ~15 s | ~4.7 MB |
| Region / window crop 1280×720, H.265 | ~17 s | ~39 s | ~4.5 MB |

- **NVENC H.264/H.265:** saves just as fast as H.264/H.265 above, because live capture already produced the video.
- **AV1 NVENC:** needs an RTX 40-series GPU.
- **Smaller file and crop sizes depend on content.** A mostly still screen comes out smaller.

**While recording**, with cursor trail + webcam + keyboard overlay all on, the app uses about **65% of one CPU core** with the mouse moving and **~49%** when it's still. Most of that is Windows' own Media Foundation encoder.

<sub>Reproduce: `cargo run --release --example bench_post -- <intermediates> <out>` in `src-tauri` (see `examples/bench_post.rs`). It runs the app's real save pass over pre-encoded clips, so no re-recording is needed.</sub>

## [1.0.0] - 2026-10-02

**Highlights:** A recording is ready to share about **1 second after you press stop** (it took ~2 minutes for a 1-minute video in 0.1.5). Capture now works on GPUs without HEVC (#2). Passwords are masked in the keyboard overlay. Window recording is new, and region recording works properly. The app uses ~6× less CPU while idle.

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

### UI
- **Idle CPU: 32% → 5% of a core** — measured on the Dashboard; the WebView GPU process went from ~10% to 0.
  - **Re-render storm** — The whole app re-rendered 20×/s from audio-level polling, even hidden in the tray. Now only the meters subscribe, polling pauses while the window is hidden, and unchanged levels are skipped.
  - **Sidebar meter** — Its rAF loop stops when the bar settles. It used to keep Chromium compositing at 60 Hz forever.
  - **Settings preview** — It drew canvas `blur()` filters forever. It now uses the overlay's multi-pass strokes (Rule 4 parity), caches the background, and pauses when scrolled off.
  - **Expensive CSS removed** — Full-window `blur(120px)` glows became radial gradients, `backdrop-filter` on opaque surfaces is gone, and the record button's infinite `box-shadow` animation became an opacity pulse.
- **Saved card** — Stays after every recording, with Open / Show in folder / Copy path / two-step Delete. The file name shows on the card.
- **Clear states** — A new "Starting…" phase means a recording can't start twice. The header shows Starting / Recording / Saving. NEW RECORDING and the record button are disabled while busy. Error toasts stay until dismissed.
- **Layout** — Content no longer clips above the window (`safe center`).
- **Accessibility** — Accessible names on the record and settings buttons. Sidebar nav items are real buttons.
- **History** — Shows the real output resolution, not the config value.
- **Fixed** — The progress poll leaked forever when stop failed. Leaving Settings killed the sidebar meter.

### Removed
- **Dead post-processing (about 4,500 lines)** — The Rust webcam capture (PNG sequence + FFmpeg composite), effect baking (`apply_effects`) and the cursor/click/keystroke metadata that only fed them. The webcam, trail and keys are drawn live by the overlay and captured directly. This also removes the `.meta.json` that briefly held every keystroke, and a thread spawned per click.
- **Unused crates** — `nokhwa`, `rayon`, `font8x8`, `miniz_oxide`, `crc32fast`.
- **Dead files** — `webcam.html`, both `region-select.html`, `Customization.tsx`, `RecordingOverlay.tsx`.
- **Auto-zoom UI** — Never implemented; planned for 2.x.
- **Hardcoded developer paths** — Removed from the sync verifier and cursor packs.

### Changed
- **Stop → file ready: 111 s → 1.2–1.5 s** for a 60 s recording with pause, webcam, trail and keyboard overlay (i5-12450H / RTX 3050). The final pass reads pause segments directly (one FFmpeg run, not concat + mux), per-segment probe launches are gone, the verifier reuses the bundled ffmpeg instead of a cold ffprobe, and ffmpeg is pre-warmed while recording.
- **Webcam picker uses the browser's camera list** — The same ordering the overlay uses, so preview and recording show the same camera.
- **Effects badge** — "Post-processing" → "Live".
- **Single-encode pipeline** — When the live-encoded codec matches the selected output, the final pass only muxes audio (`-c:v copy`) and no longer re-encodes the whole video. Region crop now runs inside the one final encode instead of a separate libx264 pass.
- **Live encoder respects settings** — Bitrate and frame rate now follow the quality and FPS settings instead of a fixed 15 Mbps @ 60 fps.
- **No redundant FFmpeg probes on stop** — The single-segment pre-check and the duration probe on the copy path are skipped; each was a ~1 s cold start of the bundled ffmpeg.
- **Noise gate is O(N)** — It used a sliding-window sum instead of re-summing a 10 ms window for every sample.
- **Effects overlay always created** — It now always runs during recording.
- **Default encoder is now H.264** for new installs, which keeps the fast copy path on every machine.

### Known issues (planned for 1.1)
- Noise reduction (RNN) gets audio at the wrong scale, so it mostly attenuates.
- The mic device selection is ignored and the default input is used.
- Mic-only mode skips mic gain and denoise.
- Slow audio/video clock drift on long recordings (sub-second, not yet corrected).
- The Resolution setting doesn't scale output; recordings are always at monitor resolution.
- The sync verifier reports ~100 ms "drift" false positives (one-frame tolerance).

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
