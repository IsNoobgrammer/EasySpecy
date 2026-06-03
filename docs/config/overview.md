# Configuration Overview

EasySpecy stores all settings in a TOML configuration file.

## Config File Location

| Platform | Path |
|----------|------|
| **Windows** | `%APPDATA%\easyspecy\config.toml` |
| **macOS** | `~/Library/Application Support/easyspecy/config.toml` |
| **Linux** | `~/.config/easyspecy/config.toml` |

## Config Format

```toml
# Video settings
[video]
resolution = "1080p"
fps = 30
encoder = "software"

# Audio settings
[audio]
enabled = true
microphone = "default"
system_audio = true
sample_rate = 48000

# Effects
[effects]
cursor_trail = true
auto_zoom = false
webcam = false

# Hotkeys
[hotkeys]
start = "Ctrl+Shift+R"
stop = "Ctrl+Shift+S"
pause = "Ctrl+Shift+P"
```

## Reloading Config

Config is loaded on app startup. To apply changes:

1. Save the config file
2. Restart EasySpecy

Or use the Settings UI for live updates.

---

::: tip
Always back up your config before editing manually!
:::
