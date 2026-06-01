# Why the Webcam Was Broken: A Tale of Two Bugs and a Privacy Nightmare

## The Problem

EasySpecy's webcam feature had three symptoms that looked unrelated but shared a single root cause:

1. **The webcam preview worked perfectly** — you could see yourself in the overlay editor
2. **The final recorded video had no webcam overlay** — the PiP was completely absent
3. **The webcam never turned off** — once you opened the preview, the camera light stayed on until you killed the app

All three trace back to a single line of React code.

---

## The Stale Closure Bug

In `WebcamPreview.tsx`, the webcam was started like this:

```tsx
const [stream, setStream] = useState<MediaStream | null>(null);

useEffect(() => {
    let mounted = true;
    navigator.mediaDevices.getUserMedia({ video: true })
      .then((s) => {
        if (!mounted) return;
        setStream(s);                    // ← stores stream in React state
        setCameraReady(true);
      });
    return () => {
      mounted = false;
      stream?.getTracks().forEach((t) => t.stop());  // ← BUG
    };
}, []);  // ← empty dependency array = runs once
```

The cleanup function (the `return () => {...}`) captures `stream` from the **initial render closure**. At that moment, `stream` is `null` — the default value from `useState`. The empty dependency array `[]` means this effect and its cleanup are created exactly once, during the first render.

When `getUserMedia` resolves and calls `setStream(s)`, React state updates and triggers a re-render. But the cleanup function was already created with the old closure. It still sees `stream` as `null`.

**Result:** `null?.getTracks()` evaluates to `undefined`. No `.forEach()`. No `.stop()`. The MediaStream tracks are never stopped. The webcam device is never released.

### Why This Is a Privacy Issue

On Windows, when a browser (or WebView2) acquires the webcam via `getUserMedia`, the camera indicator light turns on. This is a hardware-level signal that the camera is active. The light is supposed to guarantee that if you see it, something is recording.

But with this bug, the light stays on even after the user closes the preview. There's no recording happening, no frame being captured, but the device is still held open. The user has no way to release it except killing the entire application.

For users who are privacy-conscious — which should be everyone — this is unacceptable. A webcam indicator that stays on when nothing is recording trains users to ignore the light, which defeats its entire purpose.

---

## Why the Final Video Had No Webcam Overlay

This is the second-order effect of the same bug.

EasySpecy has two separate webcam systems:

| System | Technology | When Active | Purpose |
|--------|-----------|-------------|---------|
| Preview | `navigator.mediaDevices.getUserMedia` (browser) | When WebcamPreview is open | Show user what webcam looks like |
| Recording | `nokhwa` (Rust crate) | During recording | Capture frames for FFmpeg overlay |

Both systems access the **same physical webcam device**. On Windows, only one process/thread can hold the device at a time.

Here's what happened:

1. User opens webcam preview → `getUserMedia` acquires the device ✓
2. User closes preview → cleanup runs, but **doesn't release the device** (stale closure bug)
3. User starts recording with webcam enabled
4. Rust backend calls `nokhwa::Camera::new()` → tries to open the device
5. **Device is still held by the browser** → nokhwa fails
6. Error is logged but swallowed: `tracing::error!("Webcam capture error: ...")`
7. `WEBCAM_FRAME_COUNT` stays at 0
8. `stop_webcam_capture()` returns `None` (because frame_count == 0)
9. Compositing is silently skipped
10. Final video is produced without any webcam overlay

The user gets zero feedback that the webcam failed. No toast, no warning, no error dialog. The video just... doesn't have the webcam.

---

## The Fix

Replace `useState` with `useRef` for the MediaStream:

```tsx
const streamRef = useRef<MediaStream | null>(null);

useEffect(() => {
    let mounted = true;
    navigator.mediaDevices.getUserMedia({ video: true })
      .then((s) => {
        if (!mounted) {
          s.getTracks().forEach(t => t.stop());  // release if already unmounted
          return;
        }
        streamRef.current = s;  // ← stored in ref, not state
        setCameraReady(true);
      });
    return () => {
      mounted = false;
      streamRef.current?.getTracks().forEach((t) => t.stop());  // ← always current
      streamRef.current = null;
    };
}, []);
```

The key difference: `useRef` stores the value in a mutable `.current` property that persists across renders. The cleanup function always reads the latest value from `streamRef.current`, not from a stale closure. When the component unmounts, the cleanup runs and **actually stops the tracks**.

---

## Lessons Learned

### 1. React closures capture values, not references

When you write `stream?.getTracks()` inside a `useEffect` cleanup with `[]` dependencies, `stream` is captured from the render where the effect was created. It will never update. For values that change over time and need to be accessed in cleanup, use `useRef`.

### 2. Silent error swallowing is a design failure

When nokhwa fails to open the webcam, the error is logged but nothing else happens. The frame count stays at zero, and the compositing step is silently skipped. The user has no idea the webcam overlay wasn't applied. Errors should surface to the user — a toast, a warning, something.

### 3. Device contention between browser and native is real

When building desktop apps with webviews (Tauri, Electron), remember that the browser's `getUserMedia` and native camera libraries (nokhwa, OpenCV) compete for the same hardware. Always release browser streams before starting native capture.

### 4. Privacy indicators should be trustworthy

If the camera light is on, the user should be confident that something is actively using the camera. A bug that leaves the light on when nothing is recording erodes that trust. Test your cleanup paths.

---

*Written while debugging EasySpecy — a screen recorder built with Tauri 2, React, and FFmpeg.*
