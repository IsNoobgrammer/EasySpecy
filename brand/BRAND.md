# EasySpecy — Brand Identity System

> *Record like a pro. Pay like it's 2005.*

---

## 1. Brand Essence

### The Core Idea

EasySpecy exists because screen recording shouldn't cost $89/year. It shouldn't be locked to one OS. And it shouldn't require a PhD in OBS settings.

**Brand Promise:** Cinematic screen recordings. Zero cost. Zero compromise.

### Personality Archetype: The Skilled Rebel

EasySpecy is the open-source underdog that punches above its weight. It's not trying to be corporate or "enterprise-ready." It's built by developers, for developers and creators who value craft over subscription fees.

| Trait | Expression |
|-------|-----------|
| **Confident** | We don't say "almost as good as Screen Studio." We say "better, and free." |
| **Technical** | We respect our users' intelligence. No dumbing down. |
| **Generous** | MIT licensed. No freemium traps. No "upgrade to unlock." |
| **Fast** | Rust-native performance. Sub-50ms capture latency. The brand feels fast too. |
| **Precise** | Auto-zoom, cursor effects, pixel-perfect region select. Precision is our craft. |

### Voice & Tone

- **Direct.** No marketing fluff. Say what it does.
- **Confident, not arrogant.** We built something great. We don't need to trash competitors.
- **Technical when needed.** "Rust + FFmpeg pipeline" is a feature, not jargon to hide.
- **Casual but competent.** Like a senior dev explaining their side project at a meetup.

**Examples:**
- ✅ "Record your screen. Auto-zoom on clicks. Save to file. Done."
- ✅ "Screen Studio quality. OBS flexibility. $0 price tag."
- ❌ "The world's most innovative AI-powered recording solution."
- ❌ "Unleash your creative potential with our cutting-edge platform."

---

## 2. Color System

### Philosophy

The palette is built around two ideas:
1. **Dark-first** — screen recorders live in dark mode. The UI should feel like a professional editing suite.
2. **Signal green** — the universal "recording" indicator. Our accent isn't decorative — it means "active, alive, capturing."

### Primary Palette (OKLCH)

| Token | OKLCH | Hex | Role |
|-------|-------|-----|------|
| `--brand-surface-base` | `oklch(0.13 0.015 265)` | `#0d0f1a` | App background, deepest layer |
| `--brand-surface-raised` | `oklch(0.17 0.018 265)` | `#151828` | Cards, panels, elevated surfaces |
| `--brand-surface-overlay` | `oklch(0.21 0.020 265)` | `#1d2035` | Modals, dropdowns, tooltips |
| `--brand-accent` | `oklch(0.78 0.18 160)` | `#00e88a` | Primary action, recording indicator, links |
| `--brand-accent-hover` | `oklch(0.83 0.16 160)` | `#33f0a3` | Hover state for accent |
| `--brand-accent-muted` | `oklch(0.78 0.18 160 / 0.15)` | — | Accent backgrounds, subtle highlights |
| `--brand-danger` | `oklch(0.65 0.22 25)` | `#f04040` | Stop recording, destructive actions |
| `--brand-warning` | `oklch(0.80 0.16 85)` | `#e8b830` | Paused state, caution |
| `--brand-text-primary` | `oklch(0.95 0.005 265)` | `#eef0f6` | Headings, primary content |
| `--brand-text-secondary` | `oklch(0.70 0.010 265)` | `#9a9eb5` | Labels, descriptions, metadata |
| `--brand-text-muted` | `oklch(0.50 0.010 265)` | `#5c6078` | Disabled text, placeholders |
| `--brand-border` | `oklch(0.25 0.015 265)` | `#2a2d42` | Subtle dividers, card borders |

### Semantic Color Mapping

| State | Color | Usage |
|-------|-------|-------|
| **Idle** | `--brand-accent` | Ready to record, primary CTA |
| **Recording** | `--brand-danger` with pulse | Active capture indicator |
| **Paused** | `--brand-warning` | Paused state indicator |
| **Success** | `--brand-accent` | File saved, operation complete |
| **Error** | `--brand-danger` | Failed capture, missing permissions |

### Why Not Pure Black?

Pure `#000` is dead. Our darkest surface (`0d0f1a`) has a subtle indigo-violet tint — it feels like looking into deep space rather than staring at a void. This tint:
- Reduces eye strain in prolonged dark-mode use
- Creates perceptible depth between surface layers
- Aligns with the "precision instrument" feel (think: pro audio software, video editors)

### Light Mode (Secondary)

EasySpecy is dark-first, but a light mode exists for accessibility:

| Token | OKLCH | Hex |
|-------|-------|-----|
| `--brand-surface-base` | `oklch(0.98 0.005 265)` | `#f8f9fc` |
| `--brand-surface-raised` | `oklch(1.0 0 0)` | `#ffffff` |
| `--brand-accent` | `oklch(0.55 0.20 160)` | `#009960` |
| `--brand-text-primary` | `oklch(0.15 0.015 265)` | `#141729` |

---

## 3. Typography

### Type Stack

| Role | Font | Weight | Fallback |
|------|------|--------|----------|
| **Display / Hero** | Inter | 700–800 | system-ui, sans-serif |
| **Headings** | Inter | 600 | system-ui, sans-serif |
| **Body** | Inter | 400 | system-ui, sans-serif |
| **Code / Data** | JetBrains Mono | 400–500 | ui-monospace, monospace |
| **UI Labels** | Inter | 500 | system-ui, sans-serif |

### Type Scale (1.25 ratio — Major Third)

| Level | Size | Line Height | Letter Spacing | Use |
|-------|------|-------------|----------------|-----|
| Display | 48px / 3rem | 1.1 | -0.02em | Hero headlines, marketing |
| H1 | 32px / 2rem | 1.2 | -0.015em | Page titles |
| H2 | 24px / 1.5rem | 1.3 | -0.01em | Section headers |
| H3 | 20px / 1.25rem | 1.4 | 0 | Subsections |
| Body | 16px / 1rem | 1.6 | 0 | Paragraphs, descriptions |
| Small | 14px / 0.875rem | 1.5 | 0.005em | Labels, captions |
| Micro | 12px / 0.75rem | 1.4 | 0.01em | Badges, timestamps |

### Typography Rules

- **Headings**: `text-wrap: balance` — no orphans.
- **Body**: max-width `65ch` for readability.
- **Monospace**: Used for file paths, hotkey labels, resolution values, FPS counters.
- **Dark mode adjustment**: Body weight 350 (optical compensation for light-on-dark).
- **No decorative fonts.** EasySpecy is a tool, not a poster.

---

## 4. Spacing & Layout

### 4pt Grid System

All spacing derives from a 4px base unit:

| Token | Value | Use |
|-------|-------|-----|
| `--space-1` | 4px | Inline icon gaps, tight padding |
| `--space-2` | 8px | Button padding, input padding |
| `--space-3` | 12px | Card internal padding |
| `--space-4` | 16px | Section gaps, standard margin |
| `--space-6` | 24px | Card-to-card gaps |
| `--space-8` | 32px | Section separators |
| `--space-12` | 48px | Major section breaks |
| `--space-16` | 64px | Page-level vertical rhythm |

### Border Radius

| Token | Value | Use |
|-------|-------|-----|
| `--radius-sm` | 6px | Buttons, inputs, badges |
| `--radius-md` | 10px | Cards, panels |
| `--radius-lg` | 16px | Modals, large containers |
| `--radius-full` | 9999px | Pills, avatars, recording dot |

---

## 5. Iconography & Visual Language

### Icon Style

- **Stroke-based**, 1.5px weight at 24px size
- **Rounded caps and joins** — matches border-radius language
- **Consistent optical size** — icons sit within a 20px live area inside 24px bounding box
- **Source**: Lucide icons (MIT, consistent, well-maintained)

### Visual Metaphors

| Concept | Visual |
|---------|--------|
| Recording | Pulsing dot (green → red transition) |
| Screen capture | Bracket corners (⌜ ⌝ ⌞ ⌟) |
| Precision/zoom | Crosshair, concentric circles |
| Speed/performance | Minimal chrome, instant feedback |
| Open source | No lock icons, no "pro" badges |

### The Recording Dot

The pulsing recording indicator is our most recognizable micro-brand element:
- 12px circle, `--brand-danger` when recording
- `--brand-accent` when idle/ready
- Pulse animation: `scale(1) → scale(1.3) → scale(1)` at 1.5s interval, ease-in-out
- Always visible in system tray during active recording

---

## 6. Motion & Animation Principles

### Timing

| Context | Duration | Easing |
|---------|----------|--------|
| Hover/focus | 100ms | `ease-out` |
| State toggle | 150ms | `cubic-bezier(0.16, 1, 0.3, 1)` |
| Panel open/close | 200ms | `cubic-bezier(0.16, 1, 0.3, 1)` |
| Recording start | 300ms | Spring (stiffness: 400, damping: 25) |
| Page transition | 250ms | `cubic-bezier(0.33, 1, 0.68, 1)` |

### Motion Philosophy

- **Functional, not decorative.** Every animation answers "what changed?"
- **Fast.** This is a performance tool. Animations should feel snappy, not luxurious.
- **Recording-aware.** Reduce/disable animations during active recording to minimize CPU overhead.
- **Respect `prefers-reduced-motion`.** Always. No exceptions.

---

## 7. Brand Positioning

### Competitive Landscape

```
                    Premium Quality
                         ↑
                         |
          Screen Studio  |  EasySpecy ← HERE
          ($89/yr, macOS)|  (Free, cross-platform)
                         |
    ─────────────────────┼─────────────────────→ Free
                         |
          Loom           |  OBS
          ($15/mo, cloud)|  (Free, complex)
                         |
                    Basic Quality
```

### Taglines (Use Contextually)

| Context | Tagline |
|---------|---------|
| **Primary** | Record like a pro. Pay like it's 2005. |
| **Technical** | Rust-native screen recording with cinematic auto-zoom. |
| **GitHub** | Free, open-source screen recorder with auto-zoom & cursor effects. |
| **Comparison** | Screen Studio quality. $0 price tag. All platforms. |
| **Minimal** | Record. Zoom. Done. |

### Key Messages (Priority Order)

1. **Free and open source** — MIT licensed, no tricks, no freemium
2. **Cross-platform** — Windows, macOS, Linux with feature parity
3. **Auto-zoom & effects** — Cinematic output without post-production
4. **Native performance** — Rust + Tauri, sub-50ms capture latency
5. **Privacy-first** — Fully offline, no telemetry, no accounts

---

## 8. Application Contexts

### System Tray

- 16×16 / 32×32 monochrome icon (white on transparent)
- Recording state: icon swaps to red-dot variant
- Tooltip: "EasySpecy — Ready" / "EasySpecy — Recording (02:34)"

### Installer / Splash

- Full-color logo on `--brand-surface-base` background
- App name in Inter 600, 24px below logo
- Version number in `--brand-text-muted`, 14px

### GitHub / Social

- OG image: Logo centered on dark gradient background
- README badge style: flat, using `--brand-accent` for active badges
- Social preview: Logo + tagline + "Free & Open Source" badge

### In-App Header

- Logo mark (icon only) at 28px, left-aligned
- App name optional — the icon is the brand in-context
- Recording indicator dot adjacent to logo during capture

---

## 9. What EasySpecy Is NOT

- **Not corporate.** No blue gradients, no stock photos, no "enterprise solutions."
- **Not playful/childish.** No rounded bubbly shapes, no emoji-heavy copy, no pastel palettes.
- **Not minimalist to the point of emptiness.** We have personality. We have opinions.
- **Not dark for the sake of dark.** The dark theme serves function (screen recording context) not aesthetic trend.
- **Not another OBS.** We're opinionated. Fewer settings, better defaults, polished output.

---

## 10. File Manifest

| Asset | Location | Format |
|-------|----------|--------|
| Primary logo (full color) | `brand/logo.png` | PNG, 1024×1024 |
| Logo (public copy) | `public/logo.png` | PNG, 512×512 |
| Favicon package | `public/favicon-*.png` | PNG, multiple sizes |
| Favicon ICO | `public/favicon.ico` | ICO, 16+32+48 |
| WebP variants | `public/*.webp` | WebP, optimized |
| Apple touch icon | `public/apple-touch-icon.png` | PNG, 180×180 |
| Android adaptive | `src-tauri/icons/android/` | PNG, mipmap sizes |
| Web manifest | `public/site.webmanifest` | JSON |
| OG / Social image | `public/og-image.png` | PNG, 1200×630 |
| GitHub preview | `public/github-preview.png` | PNG, 1280×640 |

---

## 11. Brand Evolution Roadmap

| Phase | Brand Expression |
|-------|-----------------|
| **Phase 1 (MVP)** | Logo + dark UI + accent color. Functional brand. |
| **Phase 2 (Growth)** | Landing page, GitHub README polish, social templates. |
| **Phase 3 (Community)** | Contributor badges, community assets, swag templates. |
| **Phase 4 (Maturity)** | Full design system, component library, brand guidelines site. |

---

*Last updated: 2026-05-31*
*Brand system version: 2.0*
