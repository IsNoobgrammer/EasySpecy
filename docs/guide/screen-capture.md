# Screen Capture

EasySpecy supports three capture modes: **full screen**, **region**, and **window** capture.

## Full Screen Capture

Records your entire primary monitor at the configured resolution and FPS.

### How It Works

EasySpecy uses platform-native capture APIs:

- **Windows:** Windows Graphics Capture API
- **macOS:** ScreenCaptureKit
- **Linux:** PipeWire

### Configuration

In Settings → Video:

- **Resolution:** 480p, 720p, 1080p
- **FPS:** 24, 30, 60
- **Encoder:** Software (x264) or Hardware (NVENC, QuickSync)

## Region Capture

Record a specific rectangular area of your screen.

### Usage

1. Click the **region select** icon
2. Click and drag to select area
3. Release to confirm

The region is defined by top-left corner (x, y) and dimensions (width, height).

## Window Capture

Record a specific application window, even if you move it.

### Usage

1. Click the **window picker** icon
2. Select the window from the list
3. EasySpecy tracks that window automatically

## Performance Tips

- Use **hardware encoding** if available (NVENC, QuickSync)
- Lower **FPS** (30 vs 60) for smaller file sizes
- Capture **smaller regions** for better performance

---

::: tip
Window capture uses native OS APIs to track the window even when moved or resized.
:::
