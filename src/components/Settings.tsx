import { useState, useEffect } from "react";
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
        className="flex items-center justify-between px-6 py-3"
        style={{ borderBottom: "var(--border-width) solid var(--border-default)" }}
      >
        <motion.button
          onClick={onBack}
          className="flex items-center gap-2 font-mono text-xs cursor-pointer"
          style={{ color: "var(--text-secondary)", letterSpacing: "0.03em" }}
          whileHover={{ x: -3, color: "var(--text-primary)" }}
          whileTap={{ scale: 0.95 }}
        >
          ← Back
        </motion.button>
        <span className="font-mono text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
          Settings
        </span>
        <motion.button
          onClick={toggleTheme}
          className="font-mono text-xs px-2 py-1 cursor-pointer"
          style={{ border: "var(--border-thin) solid var(--border-default)", color: "var(--text-muted)" }}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
        >
          {theme === "dark" ? "☀ Light" : "● Dark"}
        </motion.button>
      </header>

      {/* ═══ CONTENT ═══ */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="max-w-xl mx-auto space-y-4">

          {/* ── Video Section ── */}
          <Card title="Video" icon="🎬" index={0}>
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
                  { label: "Specific Window", value: "Window" },
                ]}
              />
            </Row>
          </Card>

          {/* ── Audio Section ── */}
          <Card title="Audio" icon="🎤" index={1}>
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
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Auto-Zoom Section ── */}
          <Card title="Auto-Zoom" icon="🔍" index={2} badge="Post-processing">
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
          <Card title="Cursor Effects" icon="✨" index={3} badge="Post-processing">
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
          <Card title="Webcam" icon="📷" index={4}>
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
          <Card title="Hotkeys" icon="⌨️" index={5}>
            <Row label="Start Recording" desc="Keyboard shortcut to begin">
              <Input value={local.hotkey_start} onChange={(v) => update("hotkey_start", v)} />
            </Row>
            <Row label="Stop Recording" desc="Keyboard shortcut to stop">
              <Input value={local.hotkey_stop} onChange={(v) => update("hotkey_stop", v)} />
            </Row>
            <Row label="Pause / Resume" desc="Keyboard shortcut to pause">
              <Input value={local.hotkey_pause} onChange={(v) => update("hotkey_pause", v)} />
            </Row>
          </Card>

          {/* ── General ── */}
          <Card title="General" icon="⚙️" index={6}>
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
      <div className="px-6 py-4" style={{ borderTop: "var(--border-width) solid var(--border-default)" }}>
        <motion.button
          onClick={handleSave}
          className="w-full py-3 font-mono text-sm font-semibold cursor-pointer"
          style={{
            border: `var(--border-width) solid ${saved ? "var(--accent-success)" : "var(--accent-primary)"}`,
            background: saved ? "var(--accent-success)" : "var(--accent-primary)",
            color: "var(--text-primary)",
            letterSpacing: "0.03em",
          }}
          whileHover={{ scale: 1.01, y: -1 }}
          whileTap={{ scale: 0.98 }}
        >
          {saved ? "✓ Saved" : "Save Settings"}
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
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-surface)",
      }}
    >
      {/* Card Header */}
      <div
        className="flex items-center justify-between px-4 py-3"
        style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}
      >
        <div className="flex items-center gap-2">
          <span className="text-base">{icon}</span>
          <span className="font-mono text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
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
              letterSpacing: "0.05em",
            }}
          >
            {badge}
          </span>
        )}
      </div>

      {/* Card Body */}
      <div className="px-4 py-3 space-y-4">
        {children}
      </div>
    </motion.div>
  );
}

function Row({ label, desc, children }: { label: string; desc?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <div className="font-mono text-sm" style={{ color: "var(--text-primary)" }}>
          {label}
        </div>
        {desc && (
          <div className="font-mono mt-0.5" style={{ color: "var(--text-muted)", fontSize: "0.6rem", lineHeight: 1.4 }}>
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
      className="px-3 py-1.5 font-mono text-xs cursor-pointer min-w-[160px]"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-base)",
        color: "var(--text-primary)",
      }}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  );
}

function Segmented<T extends string | number>({ value, options, onChange }: {
  value: T; options: { label: string; value: T }[]; onChange: (v: T) => void;
}) {
  return (
    <div className="flex" style={{ border: "var(--border-width) solid var(--border-default)" }}>
      {options.map((opt, i) => (
        <motion.button
          key={String(opt.value)}
          onClick={() => onChange(opt.value)}
          className="px-3 py-1.5 font-mono text-xs cursor-pointer"
          style={{
            background: opt.value === value ? "var(--accent-primary)" : "var(--bg-base)",
            color: opt.value === value ? "var(--text-primary)" : "var(--text-secondary)",
            borderRight: i < options.length - 1 ? "var(--border-thin) solid var(--border-default)" : "none",
            letterSpacing: "0.03em",
          }}
          whileHover={opt.value !== value ? { background: "var(--bg-elevated)" } : {}}
          whileTap={{ scale: 0.95 }}
        >
          {opt.label}
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
      className="px-3 py-1.5 font-mono text-xs min-w-[160px]"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-base)",
        color: "var(--text-primary)",
      }}
    />
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
      }}
      whileTap={{ scale: 0.95 }}
    >
      <motion.div
        className="absolute top-0.5"
        style={{ width: 18, height: 18, background: "var(--text-primary)" }}
        animate={{ left: checked ? 22 : 2 }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}
