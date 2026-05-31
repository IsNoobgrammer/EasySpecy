# EasySpecy — UI/UX Design Analysis & Roadmap

**Date:** 2026-05-31
**Current State:** Brutalist Professional (OKLCH, Inter + JetBrains Mono, Motion/Framer)

---

## Current State Assessment

### What the Other Agent Did Well

| Area | Implementation | Score |
|------|---------------|-------|
| Color system | OKLCH with warm-tinted neutrals (hue 60) | ✅ Excellent |
| Typography | Inter body + JetBrains Mono, 1.25 ratio scale | ✅ Excellent |
| Design tokens | CSS variables, semantic naming (--bg-surface, --text-muted) | ✅ Excellent |
| Dark/Light themes | Full token swap, warm charcoal + warm beige | ✅ Excellent |
| Motion | Framer Motion, expo-out easing, spring physics | ✅ Excellent |
| Accessibility | Focus rings, reduced-motion, selection color | ✅ Good |
| Brutalist style | 2px borders, no rounded corners, monospace labels | ✅ Consistent |
| Settings UX | Accordion sections, conditional sub-fields | ✅ Good |

### What's Missing / Needs Work

| # | Issue | Severity | Category |
|---|-------|----------|----------|
| 1 | **No recording history** — last recording lost on app restart | P1 | UX Flow |
| 2 | **No keyboard shortcuts in UI** — hotkeys work but no visual feedback | P2 | UX Flow |
| 3 | **Status cards are cramped** — 4-column grid on 900px window is tight | P2 | Layout |
| 4 | **No empty state** — first-time user sees nothing helpful | P2 | UX Flow |
| 5 | **Encoding progress is fake** — animated bar but no real % | P2 | Feedback |
| 6 | **No confirmation on destructive actions** — stop recording is instant | P2 | Safety |
| 7 | **Settings labels ALL CAPS** — "RESOLUTION" is harder to read than "Resolution" | P3 | Typography |
| 8 | **No drag-to-reorder webcam overlay** — position is dropdown only | P3 | Interaction |
| 9 | **No recording duration limit** — can fill disk | P2 | Safety |
| 10 | **Toast position overlaps content** — top-right can cover timer | P3 | Layout |

---

## Comprehensive Feature List for UI/UX

### A. Dashboard Improvements

| Feature | Description | Priority |
|---------|-------------|----------|
| **Recording History** | List of past recordings with date, duration, size. Click to open. Persist across sessions. | P1 |
| **Quick Stats** | Total recordings, total duration, total disk used | P2 |
| **Disk Space Warning** | Show remaining space when <2GB free | P2 |
| **Recording Timer** | Show elapsed time WITH estimated file size | P2 |
| **Keyboard Shortcut Overlay** | "?" key shows all shortcuts in a modal | P3 |
| **Drag-and-Drop Region** | Visual region selector overlay for Region mode | P1 |
| **Window Picker** | List of open windows for Window capture mode | P1 |
| **Monitor Picker** | Dropdown for multi-monitor setups | P2 |

### B. Recording Flow Improvements

| Feature | Description | Priority |
|---------|-------------|----------|
| **Real Progress** | Show actual encoding progress via FFmpeg -progress pipe | P1 |
| **Countdown** | 3-2-1 countdown before recording starts (optional) | P2 |
| **Pause Indicator** | Red border on entire window when recording, yellow when paused | P1 |
| **Stop Confirmation** | "Stop recording?" if <3s elapsed (prevent accidental) | P2 |
| **Auto-Stop Timer** | Set max recording duration (30m, 1h, 2h, unlimited) | P2 |
| **Sound Feedback** | Optional beep on start/stop (accessibility) | P3 |
| **System Notification** | Native OS notification on save (not just in-app toast) | P2 |

### C. Settings Improvements

| Feature | Description | Priority |
|---------|-------------|----------|
| **Search** | Cmd+K style search across all settings | P3 |
| **Keyboard Navigation** | Tab through all fields, Enter to save | P2 |
| **Reset to Defaults** | "Reset" button per section | P2 |
| **Import/Export Config** | JSON export/import of settings | P3 |
| **Validation** | Show errors inline (e.g., invalid hotkey format) | P2 |
| **Preview** | Show what webcam overlay looks like in settings | P3 |
| **Tooltip Help** | Hover (?) icons for complex options | P3 |

### D. Visual Improvements

| Feature | Description | Priority |
|---------|-------------|----------|
| **Animated Logo** | Logo pulses/breathes when recording | P3 |
| **Recording Border** | Full window border glow when recording | P1 |
| **Micro-Interactions** | Button press feedback, card hover lift | P2 |
| **Skeleton Loading** | Replace "LOADING CONFIG..." with skeleton | P3 |
| **Empty State** | First-run welcome with quick setup | P2 |
| **Status Bar** | Bottom bar with disk space, FPS counter, audio level | P2 |
| **Audio Level Meter** | Real-time mic level visualization during recording | P1 |

---

## Color Scheme & Palette

### Current (Brutalist Warm)

```
Dark Theme:
  Base:     oklch(0.14 0.008 60)  — warm charcoal
  Surface:  oklch(0.18 0.008 60)  — slightly lighter
  Elevated: oklch(0.22 0.008 60)  — cards, panels
  Text:     oklch(0.93 0.01 60)   — warm off-white
  Muted:    oklch(0.45 0.008 60)  — secondary text
  Primary:  oklch(0.65 0.2 25)    — warm red/orange
  Success:  oklch(0.65 0.18 145)  — green
  Danger:   oklch(0.62 0.22 25)   — red (same hue as primary!)
  Info:     oklch(0.65 0.12 250)  — blue

Light Theme:
  Base:     oklch(0.96 0.012 80)  — warm cream
  Surface:  oklch(0.98 0.008 80)  — near-white
  Text:     oklch(0.18 0.015 60)  — warm dark
```

### Issues with Current Palette

1. **Primary and Danger are the same hue (25)** — red/orange for both "record" and "stop" is confusing. Danger should be distinct.
2. **No accent differentiation** — primary, danger, and warning are all warm (hue 25-80). No cool accent for contrast.
3. **Info blue (250) feels disconnected** — the only cool color in an otherwise warm palette.

### Recommended Palette (Refinement)

Keep the warm brutalist identity but add clarity:

```
Record Button:  oklch(0.65 0.22 25)   — warm red (keep)
Stop Button:    oklch(0.62 0.22 25)   — same red (keep, it's intuitive)
Primary Accent: oklch(0.65 0.18 30)   — slightly more orange (for CTAs)
Danger:         oklch(0.55 0.25 25)   — deeper red (for destructive actions)
Success:        oklch(0.65 0.18 145)  — green (keep)
Warning:        oklch(0.72 0.15 80)   — amber (keep)
Info:           oklch(0.65 0.12 250)  — blue (keep)
Encoding:       oklch(0.65 0.15 250)  — blue (matches info)
```

**Key change:** Use the primary red for record/stop (intuitive), but use a deeper red for actual destructive actions (delete config, etc.). The recording button IS red by convention — don't fight it.

---

## Typography Recommendations

### Current
- Body: Inter 400/500/600/700
- Mono: JetBrains Mono Variable
- Scale: 1.25 ratio
- Labels: ALL CAPS monospace

### Recommendations

| Change | Why |
|--------|-----|
| **Keep ALL CAPS for section headers** (VIDEO, AUDIO) | Works with brutalist style |
| **Use Sentence Case for field labels** (Resolution, Frame Rate) | Easier to scan, less aggressive |
| **Reduce monospace usage** — use body font for values | Monospace for data, body for labels |
| **Add font-weight: 350 for dark mode body** | Lighter weight on dark bg reduces visual weight |
| **text-wrap: balance** on headings | Prevents orphan words |

---

## User Flow Improvements

### Current Flow
```
Launch → Dashboard → Click Record → Click Stop → Toast → Dashboard
```

### Proposed Flow
```
Launch → Dashboard (with history)
  → Click Record (optional 3-2-1 countdown)
  → Recording (red border, timer, audio meter)
  → Click Stop → Encoding (real progress bar)
  → Success (toast + system notification + history update)
  → Dashboard (with new recording in history list)
```

### Onboarding Flow (First Run)
```
Launch → Welcome
  → "Choose output directory" (with default suggestion)
  → "Test microphone" (playback test)
  → "Set hotkeys" (with defaults shown)
  → "Start recording" (guided first recording)
```

---

## Interaction Design Improvements

### Button States (All 8)

| State | Current | Recommended |
|-------|---------|-------------|
| Default | ✅ Styled | Keep |
| Hover | ✅ scale(1.05) | Keep |
| Focus | ✅ outline | Keep |
| Active | ✅ scale(0.95) | Keep |
| Disabled | ✅ opacity-0.4 | Keep |
| Loading | ✅ Spinner | Keep |
| Success | ❌ Missing | Green flash + checkmark |
| Error | ❌ Missing | Red flash + shake |

### Recording Button Choreography

```
Idle:     Green circle (record icon) — breathe animation
Click:    Scale down (0.94) → transition to red square (stop icon)
Recording: Red square — pulse glow animation
Stop:     Scale down → transition to spinner (encoding)
Encoding: Blue spinner — indeterminate progress bar
Done:     Green checkmark flash → back to idle
```

### Toggle Improvements

Current toggle is brutalist square. Consider:
- Keep brutalist style (no rounded corners)
- Add inner shadow when OFF
- Add glow when ON
- Spring animation on knob (already implemented ✅)

---

## Accessibility Checklist

| Item | Status | Action |
|------|--------|--------|
| Focus rings | ✅ Done | Keep |
| Reduced motion | ✅ Done | Keep |
| Color contrast | ✅ OKLCH ensures good contrast | Verify AA |
| Keyboard navigation | ⚠️ Partial | Add Tab order to all fields |
| Screen reader labels | ❌ Missing | Add aria-labels |
| Touch targets | ✅ 44px+ on buttons | Verify all |
| Error messages | ⚠️ Toast only | Add inline errors |
| High contrast mode | ❌ Missing | Add @media (prefers-contrast) |

---

## Implementation Priority

### Phase 1 (Immediate — fix what's broken)
1. Recording history (persist past recordings)
2. Real encoding progress (FFmpeg -progress)
3. Audio level meter (real-time visualization)
4. Recording border glow (visual state feedback)
5. System notification on save

### Phase 2 (Next — improve UX)
6. Empty state / onboarding
7. Region select overlay
8. Window picker
9. Settings validation
10. Recording duration limit

### Phase 3 (Polish)
11. Keyboard shortcut overlay (?)
12. Import/export config
13. Search settings
14. Audio test in settings
15. Webcam preview in settings
