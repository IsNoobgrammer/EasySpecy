# Hotkeys

EasySpecy uses system-wide hotkeys for recording control.

## Default Hotkeys

| Action | Hotkey |
|--------|--------|
| **Start Recording** | `Ctrl+Shift+R` |
| **Stop Recording** | `Ctrl+Shift+S` |
| **Pause/Resume** | `Ctrl+Shift+P` |
| **Region Select** | `Ctrl+Shift+E` |

## Customizing Hotkeys

### Via Settings UI

1. Open **Settings**
2. Navigate to **Hotkeys** section
3. Click on a hotkey field
4. Press your desired key combination
5. Click **Save**

### Via Config File

Edit `config.toml`:

```toml
[hotkeys]
start = "Ctrl+Shift+R"
stop = "Ctrl+Shift+S"
pause = "Ctrl+Shift+P"
region = "Ctrl+Shift+E"
```

**Supported modifiers:** `Ctrl`, `Shift`, `Alt`, `Super` (Cmd on macOS)

## Hotkey Conflicts

If a hotkey conflicts with another application:

1. EasySpecy will attempt to register it anyway
2. If registration fails, you'll see an error toast
3. Choose a different key combination

## Background Operation

Hotkeys work even when EasySpecy is:
- Minimized to system tray
- Running in background
- Another app is in fullscreen

This is because EasySpecy registers **system-global** shortcuts.

---

::: warning
Some games or full-screen applications may block global hotkeys. In that case, use the system tray controls.
:::
