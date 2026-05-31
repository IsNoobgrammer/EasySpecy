# EasySpecy — Logo System & Design Specification

---

## 1. Logo Concept: The Capture Frame

### Design Philosophy

The EasySpecy logo is built on a single, instantly recognizable metaphor: **the screen capture frame** — the bracket corners that appear when you select a region to record.

This isn't arbitrary. Every time a user selects a screen region, they see these brackets. The logo IS the product's most fundamental interaction.

### Conceptual Layers

```
Layer 1: Capture Brackets (⌜ ⌝ ⌞ ⌟)
         → "I capture what's on screen"

Layer 2: Central Recording Dot
         → "I'm active, alive, recording"

Layer 3: Subtle Radial Energy
         → "Precision focus, zoom capability"
```

### Why This Works

- **Instant recognition**: Users see capture brackets daily. The logo triggers product association immediately.
- **Scalable**: The bracket + dot motif reads clearly from 16px favicon to 1024px app icon.
- **Animatable**: Brackets can expand/contract (recording start/stop). Dot can pulse (active state).
- **Distinctive**: No other screen recorder uses this exact combination. OBS uses a circle, Loom uses a gradient blob, Screen Studio uses a camera lens.

---

## 2. Logo Anatomy

### Primary Mark (Icon)

```
┌─────────────────────────────────┐
│                                 │
│   ⌜─────────────────────⌝      │
│   │                     │      │
│   │                     │      │
│   │         ●           │      │  ← Recording dot (accent color)
│   │                     │      │
│   │                     │      │
│   ⌞─────────────────────⌟      │
│                                 │
└─────────────────────────────────┘
```

**Construction:**
- Outer bounding box: 1024 × 1024px (design space)
- Safe zone: 128px inset on all sides (live area: 768 × 768px)
- Bracket stroke: 64px width, rounded caps (radius: 32px)
- Bracket arm length: 180px (each arm of the L-shape)
- Corner radius on bracket outer edge: 48px
- Central dot: 120px diameter, perfectly centered
- Dot glow: radial gradient, 200px spread, 40% opacity

### Proportions (Golden Ratio Aligned)

- Bracket thickness : Dot diameter = 1 : 1.875 (≈ φ)
- Bracket arm : Total width = 1 : 4.27 (≈ φ³ normalized)
- Dot : Live area = 1 : 6.4 (prominent but not dominant)

### Color Application on Logo

| Element | Color | OKLCH |
|---------|-------|-------|
| Brackets | White/Silver | `oklch(0.92 0.005 265)` |
| Central dot | Brand Accent (Emerald) | `oklch(0.78 0.18 160)` |
| Dot glow | Accent at 40% | `oklch(0.78 0.18 160 / 0.4)` |
| Background (app icon) | Brand Surface Base | `oklch(0.13 0.015 265)` |

### Recording State Variant

When recording is active, the logo transforms:
- Dot color shifts: `--brand-accent` → `--brand-danger` (green → red)
- Dot pulses: `scale(1) → scale(1.2)` at 1.5s, ease-in-out
- Brackets subtly glow red at inner edges (2px feathered edge)

---

## 3. Logo Variations

### A. Primary (Full Color on Dark)

- White brackets + green dot on dark indigo background
- Use for: App icon, splash screen, hero sections, dark UI contexts
- Minimum size: 32px (below this, use simplified mark)

### B. Monochrome (Single Color)

- All elements in single color (white OR accent green)
- Use for: System tray (16px), watermarks, single-color print
- The dot remains filled; brackets remain stroked

### C. Wordmark (Logo + Text)

```
[Icon]  EasySpecy
```

- Icon at 32px height, text baseline-aligned to icon center
- "EasySpecy" in Inter 600, tracking -0.01em
- 12px gap between icon and text
- Use for: GitHub README header, website nav, about screens

### D. Favicon (Simplified)

At 16px and 32px, the full bracket detail is lost. The favicon simplifies:
- **16px**: Single bracket corner (top-left ⌜) + dot. That's it.
- **32px**: All four brackets + dot, reduced stroke width (proportional)
- **48px+**: Full detail logo

### E. Adaptive Icon (Android)

- Foreground: Brackets + dot (centered in safe zone)
- Background: Solid `--brand-surface-base`
- Follows Android adaptive icon grid (66dp safe zone within 108dp)

---

## 4. Logo Don'ts

| Don't | Why |
|-------|-----|
| Don't rotate the logo | Brackets lose their "capture frame" meaning at angles |
| Don't stretch or squish | Proportions are mathematically derived |
| Don't add drop shadows | The glow on the dot IS the depth. Extra shadows are noise |
| Don't place on busy backgrounds | Logo needs breathing room. Min 25% padding of logo width |
| Don't change bracket proportions | Arm length and stroke width are ratio-locked |
| Don't use gradient on brackets | Brackets are structural. Solid color only |
| Don't add text inside the brackets | The dot occupies that space. Nothing else |
| Don't outline the dot | The dot is always filled. Outlined dot reads as "target" not "recording" |
| Don't use the logo smaller than 16px | Below 16px, use the dot alone as a micro-mark |

---

## 5. Logo in Motion

### App Launch Animation (300ms total)

```
t=0ms:    Dot appears (scale 0 → 1, spring easing)
t=100ms:  Top-left bracket slides in from corner
t=133ms:  Top-right bracket slides in (stagger: 33ms)
t=166ms:  Bottom-right bracket slides in
t=200ms:  Bottom-left bracket slides in
t=250ms:  Dot pulses once (scale 1 → 1.15 → 1)
t=300ms:  Settled. Ready.
```

**Easing:** `cubic-bezier(0.16, 1, 0.3, 1)` (expo-out) for bracket entrances.
**Spring:** `stiffness: 400, damping: 22` for dot appearance.

### Recording Start Transition (200ms)

```
t=0ms:    Dot color: green → red (crossfade, 150ms)
t=0ms:    Brackets contract inward 4px (squeeze effect)
t=100ms:  Brackets release back to original position
t=200ms:  Dot begins pulse loop
```

### Recording Stop Transition (250ms)

```
t=0ms:    Dot pulse stops (holds at scale 1)
t=0ms:    Dot color: red → green (crossfade, 200ms)
t=100ms:  Brackets expand outward 4px (release effect)
t=150ms:  Brackets return to rest
t=250ms:  Settled. Idle.
```

### Hover State (Logo in UI)

- Dot brightens: lightness +0.05
- Brackets shift: translateY(-1px)
- Duration: 150ms, ease-out

---

## 6. Size Specifications & Export Matrix

### Favicon Package

| File | Size | Format | Use |
|------|------|--------|-----|
| `favicon.ico` | 16+32+48 | ICO (multi-res) | Browser tab |
| `favicon-16.png` | 16×16 | PNG | Smallest browser context |
| `favicon-32.png` | 32×32 | PNG | Standard browser tab |
| `favicon-32.webp` | 32×32 | WebP | Modern browsers |
| `favicon-48.png` | 48×48 | PNG | Windows pinned sites |
| `favicon-96.png` | 96×96 | PNG | Google TV, high-DPI tabs |
| `favicon-96.webp` | 96×96 | WebP | Modern browsers |
| `favicon-128.png` | 128×128 | PNG | Chrome Web Store |
| `favicon-192.png` | 192×192 | PNG | Android homescreen |
| `apple-touch-icon.png` | 180×180 | PNG | iOS homescreen |

### App Icons (Tauri Bundle)

| File | Size | Platform |
|------|------|----------|
| `32x32.png` | 32×32 | Windows small |
| `64x64.png` | 64×64 | Windows medium |
| `128x128.png` | 128×128 | macOS / Windows |
| `128x128@2x.png` | 256×256 | macOS Retina |
| `icon.ico` | Multi-res | Windows installer |
| `icon.icns` | Multi-res | macOS bundle |

### Android Adaptive Icons

| Directory | Size | DPI |
|-----------|------|-----|
| `mipmap-mdpi` | 48×48 | 160 |
| `mipmap-hdpi` | 72×72 | 240 |
| `mipmap-xhdpi` | 96×96 | 320 |
| `mipmap-xxhdpi` | 144×144 | 480 |
| `mipmap-xxxhdpi` | 192×192 | 640 |

### Social / Marketing

| File | Size | Use |
|------|------|-----|
| `og-image.png` | 1200×630 | Open Graph (link previews) |
| `twitter-card.png` | 1200×600 | Twitter/X card |
| `github-preview.png` | 1280×640 | GitHub social preview |

---

## 7. Logo Generation Spec (For AI/Design Tools)

### Prompt Engineering (For Regeneration)

If regenerating the logo with AI image tools, use this structured prompt:

```
Subject: App icon for a screen recording application called "EasySpecy"

Composition:
- Four bracket corners (⌜ ⌝ ⌞ ⌟) forming a capture frame
- Single glowing circle at dead center
- Dark indigo-black background

Style:
- Clean, geometric, minimal
- Brackets: white/silver metallic, rounded ends, consistent stroke width
- Center dot: emerald green (#00e88a) with soft radial glow
- Background: deep space indigo (#0d0f1a), no grid lines, no texture
- No text, no additional decorative elements

Lighting:
- Subtle ambient light on bracket surfaces (top-lit, soft)
- Green dot emits light (illuminates nearby bracket inner edges faintly)
- No harsh shadows, no 3D perspective

Quality:
- Vector-clean edges (even if raster output)
- Symmetrical on both axes
- Professional app icon quality (Apple App Store / Google Play standard)

Avoid:
- Grid overlays or construction lines in final output
- Lens flares or excessive glow
- 3D perspective or isometric view
- Gradients on the brackets themselves
- Any text or wordmark
- Camera lens metaphors (we're capture brackets, not a camera)
```

### Design Tool Specifications

For manual recreation in Figma/Illustrator:

| Parameter | Value |
|-----------|-------|
| Canvas | 1024 × 1024px |
| Background | `#0d0f1a` solid fill |
| Bracket stroke | 64px, round cap, round join |
| Bracket color | `#eaecf5` (white-silver) |
| Bracket corner radius | 48px outer |
| Bracket arm length | 180px |
| Bracket inset from edge | 200px |
| Dot diameter | 120px |
| Dot fill | `#00e88a` solid |
| Dot glow | Radial gradient, `#00e88a` → transparent, 200px radius, 40% opacity |
| Dot position | Exact center (512, 512) |

---

## 8. Logo Critique: Current vs. Ideal

### Current Logo Assessment

The existing logo (viewable at `brand/logo.png`) scores:

| Criterion | Score | Notes |
|-----------|-------|-------|
| Recognizability | 7/10 | Capture brackets are clear, but grid overlay adds noise |
| Scalability | 6/10 | Grid lines disappear at small sizes, creating inconsistency |
| Distinctiveness | 5/10 | Grid + brackets is generic "tech tool" territory |
| Simplicity | 5/10 | Too many elements: grid, circles, brackets, dot, border |
| Brand alignment | 7/10 | Colors are right, concept is right, execution is cluttered |
| **Overall** | **6/10** | Good concept, needs refinement |

### What to Fix in Next Iteration

1. **Remove the grid overlay** — It adds visual noise without communicating anything. The brackets alone say "capture."
2. **Remove concentric circles** — They suggest "radar" or "target," not "recording." The dot alone is the recording indicator.
3. **Remove the rounded-rectangle border** — The icon shape is defined by the OS (squircle on iOS, circle on Android, square on Windows). Don't draw your own container.
4. **Simplify to three elements only**: brackets + dot + background. Nothing else.
5. **Increase dot glow subtlety** — Current glow is good but could be slightly more diffused for premium feel.
6. **Bracket style** — Current brackets have a nice metallic quality. Keep that. But make them slightly thinner for elegance.

### Target Score After Redesign

| Criterion | Target |
|-----------|--------|
| Recognizability | 9/10 |
| Scalability | 9/10 |
| Distinctiveness | 8/10 |
| Simplicity | 9/10 |
| Brand alignment | 9/10 |
| **Overall** | **8.8/10** |

---

## 9. Trademark & Usage Rights

- **License**: The EasySpecy logo is part of the MIT-licensed project.
- **Attribution**: Not required for use within the application or forks.
- **Modification**: Forks should modify the logo to avoid confusion with the official project.
- **Commercial use**: Permitted under MIT. The logo may be used in articles, reviews, and comparisons without permission.

---

*Logo system version: 2.0*
*Last updated: 2026-05-31*
*Design tool: Figma / AI generation + manual refinement*
