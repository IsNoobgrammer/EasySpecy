import { Icon } from "./Icon";
import { useState, useEffect, useId, useRef } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore, AppConfig } from "../stores/recording";
import { useThemeStore } from "../lib/theme";

export function Settings({ onBack }: { onBack: () => void }) {
  const { config, saveConfig, loadAudioDevices, audioDevices } = useStore();
  const { theme, toggleTheme } = useThemeStore();
  const [local, setLocal] = useState<AppConfig | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (config) setLocal({ ...config });
    loadAudioDevices();
  }, [config]);

  const handleSave = async () => {
    if (!local) return;
    await saveConfig(local);
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  if (!local) {
    return (
      <div className="flex items-center justify-center h-full" style={{ background: "var(--bg-base)" }}>
        <motion.div
          className="font-mono text-sm"
          style={{ color: "var(--text-muted)", letterSpacing: "0.05em" }}
          animate={{ opacity: [0.4, 1, 0.4] }}
          transition={{ duration: 1.5, repeat: Infinity }}
        >
          Loading settings...
        </motion.div>
      </div>
    );
  }

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setLocal((prev) => (prev ? { ...prev, [key]: value } : prev));

  return (
    <div className="flex flex-col h-full" style={{ background: "var(--bg-base)" }}>
      {/* ═══ HEADER ═══ */}
      <header
        className="flex items-center justify-between px-6 py-3 sticky top-0 z-50 backdrop-blur-md"
        style={{ borderBottom: "var(--border-width) solid var(--border-default)", background: "oklch(from var(--bg-base) l c h / 0.85)" }}
      >
        <motion.button
          onClick={onBack}
          className="flex items-center gap-2 font-mono text-xs cursor-pointer"
          style={{ color: "var(--text-secondary)", letterSpacing: "0.03em" }}
          whileHover={{ x: -3, color: "var(--text-primary)" }}
          whileTap={{ scale: 0.95 }}
        >
          <Icon name="arrow_back" size={16} /> Back
        </motion.button>
        <span className="font-mono text-sm font-semibold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>
          Settings
        </span>
        <motion.button
          onClick={toggleTheme}
          className="font-mono text-xs px-2 py-1 cursor-pointer flex items-center gap-1.5"
          style={{ border: "var(--border-thin) solid var(--border-default)", color: "var(--text-muted)", borderRadius: "var(--radius-sm)" }}
          whileHover={{ scale: 1.05, borderColor: "var(--border-strong)" }}
          whileTap={{ scale: 0.95 }}
        >
          <Icon name={theme === "dark" ? "light_mode" : "dark_mode"} size={14} />
          {theme === "dark" ? "Light" : "Dark"}
        </motion.button>
      </header>

      {/* ═══ CONTENT ═══ */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="max-w-xl mx-auto space-y-4">

          {/* ── Video Section ── */}
          <Card title="Video" icon="videocam" index={0}>
            <Row label="Resolution" desc="Output video dimensions">
              <Select
                value={`${local.resolution_width}x${local.resolution_height}`}
                onChange={(v) => { const [w, h] = v.split("x").map(Number); update("resolution_width", w); update("resolution_height", h); }}
                options={[
                  { label: "480p — 854×480", value: "854x480" },
                  { label: "720p — 1280×720", value: "1280x720" },
                  { label: "1080p — 1920×1080", value: "1920x1080" },
                ]}
              />
            </Row>
            <Row label="Frame Rate" desc="Frames per second">
              <Segmented
                value={local.fps}
                options={[{ label: "24", value: 24 }, { label: "30", value: 30 }, { label: "60", value: 60 }]}
                onChange={(v) => update("fps", v)}
              />
            </Row>
            <Row label="Capture Mode" desc="What to record">
              <Select
                value={local.recording_mode}
                onChange={(v) => update("recording_mode", v as AppConfig["recording_mode"])}
                options={[
                  { label: "Full Screen", value: "FullScreen" },
                  { label: "Region Select", value: "Region" },
                ]}
              />
            </Row>
          </Card>

          {/* ── Audio Section ── */}
          <Card title="Audio" icon="mic" index={1}>
            <Row label="Record Audio" desc="Capture audio alongside video">
              <Toggle checked={local.audio_enabled} onChange={(v) => update("audio_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.audio_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden"
                >
                  <div className="space-y-3 pt-1">
                    <Row label="Source" desc="What audio to capture">
                      <Segmented
                        value={local.audio_source}
                        options={[
                          { label: "Mic", value: "Mic" },
                          { label: "System", value: "System" },
                          { label: "Both", value: "Both" },
                        ]}
                        onChange={(v) => update("audio_source", v as AppConfig["audio_source"])}
                      />
                    </Row>
                    <Row label="Sample Rate" desc="Audio quality">
                      <Select
                        value={local.audio_sample_rate}
                        onChange={(v) => update("audio_sample_rate", Number(v))}
                        options={[
                          { label: "22050 Hz — Low", value: 22050 },
                          { label: "44100 Hz — CD Quality", value: 44100 },
                          { label: "48000 Hz — Studio", value: 48000 },
                        ]}
                      />
                    </Row>
                    <Row label="Input Device" desc="Microphone to use">
                      <Select
                        value={local.audio_device}
                        onChange={(v) => update("audio_device", v)}
                        options={[
                          { label: "System Default", value: "default" },
                          ...audioDevices.map((d) => ({ label: d, value: d })),
                        ]}
                      />
                    </Row>
                    <Row label="Mic Gain" desc="Boost or reduce mic volume (1.0 = normal)">
                      <Slider value={local.mic_gain} min={0} max={3} step={0.1} onChange={(v) => update("mic_gain", v)} suffix="×" />
                    </Row>
                    {(local.audio_source === "Both" || local.audio_source === "System") && (
                      <Row label="System Volume" desc="System audio level in mix">
                        <Slider value={local.system_volume} min={0} max={1} step={0.05} onChange={(v) => update("system_volume", v)} suffix="" />
                      </Row>
                    )}
                    <Row label="Noise Gate" desc="Suppress background noise (0 = off, 1 = aggressive)">
                      <Slider value={local.noise_gate_threshold} min={0} max={1} step={0.05} onChange={(v) => update("noise_gate_threshold", v)} suffix="" />
                    </Row>
                    <Row label="Noise Reduction" desc="Remove constant hiss/hum (0 = off, 1 = max)">
                      <Slider value={local.noise_reduction} min={0} max={1} step={0.05} onChange={(v) => update("noise_reduction", v)} suffix="" />
                    </Row>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Auto-Zoom Section ── */}
          <Card title="Auto-Zoom" icon="zoom_in" index={2} badge="Post-processing">
            <Row label="Enabled" desc="Zoom toward click positions after recording">
              <Toggle checked={local.auto_zoom_enabled} onChange={(v) => update("auto_zoom_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.auto_zoom_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden"
                >
                  <div className="space-y-3 pt-1">
                    <Row label="Zoom Level" desc="How much to zoom in">
                      <Segmented
                        value={local.zoom_level}
                        options={[
                          { label: "1.5×", value: 1.5 },
                          { label: "2×", value: 2 },
                          { label: "2.5×", value: 2.5 },
                          { label: "3×", value: 3 },
                        ]}
                        onChange={(v) => update("zoom_level", v as number)}
                      />
                    </Row>
                    <Row label="Hold Duration" desc="Time to stay zoomed in">
                      <Select
                        value={local.zoom_dwell_ms}
                        onChange={(v) => update("zoom_dwell_ms", Number(v))}
                        options={[
                          { label: "1 second", value: 1000 },
                          { label: "1.5 seconds", value: 1500 },
                          { label: "2 seconds", value: 2000 },
                          { label: "3 seconds", value: 3000 },
                        ]}
                      />
                    </Row>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Cursor Effects ── */}
          <Card title="Cursor Effects" icon="auto_fix_high" index={3} badge="Post-processing">
            <Row label="Cursor Pack" desc="Replace cursor style during recording">
              <CursorPackSelector value={local.cursor_pack || "default"} onChange={(v) => update("cursor_pack" as any, v)} />
            </Row>
            <Row label="Trail Effect" desc="Glowing trail follows cursor path">
              <Toggle checked={local.cursor_trail_enabled} onChange={(v) => update("cursor_trail_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.cursor_trail_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden"
                >
                  <div className="space-y-3 pt-1">
                    <Row label="Trail Color" desc="Color of the cursor trail">
                      <div className="flex items-center gap-2">
                        <input
                          type="color"
                          value={local.cursor_trail_color}
                          onChange={(e) => update("cursor_trail_color", e.target.value)}
                          className="w-8 h-8 cursor-pointer"
                          style={{ border: "var(--border-width) solid var(--border-default)", background: "transparent" }}
                        />
                        <span className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>
                          {local.cursor_trail_color.toUpperCase()}
                        </span>
                      </div>
                    </Row>
                    <Row label="Cursor Size" desc="Enlarge cursor in output">
                      <Segmented
                        value={local.cursor_size_multiplier}
                        options={[
                          { label: "1×", value: 1 },
                          { label: "1.5×", value: 1.5 },
                          { label: "2×", value: 2 },
                          { label: "3×", value: 3 },
                        ]}
                        onChange={(v) => update("cursor_size_multiplier", v as number)}
                      />
                    </Row>
                    <Row label="Smoothing" desc="Smooth out jittery cursor movements">
                      <Toggle checked={local.cursor_smoothing} onChange={(v) => update("cursor_smoothing", v)} />
                    </Row>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Webcam ── */}
          <Card title="Webcam" icon="videocam" index={4}>
            <Row label="Enabled" desc="Overlay webcam on recording">
              <Toggle checked={local.webcam_enabled} onChange={(v) => update("webcam_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.webcam_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden"
                >
                  <div className="space-y-3 pt-1">
                    <Row label="Position" desc="Where to place webcam overlay">
                      <Segmented
                        value={local.webcam_position}
                        options={[
                          { label: "TL", value: "TopLeft" },
                          { label: "TR", value: "TopRight" },
                          { label: "BL", value: "BottomLeft" },
                          { label: "BR", value: "BottomRight" },
                        ]}
                        onChange={(v) => update("webcam_position", v as AppConfig["webcam_position"])}
                      />
                    </Row>
                    <Row label="Size" desc="Webcam overlay dimensions">
                      <Select
                        value={local.webcam_size}
                        onChange={(v) => update("webcam_size", Number(v))}
                        options={[
                          { label: "150px — Small", value: 150 },
                          { label: "200px — Medium", value: 200 },
                          { label: "250px — Large", value: 250 },
                          { label: "300px — Extra Large", value: 300 },
                        ]}
                      />
                    </Row>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Hotkeys ── */}
          <Card title="Hotkeys" icon="keyboard" index={5}>
            <Row label="Start Recording" desc="Keyboard shortcut to begin">
              <HotkeyRecorder value={local.hotkey_start} onChange={(v) => update("hotkey_start", v)} />
            </Row>
            <Row label="Stop Recording" desc="Keyboard shortcut to stop">
              <HotkeyRecorder value={local.hotkey_stop} onChange={(v) => update("hotkey_stop", v)} />
            </Row>
            <Row label="Pause / Resume" desc="Keyboard shortcut to pause">
              <HotkeyRecorder value={local.hotkey_pause} onChange={(v) => update("hotkey_pause", v)} />
            </Row>
          </Card>

          {/* ── General ── */}
          <Card title="General" icon="settings" index={6}>
            <Row label="Output Directory" desc="Where recordings are saved">
              <Input value={local.output_dir} onChange={(v) => update("output_dir", v)} />
            </Row>
            <Row label="Minimize to Tray" desc="Keep running in background">
              <Toggle checked={local.minimize_to_tray} onChange={(v) => update("minimize_to_tray", v)} />
            </Row>
            <Row label="Copy Path on Save" desc="Copy file path to clipboard">
              <Toggle checked={local.copy_path_on_save} onChange={(v) => update("copy_path_on_save", v)} />
            </Row>
          </Card>
        </div>
      </div>

      {/* ═══ SAVE BUTTON ═══ */}
      <div className="px-6 py-4" style={{ borderTop: "var(--border-width) solid var(--border-default)", background: "oklch(from var(--bg-base) l c h / 0.5)" }}>
        <motion.button
          onClick={handleSave}
          className="w-full py-3 font-mono text-xs uppercase font-bold cursor-pointer shadow-md flex items-center justify-center gap-2"
          style={{
            border: "none",
            background: saved ? "var(--accent-success)" : "var(--accent-primary-container, #00e88a)",
            color: saved ? "#fff" : "var(--on-primary, #00391e)",
            borderRadius: "var(--radius-sm)",
            letterSpacing: "0.05em",
          }}
          whileHover={{ scale: 1.01, y: -1, boxShadow: "var(--shadow-lg)", borderColor: "var(--border-strong)" }}
          whileTap={{ scale: 0.98 }}
        >
          {saved ? "✓ SAVED SUCCESSFULLY" : "SAVE SETTINGS"}
        </motion.button>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════
   COMPONENTS
   ═══════════════════════════════════════════════════════ */

function Card({
  title, icon, children, index, badge,
}: {
  title: string; icon: string; children: React.ReactNode; index: number; badge?: string;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.05, duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
      className="shadow-sm overflow-hidden"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-surface)",
        borderRadius: "var(--radius-md)",
      }}
    >
      {/* Card Header */}
      <div
        className="flex items-center justify-between px-4 py-3"
        style={{ borderBottom: "var(--border-thin) solid var(--border-default)", background: "oklch(from var(--bg-surface) l c h / 0.3)" }}
      >
        <div className="flex items-center gap-2">
                    <Icon name={icon} size={18} style={{ color: "var(--accent-primary)" }} />
          <span className="font-mono text-xs font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.03em" }}>
            {title}
          </span>
        </div>
        {badge && (
          <span
            className="font-mono px-2 py-0.5"
            style={{
              fontSize: "0.55rem",
              color: "var(--text-muted)",
              border: "var(--border-thin) solid var(--border-default)",
              borderRadius: "var(--radius-xs)",
              letterSpacing: "0.05em",
            }}
          >
            {badge}
          </span>
        )}
      </div>

      {/* Card Body */}
      <div className="px-4 py-3.5 space-y-4">
        {children}
      </div>
    </motion.div>
  );
}

function Row({ label, desc, children }: { label: string; desc?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <div className="font-mono text-xs font-semibold" style={{ color: "var(--text-primary)" }}>
          {label}
        </div>
        {desc && (
          <div className="font-mono mt-0.5" style={{ color: "var(--text-muted)", fontSize: "0.58rem", lineHeight: 1.4 }}>
            {desc}
          </div>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Select({ value, onChange, options }: {
  value: string | number; onChange: (v: string) => void;
  options: { label: string; value: string | number }[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="px-3 py-1.5 font-mono text-xs cursor-pointer min-w-[160px] outline-none transition-colors duration-200"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-base)",
        color: "var(--text-primary)",
        borderRadius: "var(--radius-sm)",
        boxShadow: "var(--shadow-sm)",
      }}
      onFocus={(e) => e.target.style.borderColor = "var(--border-focus)"}
      onBlur={(e) => e.target.style.borderColor = "var(--border-default)"}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value} style={{ background: "var(--bg-elevated)", color: "var(--text-primary)" }}>{o.label}</option>
      ))}
    </select>
  );
}

function Segmented<T extends string | number>({ value, options, onChange }: {
  value: T; options: { label: string; value: T }[]; onChange: (v: T) => void;
}) {
  const layoutId = useId();
  return (
    <div className="flex p-0.5 relative gap-0.5" style={{ border: "var(--border-width) solid var(--border-default)", background: "var(--bg-base)", borderRadius: "var(--radius-sm)" }}>
      {options.map((opt) => (
        <motion.button
          key={String(opt.value)}
          onClick={() => onChange(opt.value)}
          className="px-3 py-1 font-mono text-[10px] cursor-pointer relative z-10 font-bold"
          style={{
            color: opt.value === value ? "var(--bg-base)" : "var(--text-secondary)",
            border: "none",
            background: "transparent",
            letterSpacing: "0.03em",
          }}
          whileHover={opt.value !== value ? { color: "var(--text-primary)" } : {}}
          whileTap={{ scale: 0.96 }}
        >
          <span className="relative z-20">{opt.label}</span>
          {opt.value === value && (
            <motion.div
              layoutId={layoutId}
              className="absolute inset-0 z-0 shadow-sm"
              style={{
                background: "var(--accent-primary)",
                borderRadius: "calc(var(--radius-sm) - 3px)",
              }}
              transition={{ type: "spring", stiffness: 450, damping: 28 }}
            />
          )}
        </motion.button>
      ))}
    </div>
  );
}

function Input({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="px-3 py-1.5 font-mono text-xs min-w-[160px] outline-none transition-colors duration-200"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-base)",
        color: "var(--text-primary)",
        borderRadius: "var(--radius-sm)",
        boxShadow: "var(--shadow-sm)",
      }}
      onFocus={(e) => e.target.style.borderColor = "var(--border-focus)"}
      onBlur={(e) => e.target.style.borderColor = "var(--border-default)"}
    />
  );
}

function HotkeyRecorder({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [isRecording, setIsRecording] = useState(false);
  const [tempModifiers, setTempModifiers] = useState({
    ctrl: false,
    shift: false,
    alt: false,
    super: false,
  });
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!isRecording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;
      const isModifier = ["Control", "Shift", "Alt", "Meta", "OS"].includes(key);

      // Track current modifiers state in real-time
      const newModifiers = {
        ctrl: e.ctrlKey || key === "Control",
        shift: e.shiftKey || key === "Shift",
        alt: e.altKey || key === "Alt",
        super: e.metaKey || key === "Meta" || key === "OS",
      };
      setTempModifiers(newModifiers);

      if (key === "Escape") {
        setIsRecording(false);
        buttonRef.current?.blur();
        return;
      }

      if (key === "Backspace" && !newModifiers.ctrl && !newModifiers.shift && !newModifiers.alt && !newModifiers.super) {
        onChange("");
        setIsRecording(false);
        buttonRef.current?.blur();
        return;
      }

      if (!isModifier) {
        // A non-modifier key was pressed - compile shortcut
        const parts: string[] = [];
        if (newModifiers.ctrl) parts.push("Ctrl");
        if (newModifiers.shift) parts.push("Shift");
        if (newModifiers.alt) parts.push("Alt");
        if (newModifiers.super) parts.push("Super");

        // Format key name nicely
        let keyName = key;
        if (key === " ") {
          keyName = "Space";
        } else if (key.length === 1) {
          keyName = key.toUpperCase();
        } else if (key.startsWith("Arrow")) {
          keyName = key.replace("Arrow", ""); // Up, Down, Left, Right
        }

        parts.push(keyName);
        const shortcutString = parts.join("+");

        onChange(shortcutString);
        setIsRecording(false);
        buttonRef.current?.blur();
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Update modifier states when they are released
      setTempModifiers({
        ctrl: e.ctrlKey,
        shift: e.shiftKey,
        alt: e.altKey,
        super: e.metaKey,
      });
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("keyup", handleKeyUp, true);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("keyup", handleKeyUp, true);
    };
  }, [isRecording, onChange]);

  const handleClick = () => {
    setIsRecording(true);
    setTempModifiers({ ctrl: false, shift: false, alt: false, super: false });
  };

  const handleBlur = () => {
    setIsRecording(false);
  };

  return (
    <div className="relative">
      <motion.button
        ref={buttonRef}
        onClick={handleClick}
        onBlur={handleBlur}
        className="px-3 py-1.5 font-mono text-xs cursor-pointer min-w-[160px] text-center outline-none select-none transition-all duration-200"
        style={{
          border: "var(--border-width) solid " + (isRecording ? "var(--border-focus)" : "var(--border-default)"),
          background: isRecording ? "oklch(from var(--border-focus) l c h / 0.08)" : "var(--bg-base)",
          color: isRecording ? "var(--border-focus)" : "var(--text-primary)",
          borderRadius: "var(--radius-sm)",
          boxShadow: isRecording ? "0 0 10px oklch(from var(--border-focus) l c h / 0.15)" : "var(--shadow-sm)",
        }}
        whileTap={{ scale: 0.98 }}
      >
        {isRecording ? (
          <span className="flex items-center justify-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-current animate-pulse-dot" />
            Listening...
          </span>
        ) : (
          value || "None"
        )}
      </motion.button>

      <AnimatePresence>
        {isRecording && (
          <motion.div
            initial={{ opacity: 0, y: 4, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.95 }}
            transition={{ duration: 0.15, ease: "easeOut" }}
            className="absolute top-full mt-2 left-1/2 -translate-x-1/2 flex gap-1 p-1 bg-elevated border border-default shadow-lg z-50 rounded"
            style={{
              background: "var(--bg-elevated)",
              border: "var(--border-thin) solid var(--border-default)",
              borderRadius: "var(--radius-sm)",
              boxShadow: "var(--shadow-lg)",
            }}
          >
            <span
              className="px-1.5 py-0.5 rounded text-[10px] transition-colors duration-150 font-bold"
              style={{
                background: tempModifiers.ctrl ? "var(--accent-primary)" : "var(--bg-base)",
                color: tempModifiers.ctrl ? "var(--bg-base)" : "var(--text-muted)",
                border: tempModifiers.ctrl ? "var(--border-thin) solid var(--accent-primary)" : "var(--border-thin) dashed var(--border-default)",
              }}
            >
              Ctrl
            </span>
            <span
              className="px-1.5 py-0.5 rounded text-[10px] transition-colors duration-150 font-bold"
              style={{
                background: tempModifiers.shift ? "var(--accent-primary)" : "var(--bg-base)",
                color: tempModifiers.shift ? "var(--bg-base)" : "var(--text-muted)",
                border: tempModifiers.shift ? "var(--border-thin) solid var(--accent-primary)" : "var(--border-thin) dashed var(--border-default)",
              }}
            >
              Shift
            </span>
            <span
              className="px-1.5 py-0.5 rounded text-[10px] transition-colors duration-150 font-bold"
              style={{
                background: tempModifiers.alt ? "var(--accent-primary)" : "var(--bg-base)",
                color: tempModifiers.alt ? "var(--bg-base)" : "var(--text-muted)",
                border: tempModifiers.alt ? "var(--border-thin) solid var(--accent-primary)" : "var(--border-thin) dashed var(--border-default)",
              }}
            >
              Alt
            </span>
            <span
              className="px-1.5 py-0.5 rounded text-[10px] transition-colors duration-150 font-bold"
              style={{
                background: tempModifiers.super ? "var(--accent-primary)" : "var(--bg-base)",
                color: tempModifiers.super ? "var(--bg-base)" : "var(--text-muted)",
                border: tempModifiers.super ? "var(--border-thin) solid var(--accent-primary)" : "var(--border-thin) dashed var(--border-default)",
              }}
            >
              Win
            </span>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function Slider({ value, min, max, step, onChange, suffix }: {
  value: number; min: number; max: number; step: number; onChange: (v: number) => void; suffix?: string;
}) {
  return (
    <div className="flex items-center gap-2 min-w-[160px]">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="flex-1 h-1.5 cursor-pointer accent-[var(--accent-primary)]"
        style={{ background: "var(--bg-base)", borderRadius: "var(--radius-full)" }}
      />
      <span className="font-mono text-[10px] min-w-[36px] text-right" style={{ color: "var(--text-muted)" }}>
        {value.toFixed(step < 0.1 ? 2 : 1)}{suffix}
      </span>
    </div>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <motion.button
      onClick={() => onChange(!checked)}
      className="relative cursor-pointer"
      style={{
        width: 44,
        height: 24,
        border: `var(--border-width) solid ${checked ? "var(--accent-primary)" : "var(--border-default)"}`,
        background: checked ? "var(--accent-primary)" : "var(--bg-base)",
        borderRadius: "var(--radius-full)",
      }}
      whileTap={{ scale: 0.95 }}
    >
      <motion.div
        className="absolute top-0.5"
        style={{ width: 16, height: 16, background: "var(--text-primary)", borderRadius: "var(--radius-full)" }}
        animate={{ left: checked ? 22 : 2 }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}

function CursorPackSelector({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const packs = [
    { id: "default", name: "System Default", desc: "No change", color: "#8b949e" },
    { id: "macos", name: "macOS", desc: "Apple-style", color: "#ffffff" },
    { id: "posy", name: "Posy's", desc: "Community fav", color: "#ffffff" },
    { id: "neon_green", name: "Neon Green", desc: "Bright glow", color: "#00ff88" },
    { id: "neon_pink", name: "Neon Pink", desc: "Hot pink", color: "#ff44cc" },
    { id: "minimal_dot", name: "Minimal Dot", desc: "Clean circle", color: "#ffffff" },
    { id: "crosshair", name: "Crosshair", desc: "Precision", color: "#ff4444" },
    { id: "retro_pixel", name: "Retro Pixel", desc: "8-bit style", color: "#ffff00" },
    { id: "glass_arrow", name: "Glass", desc: "Translucent", color: "#ccccff" },
    { id: "easyspecy", name: "EasySpecy", desc: "Branded", color: "#00e88a" },
  ];

  const selected = packs.find(p => p.id === value) || packs[0];

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={ref} className="relative">
      <motion.button
        onClick={() => setOpen(!open)}
        className="px-3 py-1.5 font-mono text-xs cursor-pointer min-w-[160px] flex items-center gap-2 outline-none"
        style={{
          border: `var(--border-width) solid ${open ? "var(--accent-primary)" : "var(--border-default)"}`,
          background: "var(--bg-base)",
          color: "var(--text-primary)",
          borderRadius: "var(--radius-sm)",
          boxShadow: "var(--shadow-sm)",
        }}
        whileTap={{ scale: 0.98 }}
      >
        <span className="w-3 h-3 rounded-full" style={{ background: selected.color, border: "1px solid rgba(255,255,255,0.2)" }} />
        <span className="flex-1 text-left">{selected.name}</span>
        <span style={{ fontSize: "0.5rem", color: "var(--text-muted)", transform: open ? "rotate(180deg)" : "rotate(0deg)", transition: "transform 0.2s" }}>▼</span>
      </motion.button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: 4, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className="absolute right-0 top-full mt-1 z-50 w-[220px] max-h-[280px] overflow-y-auto"
            style={{
              border: "var(--border-thin) solid var(--border-strong)",
              background: "var(--bg-elevated)",
              boxShadow: "var(--shadow-lg)",
              borderRadius: "var(--radius-sm)",
            }}
          >
            {packs.map((pack) => (
              <motion.button
                key={pack.id}
                className="w-full px-3 py-2 flex items-center gap-2.5 cursor-pointer text-left"
                style={{
                  color: pack.id === value ? "var(--text-primary)" : "var(--text-secondary)",
                  background: pack.id === value ? "var(--bg-surface)" : "transparent",
                  borderBottom: "var(--border-thin) solid var(--border-default)",
                }}
                whileHover={{ background: "var(--bg-surface)", color: "var(--text-primary)" }}
                onClick={() => { onChange(pack.id); setOpen(false); }}
              >
                <span className="w-3.5 h-3.5 rounded-full shrink-0" style={{ background: pack.color, border: "1px solid rgba(255,255,255,0.15)", boxShadow: `0 0 6px ${pack.color}40` }} />
                <div className="flex-1 min-w-0">
                  <div className="font-mono text-[11px] font-semibold">{pack.name}</div>
                  <div className="font-mono text-[9px]" style={{ color: "var(--text-muted)" }}>{pack.desc}</div>
                </div>
                {pack.id === value && <span style={{ color: "var(--accent-primary)", fontSize: "12px" }}>✓</span>}
              </motion.button>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
