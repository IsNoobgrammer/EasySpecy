# EasySpecy Brand Identity Guidelines

## Identity Concept
EasySpecy is a premium, free, cross-platform screen recorder with auto-zoom and cursor effects. The logo represents screen capture, focal zoom, and mouse movement paths.
- **Focal Camera Lens**: Representing frame capture and screen recording.
- **Glowing Red Recording Indicator**: Conveys active state and urgency.
- **Emerald-Mint and Space Indigo Glows**: Matches the application's premium slate dark mode.

## Color Tokens

### Dark Mode Theme
- **Base Background**: `oklch(0.11 0.012 260)` (Slate Indigo-Black)
- **Surface Card**: `oklch(0.15 0.016 260)` (Deep Indigo Surface)
- **Primary Accent**: `oklch(0.74 0.14 165)` (Emerald-Mint)
- **Active Record**: `oklch(0.64 0.20 25)` (Coral Red)

### Typography Pairing
- **Header & Labels**: Inter (Clean Geometric Sans)
- **Numerical & Data Readings**: JetBrains Mono (Technical Monospace)

## Usage Guidelines
1. **Web Favicons**: Root assets are served directly from the `/public` folder.
2. **App Icons**: Tauri automatically builds platform installers using the multi-scale formats located in `src-tauri/icons/`.
3. **Logo Inclusion**: The header in `Dashboard.tsx` loads the primary logo icon, maintaining brand cohesion.
