# Installation

EasySpecy is available for **Windows**, **macOS**, and **Linux**. Choose your platform below.

## Quick Install

### Windows

**Option 1: MSI Installer (Recommended)**

1. Download the latest `.msi` from [Releases](https://github.com/IsNoobgrammer/EasySpecy/releases)
2. Double-click to install
3. Launch from Start Menu or Desktop shortcut

**Option 2: Portable EXE**

1. Download the `.exe` from Releases
2. Run directly — no installation required
3. Logs and config stored next to the executable

### macOS

1. Download the `.dmg` from [Releases](https://github.com/IsNoobgrammer/EasySpecy/releases)
2. Open DMG and drag EasySpecy to Applications
3. **First launch:** Right-click → Open (macOS Gatekeeper workaround for unsigned app)
4. Grant screen recording permissions when prompted

### Linux

**Ubuntu/Debian:**

```bash
# Download .deb package
wget https://github.com/IsNoobgrammer/EasySpecy/releases/latest/download/easyspecy_amd64.deb

# Install
sudo dpkg -i easyspecy_amd64.deb
```

**Other Distributions:**

```bash
# Download AppImage
wget https://github.com/IsNoobgrammer/EasySpecy/releases/latest/download/easyspecy.AppImage

# Make executable
chmod +x easyspecy.AppImage

# Run
./easyspecy.AppImage
```

---

## Build from Source

If you want the latest development version or need to contribute, build from source.

### Prerequisites

#### All Platforms

- **Rust** (1.70+) — Install via [rustup](https://rustup.rs)
- **Node.js** (18+) — Install via [nvm](https://github.com/nvm-sh/nvm) or package manager
- **Git** — For cloning the repository

#### Windows

- **MSVC Build Tools** — Install via [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  - Select "C++ build tools" workload
  - Ensure "Windows 10 SDK" is checked

#### macOS

- **Xcode Command Line Tools**

```bash
xcode-select --install
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libayatana-appindicator3-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libasound2-dev
```

#### Linux (Fedora/RHEL)

```bash
sudo dnf install -y \
  gcc \
  gcc-c++ \
  make \
  pkg-config \
  openssl-devel \
  libappindicator-gtk3-devel \
  gtk3-devel \
  webkit2gtk3-devel \
  alsa-lib-devel
```

### Build Steps

```bash
# 1. Clone the repository
git clone https://github.com/IsNoobgrammer/EasySpecy.git
cd EasySpecy

# 2. Install frontend dependencies
npm install

# 3. Development mode (hot reload)
npm run tauri dev

# 4. Production build
npm run tauri build
```

### Build Output

After `npm run tauri build`, you'll find installers in `src-tauri/target/release/bundle/`:

```
src-tauri/target/release/bundle/
├── msi/
│   └── EasySpecy_0.1.0_x64_en-US.msi     # Windows installer
├── nsis/
│   └── EasySpecy_0.1.0_x64-setup.exe     # Windows NSIS installer
├── deb/
│   └── easyspecy_0.1.0_amd64.deb         # Debian package
├── appimage/
│   └── easyspecy_0.1.0_amd64.AppImage    # Linux AppImage
└── dmg/
    └── EasySpecy_0.1.0_aarch64.dmg       # macOS DMG (Apple Silicon)
```

---

## FFmpeg Bundling

EasySpecy requires FFmpeg for encoding. The binary is **~80-130MB** and is handled differently in development vs production.

### Development

FFmpeg is **not** bundled in Git. Download it manually:

```bash
# Windows
# Download ffmpeg.exe from https://github.com/BtbN/FFmpeg-Builds/releases
# Place in src-tauri/resources/ffmpeg.exe

# macOS/Linux
# Download ffmpeg binary from https://ffmpeg.org/download.html
# Place in src-tauri/resources/ffmpeg (no extension)
```

### Production (Tauri Sidecar)

For releases, FFmpeg is bundled as a **Tauri sidecar** using `externalBin` in `tauri.conf.json`:

```json
{
  "bundle": {
    "externalBin": ["resources/ffmpeg"]
  }
}
```

This bundles FFmpeg in the installer but excludes it from Git.

### CI/CD

GitHub Actions downloads FFmpeg during build:

```yaml
- name: Download FFmpeg
  run: |
    curl -L https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip -o ffmpeg.zip
    unzip ffmpeg.zip
    cp ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe src-tauri/resources/
```

---

## Platform-Specific Notes

### Windows

**Screen Capture:** Uses Windows Graphics Capture API (Windows 10+).

**Permissions:** No special permissions required for basic capture. Administrator rights only needed if you enable keyboard hook for game capture.

**Antivirus:** May flag the installer as unknown publisher. This is expected for open-source tools without code signing certificates. You can safely ignore the warning or verify the SHA256 hash from Releases.

### macOS

**Screen Capture:** Uses ScreenCaptureKit (macOS 12.3+).

**Permissions Required:**

1. **Screen Recording** — System Preferences → Security & Privacy → Privacy → Screen Recording
2. **Microphone** — Prompted on first use
3. **Accessibility** — Only if using keyboard shortcuts in background

**Gatekeeper:** Unsigned apps require right-click → Open on first launch. This is a macOS security feature, not a bug.

### Linux

**Screen Capture:** Uses PipeWire (modern) or X11 (fallback).

**Dependencies:**

- PipeWire 0.3+ (recommended)
- GTK3 (for UI)
- WebKit2GTK (for Tauri WebView)
- ALSA or PulseAudio (for audio)

**Wayland:** PipeWire works on Wayland. If you're on X11, ensure `xdg-desktop-portal` is installed.

---

## Verify Installation

After installation, verify EasySpecy is working:

1. **Launch the app**
2. **Check the header bar** — Should show "System Ready" with a green dot
3. **Open Settings** — Verify your screen and audio devices are detected
4. **Test recording** — Record 5 seconds and play back the file

### Troubleshooting

| Issue | Platform | Solution |
|-------|----------|----------|
| "No screens detected" | All | Ensure display is connected and active |
| "No audio devices" | Windows | Check WASAPI is enabled in Sound settings |
| "Permission denied" | macOS | Grant Screen Recording in System Preferences |
| "FFmpeg not found" | All | Ensure `ffmpeg` binary is in `resources/` folder |
| App won't launch | Linux | Install missing dependencies (see above) |

---

## Next Steps

- [Quick Start](/guide/quick-start) — Record your first video in 5 minutes
- [Configuration](/config/overview) — Customize recording settings
- [Hotkeys](/guide/hotkeys) — Learn keyboard shortcuts

---

**Need help?** [Open an issue](https://github.com/IsNoobgrammer/EasySpecy/issues) on GitHub.
