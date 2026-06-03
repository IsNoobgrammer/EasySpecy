import { useState, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { Rnd } from "react-rnd";
import { Icon } from "./Icon";

export interface KeyboardOverlayConfig {
  enabled: boolean;
  fontFamily: string;
  fontSize: number;
  opacity: number;
  x: number;
  y: number;
  width: number;
  cornerRadius: number;
  borderWidth: number;
  borderColor: string;
  backgroundColor: string;
  textColor: string;
  theme: string;
  keyMappings: string; // JSON string
  maxBubbles: number;
  bubbleTimeoutMs: number;
}

interface KeyboardPreviewProps {
  onSave: (config: KeyboardOverlayConfig) => void;
  onCancel: () => void;
  initial?: Partial<KeyboardOverlayConfig>;
  recordingWidth?: number;
  recordingHeight?: number;
}

interface RenderedBubble {
  id: string;
  type: "text" | "shortcut" | "special";
  text: string;
  timestamp: number;
}

const FONT_FAMILIES = [
  { label: "JetBrains Mono (Developer)", value: "JetBrains Mono" },
  { label: "Inter (Sleek Sans-Serif)", value: "Inter" },
  { label: "Times New Roman (Classic Serif)", value: "Times New Roman" },
  { label: "Comic Sans MS (Casual)", value: "Comic Sans MS" },
  { label: "Algerian (Decorative)", value: "Algerian" },
];

const PRESET_THEMES = [
  {
    id: "speccy-classic",
    name: "Speccy's Classic",
    background: "rgba(20, 20, 20, 0.75)",
    border: "rgba(140, 140, 140, 0.15)",
    text: "#e5e5e0",
  },
  {
    id: "light-glass",
    name: "Light Glassmorphism",
    background: "rgba(255, 255, 255, 0.65)",
    border: "rgba(0, 0, 0, 0.12)",
    text: "#1c1b1b",
  },
  {
    id: "neon-green",
    name: "Neon Cyberpunk (Green)",
    background: "rgba(9, 11, 20, 0.85)",
    border: "#00e88a",
    text: "#85ffb4",
  },
  {
    id: "neon-purple",
    name: "Neon Vaporwave (Purple)",
    background: "rgba(15, 11, 28, 0.85)",
    border: "#a855f7",
    text: "#c0c1ff",
  },
  {
    id: "custom",
    name: "Custom Theme Settings",
    background: "rgba(0, 0, 0, 0.65)",
    border: "rgba(255, 255, 255, 0.15)",
    text: "#ffffff",
  }
];

export function KeyboardPreview({
  onSave,
  onCancel,
  initial,
  recordingWidth = 1920,
  recordingHeight = 1080,
}: KeyboardPreviewProps) {
  const initialWidth = initial?.width ?? 360;
  const defaultX = Math.round((recordingWidth - initialWidth) / 2);
  const defaultY = Math.round(recordingHeight - 120);

  const initialX = Math.max(0, Math.min(recordingWidth - initialWidth, initial?.x ?? defaultX));
  const initialY = Math.max(0, Math.min(recordingHeight - 80, initial?.y ?? defaultY));

  // Config state
  const [overlay, setOverlay] = useState<KeyboardOverlayConfig>({
    enabled: initial?.enabled ?? true,
    fontFamily: initial?.fontFamily ?? "JetBrains Mono",
    fontSize: initial?.fontSize ?? 14,
    opacity: initial?.opacity ?? 0.9,
    x: initialX,
    y: initialY,
    width: initialWidth,
    cornerRadius: initial?.cornerRadius ?? 8,
    borderWidth: initial?.borderWidth ?? 1,
    borderColor: initial?.borderColor ?? "rgba(140, 140, 140, 0.15)",
    backgroundColor: initial?.backgroundColor ?? "rgba(20, 20, 20, 0.75)",
    textColor: initial?.textColor ?? "#e5e5e0",
    theme: initial?.theme ?? "speccy-classic",
    keyMappings: initial?.keyMappings ?? `{"Ctrl":"⌃","Shift":"⇧","Alt":"⌥","Win":"⊞","Enter":"↵","Backspace":"⌫","Space":"␣","Esc":"⎋"}`,
    maxBubbles: initial?.maxBubbles ?? 4,
    bubbleTimeoutMs: initial?.bubbleTimeoutMs ?? 3000,
  });

  // Test typing sandbox state
  const [sandboxText, setSandboxText] = useState("");
  const [bubbles, setBubbles] = useState<RenderedBubble[]>([]);
  const lastEventTimeRef = useRef<number>(0);
  const sandboxRef = useRef<HTMLInputElement>(null);

  // Focus the sandbox on mount
  useEffect(() => {
    sandboxRef.current?.focus();
  }, []);

  // Update a config key
  const update = <K extends keyof KeyboardOverlayConfig>(
    key: K,
    value: KeyboardOverlayConfig[K]
  ) => {
    setOverlay((prev) => {
      const next = { ...prev, [key]: value };
      
      // Clamp X position if width pushes it offscreen
      if (key === "width") {
        const w = value as number;
        if (next.x + w > recordingWidth) {
          next.x = Math.max(0, recordingWidth - w);
        }
      }

      // If theme preset is selected, automatically apply its styling properties
      if (key === "theme" && value !== "custom") {
        const themePreset = PRESET_THEMES.find((t) => t.id === value);
        if (themePreset) {
          next.backgroundColor = themePreset.background;
          next.borderColor = themePreset.border;
          next.textColor = themePreset.text;
        }
      } else if (key === "backgroundColor" || key === "borderColor" || key === "textColor") {
        // If user modifies color directly, switch theme selection to custom
        next.theme = "custom";
      }
      
      return next;
    });
  };

  // Parser helper functions
  const normalizeKey = (key: string): string => {
    if (key === "LCtrl" || key === "RCtrl") return "Ctrl";
    if (key === "LShift" || key === "RShift") return "Shift";
    if (key === "LAlt" || key === "RAlt") return "Alt";
    return key;
  };

  const getMapping = (key: string, mappingsJson: string): string => {
    const norm = normalizeKey(key);
    try {
      const map = JSON.parse(mappingsJson);
      return map[norm] || map[key] || norm;
    } catch {
      return norm;
    }
  };

  // Keep a loop running to handle timeout & clamping for bubbles
  useEffect(() => {
    const timer = setInterval(() => {
      const now = Date.now();
      
      setBubbles((prev) => {
        let nextBubbles = [...prev];
        let activeIdx = nextBubbles.findIndex(b => b.id === "active");
        let active = activeIdx > -1 ? nextBubbles[activeIdx] : null;

        // Idle split check: if active text bubble has been inactive for > 1.2s, seal it
        if (active && lastEventTimeRef.current > 0 && now - lastEventTimeRef.current > 1200) {
          active.id = `bubble-${active.timestamp}`; // seal it
          active = null;
        }

        // Filter out bubbles that have timed out (unless it's the active bubble)
        let filtered = nextBubbles.filter(b => {
          if (b.id === "active") return true;
          return (now - b.timestamp) < overlay.bubbleTimeoutMs;
        });

        // Clamp to max visible bubbles
        while (filtered.length > overlay.maxBubbles) {
          filtered.shift();
        }

        // Avoid state updates if nothing changed
        if (JSON.stringify(filtered) !== JSON.stringify(prev)) {
          return filtered;
        }
        return prev;
      });
    }, 100);

    return () => clearInterval(timer);
  }, [overlay.maxBubbles, overlay.bubbleTimeoutMs]);

  // Capture keystrokes from the input field and process incrementally
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    // Prevent default behaviour for tab, enter, escape to keep focus and prevent default form actions
    if (["Tab", "Enter", "Escape"].includes(e.key)) {
      e.preventDefault();
      if (e.key === "Enter") {
        setSandboxText("");
      }
    }

    const key = e.key;
    const t = Date.now();
    const ctrl = e.ctrlKey;
    const shift = e.shiftKey;
    const alt = e.altKey;
    const win = e.metaKey;

    // Map system names to VK hook names
    let keyName = key;
    if (key === " ") keyName = "Space";
    else if (key === "Control") keyName = "Ctrl";
    else if (key === "Shift") keyName = "Shift";
    else if (key === "Alt") keyName = "Alt";
    else if (key === "Meta" || key === "OS") keyName = "Win";
    else if (key === "ArrowLeft" || key === "Left") keyName = "←";
    else if (key === "ArrowUp" || key === "Up") keyName = "↑";
    else if (key === "ArrowRight" || key === "Right") keyName = "→";
    else if (key === "ArrowDown" || key === "Down") keyName = "↓";

    const SHIFT_MAP: Record<string, string> = {
      "1": "!", "2": "@", "3": "#", "4": "$", "5": "%", "6": "^", "7": "&", "8": "*", "9": "(", "0": ")",
      "-": "_", "=": "+", "[": "{", "]": "}", "\\": "|", ";": ":", "'": "\"", ",": "<", ".": ">", "/": "?",
      "`": "~"
    };

    const norm = normalizeKey(keyName);

    setBubbles((prev) => {
      let nextBubbles = [...prev];
      
      let activeIdx = nextBubbles.findIndex(b => b.id === "active");
      let active = activeIdx > -1 ? nextBubbles[activeIdx] : null;

      // Idle split check: if active exists but last event was > 1.2s ago, seal it
      if (active && lastEventTimeRef.current > 0 && t - lastEventTimeRef.current > 1200) {
        active.id = `bubble-${active.timestamp}`; // seal it
        active = null;
      }
      lastEventTimeRef.current = t;

      const appendCharacter = (char: string) => {
        const PUNCTUATION = [";", ":", ",", ".", "?", "!"];
        if (PUNCTUATION.includes(char)) {
          const last = nextBubbles[nextBubbles.length - 1];
          if (last && last.type === "text") {
            last.text += char;
            last.timestamp = t;
            last.id = "active";
            return;
          }
        }

        if (active) {
          active.text += char;
          active.timestamp = t;
        } else {
          active = {
            id: "active",
            type: "text",
            text: char,
            timestamp: t
          };
          nextBubbles.push(active);
        }
      };

      const getShiftedKey = (k: string) => {
        if (/^[a-zA-Z]$/.test(k)) {
          return k.toUpperCase();
        }
        return SHIFT_MAP[k] || null;
      };

      // Space delimiter split
      if (norm === "Space") {
        if (active) {
          active.id = `bubble-${active.timestamp}`; // seal
        }
        appendCharacter(" ");
        return nextBubbles;
      }

      // Arrow keys — MUST seal active text and be shown as their own box (as requested)
      const isArrow = ["←", "↑", "→", "↓", "ArrowLeft", "ArrowUp", "ArrowRight", "ArrowDown"].includes(norm);
      if (isArrow) {
        if (active) {
          active.id = `bubble-${active.timestamp}`; // seal
        }
        let text = getMapping(norm, overlay.keyMappings);
        if (ctrl || alt || win || shift) {
          const parts = [];
          if (ctrl) parts.push("Ctrl");
          if (alt) parts.push("Alt");
          if (win) parts.push("Win");
          if (shift) parts.push("Shift");
          parts.push(text);
          text = parts.join(" + ");
          nextBubbles.push({
            id: `shortcut-${t}`,
            type: "shortcut",
            text: text,
            timestamp: t
          });
        } else {
          nextBubbles.push({
            id: `special-${t}`,
            type: "special",
            text: text,
            timestamp: t
          });
        }
        return nextBubbles;
      }

      // Check if it's a shortcut combo:
      const isSpecialKey = ["Enter", "Tab", "Esc", "NumEnter", "Backspace", "Delete", "Insert", "PageUp", "PageDown", "Home", "End", "CapsLock", "ScrollLock", "NumLock", "Pause", "PrintScreen"].includes(norm) || (norm.startsWith("F") && norm.length > 1);
      
      const isShortcut = ctrl || alt || win || (shift && isSpecialKey);

      if (isShortcut) {
        const parts = [];
        if (ctrl) parts.push("Ctrl");
        if (alt) parts.push("Alt");
        if (win) parts.push("Win");
        
        if (shift && norm !== "Shift") {
          parts.push("Shift");
        }

        if (norm !== "Ctrl" && norm !== "Alt" && norm !== "Win" && norm !== "Shift") {
          parts.push(getMapping(keyName, overlay.keyMappings));
        }

        if (parts.length > 0) {
          if (active) {
            active.id = `bubble-${active.timestamp}`; // seal
          }
          nextBubbles.push({
            id: `shortcut-${t}`,
            type: "shortcut",
            text: parts.join(" + "),
            timestamp: t
          });
        }
        return nextBubbles;
      }

      // Backspace logic
      if (norm === "Backspace") {
        if (active && active.text.length > 0) {
          active.text = active.text.slice(0, -1);
          active.timestamp = t;
          if (active.text.length === 0) {
            nextBubbles = nextBubbles.filter(b => b.id !== "active");
          }
        }
        return nextBubbles;
      }

      // Delimiter keys
      const isDelim = ["Enter", "Tab", "Esc", "NumEnter"].includes(norm);
      if (isDelim) {
        if (active) {
          active.id = `bubble-${active.timestamp}`; // seal
        }
        nextBubbles.push({
          id: `special-${t}`,
          type: "special",
          text: getMapping(keyName, overlay.keyMappings),
          timestamp: t
        });
        return nextBubbles;
      }

      // Handle Shift + printable keys (e.g. Shift + 1 -> !, Shift + a -> A)
      if (shift && norm !== "Shift") {
        const shifted = getShiftedKey(keyName);
        if (shifted !== null) {
          appendCharacter(shifted);
          return nextBubbles;
        }
      }

      // Normal characters / symbols (excluding modifier keys themselves)
      if (norm !== "Ctrl" && norm !== "Alt" && norm !== "Win" && norm !== "Shift") {
        if (keyName.length === 1) {
          const char = /^[A-Z]$/.test(keyName) ? keyName.toLowerCase() : keyName;
          appendCharacter(char);
        } else {
          if (active) {
            active.id = `bubble-${active.timestamp}`; // seal
          }
          nextBubbles.push({
            id: `special-${t}`,
            type: "special",
            text: getMapping(keyName, overlay.keyMappings),
            timestamp: t
          });
        }
      }

      return nextBubbles;
    });
  };

  // If no user typing exists, show a nice mock preview
  const mockBubbles: RenderedBubble[] = [
    { id: "mock-1", type: "text", text: "EasySpecy", timestamp: 0 },
    { id: "mock-2", type: "special", text: "↵", timestamp: 0 },
    { id: "mock-3", type: "shortcut", text: "⌃ + C", timestamp: 0 },
  ];

  const displayBubbles = bubbles.length > 0 ? bubbles : mockBubbles;

  // Calculate layout preview scale
  const PREVIEW_MAX_W = 900;
  const PREVIEW_MAX_H = 540;
  const scaleX = PREVIEW_MAX_W / recordingWidth;
  const scaleY = PREVIEW_MAX_H / recordingHeight;
  const scale = Math.min(scaleX, scaleY, 0.95);
  const previewW = recordingWidth * scale;
  const previewH = recordingHeight * scale;

  // Theme presets select styles
  const isThemeSelected = (id: string) => overlay.theme === id;

  // Parse mappings to let user edit them
  const parsedMappings = (() => {
    try {
      return JSON.parse(overlay.keyMappings) as Record<string, string>;
    } catch {
      return {};
    }
  })();

  const handleUpdateMapping = (keyName: string, val: string) => {
    const updated = { ...parsedMappings, [keyName]: val };
    update("keyMappings", JSON.stringify(updated));
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[9999] flex"
      style={{ background: "#0d0f1a", color: "#e1e1f2" }}
    >
      {/* ═══ LEFT SIDEBAR — Controls ═══ */}
      <div
        className="w-80 flex-shrink-0 overflow-y-auto flex flex-col scrollbar-thin"
        style={{
          background: "#151828",
          borderRight: "1px solid #2d314d",
        }}
      >
        {/* Header */}
        <div className="px-5 py-4 flex items-center justify-between border-b border-[#2d314d]">
          <div>
            <div className="font-mono text-xs font-bold uppercase tracking-wider text-[#e1e1f2]">
              KEYBOARD OVERLAY CONFIG
            </div>
            <div className="font-mono mt-0.5 text-[#bacbbc]" style={{ fontSize: "0.6rem" }}>
              Configure overlay geometry, glass presets & glyphs
            </div>
          </div>
          <motion.button
            onClick={onCancel}
            className="p-1 cursor-pointer text-[#bacbbc]"
            whileHover={{ color: "#e1e1f2", scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="close" size={18} />
          </motion.button>
        </div>

        <div className="flex-1 px-5 py-4 space-y-5">
          {/* ── Enable/Disable Toggle ── */}
          <Section label="KEYBOARD OVERLAY STATUS">
            <div className="flex items-center justify-between py-1">
              <span className="font-mono text-xs text-[#bacbbc]">Enable Overlay</span>
              <button
                onClick={() => update("enabled", !overlay.enabled)}
                className="w-12 h-6 flex items-center rounded-full p-1 cursor-pointer transition-colors duration-200"
                style={{
                  backgroundColor: overlay.enabled ? "#00e88a" : "#2d314d",
                }}
              >
                <motion.div
                  className="w-4 h-4 bg-[#0d0f1a] rounded-full shadow-md"
                  layout
                  transition={{ type: "spring", stiffness: 500, damping: 30 }}
                  style={{
                    marginLeft: overlay.enabled ? "1.5rem" : "0rem",
                  }}
                />
              </button>
            </div>
          </Section>

          {/* ── Test Sandbox Input ── */}
          <Section label="TEST TYPING SANDBOX">
            <div className="space-y-1">
              <input
                ref={sandboxRef}
                type="text"
                value={sandboxText}
                onChange={(e) => setSandboxText(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Type keys here to test overlay..."
                className="px-3 py-2 font-mono text-xs w-full outline-none border border-[#2d314d] bg-[#090b14] text-[#e1e1f2] rounded focus:border-[#00e88a]"
                disabled={!overlay.enabled}
              />
              <div className="flex justify-between font-mono" style={{ fontSize: "0.55rem", color: "#bacbbc" }}>
                <span>Press Enter to clear sandbox text</span>
                {bubbles.length > 0 && (
                  <button
                    onClick={() => {
                      setBubbles([]);
                      setSandboxText("");
                      sandboxRef.current?.focus();
                    }}
                    className="text-[#00e88a] hover:underline"
                  >
                    Clear Preview
                  </button>
                )}
              </div>
            </div>
          </Section>

          {/* ── Preset Themes ── */}
          <Section label="LIQUID GLASS THEME PRESET">
            <div className="space-y-1.5">
              {PRESET_THEMES.map((theme) => (
                <button
                  key={theme.id}
                  onClick={() => update("theme", theme.id)}
                  className="w-full flex items-center justify-between px-3 py-2 rounded border font-mono text-xs text-left cursor-pointer transition-all duration-150"
                  style={{
                    background: isThemeSelected(theme.id) ? "rgba(0, 232, 138, 0.08)" : "#090b14",
                    borderColor: isThemeSelected(theme.id) ? "#00e88a" : "#2d314d",
                    color: isThemeSelected(theme.id) ? "#00e88a" : "#e1e1f2",
                  }}
                >
                  <span>{theme.name}</span>
                  {isThemeSelected(theme.id) && (
                    <Icon name="check" size={14} style={{ color: "#00e88a" }} />
                  )}
                </button>
              ))}
            </div>
          </Section>

          {/* ── Font Family ── */}
          <Section label="FONT FAMILY">
            <select
              value={overlay.fontFamily}
              onChange={(e) => update("fontFamily", e.target.value)}
              className="px-3 py-1.5 font-mono text-xs cursor-pointer w-full outline-none border border-[#2d314d] bg-[#090b14] text-[#e1e1f2] rounded focus:border-[#00e88a]"
            >
              {FONT_FAMILIES.map((f) => (
                <option key={f.value} value={f.value}>
                  {f.label}
                </option>
              ))}
            </select>
          </Section>

          {/* ── Font Size ── */}
          <Section label="FONT SIZE">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={10}
                max={32}
                value={overlay.fontSize}
                onChange={(e) => update("fontSize", Number(e.target.value))}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-10 text-right">
                {overlay.fontSize}px
              </span>
            </div>
          </Section>

          {/* ── Width ── */}
          <Section label="OVERLAY WIDTH">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={200}
                max={recordingWidth}
                value={overlay.width}
                onChange={(e) => update("width", Number(e.target.value))}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-10 text-right">
                {overlay.width}px
              </span>
            </div>
          </Section>

          {/* ── Opacity ── */}
          <Section label="OVERLAY OPACITY">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={30}
                max={100}
                value={Math.round(overlay.opacity * 100)}
                onChange={(e) => update("opacity", Number(e.target.value) / 100)}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">
                {Math.round(overlay.opacity * 100)}%
              </span>
            </div>
          </Section>

          {/* ── Max Bubbles Limit ── */}
          <Section label="MAX VISIBLE BLOCKS">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={1}
                max={10}
                value={overlay.maxBubbles}
                onChange={(e) => update("maxBubbles", Number(e.target.value))}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">
                {overlay.maxBubbles}
              </span>
            </div>
          </Section>

          {/* ── Bubble Timeout ── */}
          <Section label="BLOCK TIMEOUT">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={1}
                max={10}
                value={Math.round(overlay.bubbleTimeoutMs / 1000)}
                onChange={(e) => update("bubbleTimeoutMs", Number(e.target.value) * 1000)}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">
                {Math.round(overlay.bubbleTimeoutMs / 1000)}s
              </span>
            </div>
          </Section>

          {/* ── Custom Theme Details (if custom selected) ── */}
          {overlay.theme === "custom" && (
            <Section label="CUSTOM COLORS">
              <div className="space-y-2 pt-1">
                <div className="flex justify-between items-center gap-2">
                  <span className="font-mono text-[10px] text-[#bacbbc]">Text Color</span>
                  <input
                    type="color"
                    value={overlay.textColor}
                    onChange={(e) => update("textColor", e.target.value)}
                    className="w-10 h-6 border-none bg-transparent cursor-pointer rounded"
                  />
                </div>
                <div className="flex justify-between items-center gap-2">
                  <span className="font-mono text-[10px] text-[#bacbbc]">Background</span>
                  <input
                    type="color"
                    value={overlay.backgroundColor.startsWith("rgba") ? "#000000" : overlay.backgroundColor}
                    onChange={(e) => update("backgroundColor", e.target.value)}
                    className="w-10 h-6 border-none bg-transparent cursor-pointer rounded"
                  />
                </div>
                <div className="flex justify-between items-center gap-2">
                  <span className="font-mono text-[10px] text-[#bacbbc]">Border Color</span>
                  <input
                    type="color"
                    value={overlay.borderColor.startsWith("rgba") ? "#ffffff" : overlay.borderColor}
                    onChange={(e) => update("borderColor", e.target.value)}
                    className="w-10 h-6 border-none bg-transparent cursor-pointer rounded"
                  />
                </div>
              </div>
            </Section>
          )}

          {/* ── Geometry Borders/Corners ── */}
          <Section label="BORDER & CORNERS">
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <span className="font-mono text-[10px] text-[#bacbbc] w-20">Corners</span>
                <input
                  type="range"
                  min={0}
                  max={24}
                  value={overlay.cornerRadius}
                  onChange={(e) => update("cornerRadius", Number(e.target.value))}
                  className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
                />
                <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">
                  {overlay.cornerRadius}px
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="font-mono text-[10px] text-[#bacbbc] w-20">Border Width</span>
                <input
                  type="range"
                  min={0}
                  max={6}
                  value={overlay.borderWidth}
                  onChange={(e) => update("borderWidth", Number(e.target.value))}
                  className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
                />
                <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">
                  {overlay.borderWidth}px
                </span>
              </div>
            </div>
          </Section>

          {/* ── Key Glyph Emojis Mappings ── */}
          <Section label="INDIVIDUAL KEY MAPPING">
            <div className="space-y-2 border border-[#2d314d] rounded bg-[#090b14] p-3 max-h-48 overflow-y-auto scrollbar-thin">
              {Object.keys(parsedMappings).map((mappingKey) => (
                <div key={mappingKey} className="flex items-center justify-between gap-2">
                  <span className="font-mono text-[10px] text-[#bacbbc]">{mappingKey}</span>
                  <input
                    type="text"
                    value={parsedMappings[mappingKey] || ""}
                    onChange={(e) => handleUpdateMapping(mappingKey, e.target.value)}
                    maxLength={10}
                    className="px-2 py-0.5 font-mono text-[10px] w-20 outline-none border border-[#2d314d] bg-[#151828] text-center text-[#00e88a] rounded"
                  />
                </div>
              ))}
            </div>
          </Section>
        </div>

        {/* Bottom buttons */}
        <div className="px-5 py-4 flex gap-3 border-t border-[#2d314d]">
          <motion.button
            onClick={onCancel}
            className="flex-1 py-2 font-mono text-xs uppercase cursor-pointer border border-[#2d314d] rounded text-[#bacbbc]"
            whileHover={{ borderColor: "#bacbbc", color: "#e1e1f2" }}
            whileTap={{ scale: 0.97 }}
          >
            Cancel
          </motion.button>
          <motion.button
            onClick={() => onSave(overlay)}
            className="flex-1 py-2 font-mono text-xs uppercase font-extrabold cursor-pointer rounded text-[#00391e]"
            style={{ background: "#00e88a" }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.97 }}
          >
            Apply
          </motion.button>
        </div>
      </div>

      {/* ═══ MAIN AREA — Preview Canvas ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center overflow-hidden bg-[#0d0f1a] relative">
        <div className="absolute inset-0 pointer-events-none opacity-5 bg-[linear-gradient(to_right,#808080_1px,transparent_1px),linear-gradient(to_bottom,#808080_1px,transparent_1px)] bg-[size:24px_24px]"></div>

        {/* Recording Monitor Aspect-Video simulation */}
        <div
          className="relative shadow-2xl border border-[#2d314d]/50 bg-[#090b14]"
          style={{
            width: previewW,
            height: previewH,
            borderRadius: "8px",
            overflow: "hidden",
          }}
        >
          {/* Mock dashboard silhouette */}
          <div className="absolute inset-0 opacity-10 pointer-events-none p-6 grid grid-cols-12 gap-4">
            <div className="col-span-3 h-32 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-9 h-32 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
          </div>

          {/* Reference guidelines */}
          <div className="absolute inset-0 pointer-events-none opacity-5 border border-dashed border-[#bacbbc]"></div>

          {/* ═══ RND DRAGGABLE KEYBOARD OVERLAY ═══ */}
          {overlay.enabled && (
            <Rnd
              size={{ width: overlay.width * scale, height: 80 * scale }}
              position={{ x: overlay.x * scale, y: overlay.y * scale }}
              onDragStop={(_, d) => {
                update("x", Math.round(d.x / scale));
                update("y", Math.round(d.y / scale));
              }}
              onResizeStop={(_e, _direction, ref, _delta, position) => {
                update("width", Math.round(ref.offsetWidth / scale));
                update("x", Math.round(position.x / scale));
              }}
              enableResizing={{
                top: false,
                right: true,
                bottom: false,
                left: true,
                topRight: false,
                bottomRight: false,
                bottomLeft: false,
                topLeft: false,
              }}
              minWidth={200 * scale}
              maxWidth={recordingWidth * scale}
              bounds="parent"
              style={{
                opacity: overlay.opacity,
                zIndex: 10,
              }}
            >
              <div
                className="flex items-center justify-center relative group select-none cursor-grab active:cursor-grabbing w-full h-full"
                style={{
                  background: overlay.backgroundColor,
                  borderColor: overlay.borderColor,
                  borderWidth: `${overlay.borderWidth * scale}px`,
                  borderStyle: overlay.borderWidth > 0 ? "solid" : "none",
                  borderRadius: `${overlay.cornerRadius * scale}px`,
                  fontFamily: overlay.fontFamily,
                  color: overlay.textColor,
                  gap: `${10 * scale}px`,
                  padding: `${8 * scale}px ${16 * scale}px`,
                  backdropFilter: `blur(${12 * scale}px)`,
                  boxShadow: `0 ${4 * scale}px ${30 * scale}px rgba(0, 0, 0, 0.4), inset 0 ${1 * scale}px 0 rgba(255,255,255,0.1)`,
                }}
              >
                {displayBubbles.map((bubble) => (
                  <motion.div
                    key={bubble.id}
                    initial={{ scale: 0.8, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    className="font-bold flex items-center justify-center text-center rounded shadow-sm border border-white/5 bg-white/5"
                    style={{
                      fontSize: `${overlay.fontSize * scale}px`,
                      padding: `${6 * scale}px ${12 * scale}px`,
                      minWidth: `${32 * scale}px`,
                      whiteSpace: "nowrap",
                    }}
                  >
                    {bubble.text}
                  </motion.div>
                ))}
              </div>
            </Rnd>
          )}
        </div>

        {/* Caption */}
        <div className="mt-4 font-mono text-center text-[#bacbbc]" style={{ fontSize: "0.65rem", letterSpacing: "0.05em" }}>
          DRAG OVERLAY TO POSITION · CLICK SIDEBAR TEXT FIELD TO TYPE
        </div>
      </div>
    </motion.div>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <div className="font-mono text-[9px] text-[#bacbbc] tracking-widest font-bold uppercase">
        {label}
      </div>
      {children}
    </div>
  );
}
