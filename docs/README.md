# EasySpecy Documentation — Complete Structure

This directory contains the full documentation site for EasySpecy, built with **VitePress** and deployed to **GitHub Pages**.

## 📁 Directory Structure

```
docs/
├── index.md                          # Landing page (hero + features)
├── .vitepress/
│   ├── config.ts                     # VitePress configuration
│   ├── theme/
│   │   ├── index.ts                  # Custom theme entry
│   │   └── custom.css                # EasySpecy brand system (colors, typography, animations)
│   └── public/
│       └── logo.png                  # EasySpecy logo (copy from brand/)
├── guide/
│   ├── introduction.md               # What is EasySpecy, why it exists, comparison table
│   ├── installation.md               # Install guides for Win/Mac/Linux + build from source
│   ├── quick-start.md                # Record first video in 5 minutes
│   ├── screen-capture.md             # Full screen, region, window capture modes
│   ├── audio-recording.md            # Mic + system audio, noise reduction, devices
│   ├── auto-zoom.md                  # Cinematic zoom on clicks (in development)
│   ├── cursor-effects.md             # Trails, smoothing, custom packs
│   ├── webcam-overlay.md             # PiP webcam recording
│   ├── hotkeys.md                    # Default + custom keyboard shortcuts
│   ├── system-tray.md                # Tray controls, background recording
│   └── region-selection.md           # Interactive region picker
├── api/
│   ├── commands.md                   # All Tauri IPC commands reference
│   ├── capture.md                    # Capture module API
│   ├── audio.md                      # Audio module API
│   └── postprocess.md                # Post-processing pipeline API
├── config/
│   ├── overview.md                   # Config file location, format, reloading
│   ├── video.md                      # Resolution, FPS, encoder, quality settings
│   ├── audio.md                      # Sample rate, devices, noise reduction
│   └── effects.md                    # Cursor trail, auto-zoom, webcam config
└── advanced/
    ├── architecture.md               # System design, module breakdown, IPC
    ├── av-sync.md                    # A/V synchronization architecture
    ├── performance.md                # Optimization, parallelism, profiling
    ├── building.md                   # Build process, CI/CD, sidecar bundling
    └── contributing.md               # How to contribute, coding standards
```

## 🎨 Brand System Applied

The documentation site uses EasySpecy's complete brand system:

### Colors (OKLCH-based)

**Dark Mode (Primary):**
- Surface Base: `#0d0f1a` (OKLCH 0.13 0.015 265)
- Surface Raised: `#151828` (OKLCH 0.17 0.018 265)
- Surface Overlay: `#1d2035` (OKLCH 0.21 0.020 265)
- Accent (Signal Green): `#00e88a` (OKLCH 0.78 0.18 160)
- Text Primary: `#eef0f6` (OKLCH 0.95 0.005 265)
- Text Secondary: `#9a9eb5` (OKLCH 0.70 0.010 265)

**Light Mode (Accessibility):**
- Surface: `#f4fbf2`
- Accent: `#006e3e`
- Text: `#2d342e`

### Typography

- **Headings/Body:** Inter (variable font, weights 300-800)
- **Code:** JetBrains Mono (weights 400-600)
- **Type Scale:** 1.25 ratio (Major Third)
- **Line Length:** ≤75ch for readability

### Design Style

**Dark Mode + Aurora UI** — Modern SaaS aesthetic with:
- Subtle aurora background blobs (green + indigo)
- Frosted glass navigation bar
- Hover animations on feature cards
- Smooth transitions (150-250ms, expo-out easing)
- WCAG AAA contrast compliance

## 🚀 Deployment

### GitHub Pages Workflow

The site deploys automatically via GitHub Actions:

```yaml
# .github/workflows/deploy-docs.yml
name: Deploy Documentation

on:
  push:
    branches: [main]
    paths: ['docs/**']
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
      - run: npm ci
      - run: npm run docs:build
      - uses: actions/configure-pages@v4
      - uses: actions/upload-pages-artifact@v3
        with:
          path: docs/.vitepress/dist

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/deploy-pages@v4
```

### Deploy URL

After deployment, docs are live at:

```
https://<username>.github.io/EasySpecy/
```

For custom domains, configure in repository Settings → Pages.

## 📝 Documentation Philosophy

### Writing Rules

1. **Start with Why** — Explain purpose before implementation
2. **Show, Don't Tell** — Every concept needs a runnable example
3. **Progressive Disclosure** — Getting Started → Guides → API Reference → Advanced
4. **Consistent Patterns** — Same structure for every function/module doc
5. **Write for Scanning** — Bold key terms, short paragraphs, bullet lists
6. **Include Failure Cases** — Show what breaks and why
7. **Realistic Examples** — No `foo`/`bar` — use real domain names and data
8. **Version Tagging** — Mark when features were added: `**Added in:** vX.Y`

### Content Hierarchy

```
Getting Started (zero to first success in < 5 min)
  ↓
Guides (common use cases, patterns, workflows)
  ↓
API Reference (complete technical spec)
  ↓
Advanced (optimization, edge cases, internals)
```

## 🛠️ Development

### Local Development

```bash
# Install dependencies
npm install

# Start dev server (hot reload)
npm run docs:dev

# Build for production
npm run docs:build

# Preview production build
npm run docs:preview
```

### Adding New Pages

1. Create `.md` file in appropriate directory (`guide/`, `api/`, etc.)
2. Add to sidebar in `.vitepress/config.ts`
3. Use frontmatter for page-specific metadata:

```yaml
---
title: Page Title
description: Brief description for SEO
---
```

### Custom Components

VitePress supports Vue components in Markdown:

```markdown
<script setup>
import MyComponent from './MyComponent.vue'
</script>

<MyComponent />
```

## 📊 Quality Checklist

Before merging documentation changes:

### Completeness
- [ ] Every public feature documented
- [ ] Every configuration option explained
- [ ] Every error condition documented
- [ ] Examples for common use cases
- [ ] Troubleshooting for common errors

### Accuracy
- [ ] All examples tested and runnable
- [ ] Code matches current version
- [ ] No broken links
- [ ] Version numbers correct

### Usability
- [ ] Getting Started works standalone
- [ ] Examples are copy-pasteable
- [ ] Navigation is intuitive
- [ ] Search works
- [ ] Mobile-friendly

### Style
- [ ] Consistent structure across sections
- [ ] Clear headings
- [ ] Short paragraphs (2-3 sentences)
- [ ] Code formatted consistently
- [ ] No jargon without explanation

## 🎯 Future Improvements

- [ ] Video tutorials embedded in guides
- [ ] Interactive API playground
- [ ] Multi-language support (i18n)
- [ ] Versioned documentation (v0.1, v0.2, etc.)
- [ ] Algolia DocSearch integration
- [ ] Community-contributed examples section
- [ ] Changelog page

---

**Built with:** VitePress, Inter, JetBrains Mono, EasySpecy Brand System
**Deployed to:** GitHub Pages
**License:** MIT
