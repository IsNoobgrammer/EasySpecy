# EasySpecy — User Flow Document

**Version:** 1.0
**Date:** 2026-05-31

---

## 1. Primary User Journey: Record a Demo

```mermaid
flowchart TD
    A[Launch EasySpecy] --> B[Main Dashboard]
    B --> C{Configure settings?}
    C -->|Yes| D[Settings Panel]
    D --> B
    C -->|No| E[Press Record Hotkey or Click Button]
    E --> F{Recording mode?}
    F -->|Full Screen| G[Start capturing entire screen]
    F -->|Region| H[Drag to select area] --> G
    F -->|Window| I[Select window from list] --> G
    G --> J[Recording active: overlay shows timer]
    J --> K{User actions during recording}
    K -->|Clicks on screen| L[Click events logged for auto-zoom]
    K -->|Moves cursor| M[Cursor positions logged for trail]
    K -->|Presses pause hotkey| N[Recording paused] --> J
    K -->|Presses stop hotkey| O[Stop capturing]
    O --> P{Auto-zoom enabled?}
    P -->|Yes| Q[Post-process: apply zoom + cursor effects]
    P -->|No| R[Encode raw to MP4]
    Q --> R
    R --> S[Save to output directory]
    S --> T[Copy path to clipboard]
    T --> U[Tray notification: Saved!]
    U --> V{Continue recording?}
    V -->|Yes| E
    V -->|No| B
```

---

## 2. Onboarding Flow

```mermaid
journey
    title First-time User Experience
    section Install
      Download installer: 5: User
      Run installer: 5: User
      App launches: 5: System
    section First Setup
      Welcome screen: 4: EasySpecy
      Select output directory: 4: User
      Test microphone: 4: User
      Test webcam: 4: User
      Set hotkeys: 3: User
      Choose default resolution/FPS: 4: User
    section First Recording
      Press hotkey: 5: User
      Recording starts: 5: EasySpecy
      Press stop: 5: User
      File saved notification: 5: EasySpecy
      Open file: 5: User
```

---

## 3. Edge Cases & Error States

| Scenario | User Sees | System Does |
|----------|-----------|-------------|
| No webcam connected | Webcam toggle greyed out, tooltip: "No camera detected" | Disables webcam overlay, continues without |
| Mic unplugged mid-record | Tray warning: "Audio device lost" | Continues video-only, marks file as no-audio |
| Disk fills up during record | Tray notification: "Low disk space, stopping" | Auto-stops, saves partial recording |
| Hotkey conflicts with another app | Settings: "Hotkey unavailable, choose another" | Rejects binding, keeps previous |
| User closes window while recording | App minimizes to tray, recording continues | Tray shows recording state |
| FFmpeg encoding fails | Toast: "Encoding failed, raw file saved at..." | Preserves raw .yuv + .wav, offers manual encode |
| 0-second recording (accidental) | Silently discarded, no file saved | Cleanup temp files, no notification |
| Region select cancelled (Escape) | Returns to dashboard | No recording started |
| Multiple monitors | Settings: "Select monitor" dropdown | Enumerates via scap, user picks one |

---

## 4. Screen Inventory

| Screen | Route | Auth | Description |
|--------|-------|------|-------------|
| Dashboard | `/` | None | Main view: record button, recent recordings, quick settings |
| Settings | `/settings` | None | All configuration options |
| Recording Overlay | Floating window | None | Minimal timer + stop button during recording |
| Region Selector | Fullscreen overlay | None | Transparent overlay for drag-to-select |
| Webcam Config | `/settings/webcam` | None | Webcam device selection, preview, position/size |
| About | `/about` | None | Version, license, links |
