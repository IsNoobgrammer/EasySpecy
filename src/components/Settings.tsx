import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore, AppConfig } from "../stores/recording";
import { useThemeStore } from "../lib/theme";

export function Settings({ onBack }: { onBack: () => void }) {
  const { config, saveConfig, loadAudioDevices, audioDevices } = useStore();
  const { theme, toggleTheme } = useThemeStore();
  const [local, setLocal] = useState<AppConfig | null>(null);
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(["video", "audio"])
  );

  useEffect(() => {
    if (config) setLocal({ ...config });
    loadAudioDevices();
  }, [config]);

  const handleSave = async () => {
    if (!local) return;
    await saveConfig(local);
    onBack();
  };

  if (!local) {
    return (
      <div className="flex items-center justify-center h-full" style={{ background: "var(--bg-base)" }}>
        <motion.div
          className="font-mono text-sm uppercase"
          style={{ color: "var(--text-muted)", letterSpacing: "0.1em" }}
          animate={{ opacity: [0.4, 1, 0.4] }}
          transition={{ duration: 1.5, repeat: Infinity }}
        >
          "Loading settings..."
        </motion.div>
      </div>
    );
  }

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setLocal((prev) => (prev ? { ...prev, [key]: value } : prev));

  const toggleSection = (id: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="flex flex-col h-full" style={{ background: "var(--bg-base)" }}>
      {/* Header */}
      <header
        className="flex items-center justify-between px-6 py-3"
        style={{ borderBottom: "var(--border-width) solid var(--border-default)" }}
      >
        <div className="flex items-center gap-3">
          <motion.button
            onClick={onBack}
            className="flex items-center gap-2 font-mono text-xs uppercase cursor-pointer"
            style={{ color: "var(--text-secondary)", letterSpacing: "0.05em" }}
            whileHover={{ x: -3, color: "var(--text-primary)" }}
            whileTap={{ scale: 0.95 }}
          >
            ← BACK
          </motion.button>
        </div>
        <span className="font-mono text-sm font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>
          "Configuration"
        </span>
        <motion.button
          onClick={toggleTheme}
          className="font-mono text-xs px-2 py-1 cursor-pointer"
          style={{
            border: "var(--border-thin) solid var(--border-default)",
            color: "var(--text-muted)",
          }}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
        >
          {theme === "dark" ? "☀ LIGHT" : "● DARK"}
        </motion.button>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-3">
        {/* Video */}
        <Section
          id="video"
          title="Video"
          expanded={expandedSections.has("video")}
          onToggle={() => toggleSection("video")}
          index={0}
        >
          <Field label="Resolution">
            <Select
              value={`${local.resolution_width}x${local.resolution_height}`}
              onChange={(v) => { const [w, h] = v.split("x").map(Number); update("resolution_width", w); update("resolution_height", h); }}
              options={[
                { label: "480P (854×480)", value: "854x480" },
                { label: "720P (1280×720)", value: "1280x720" },
                { label: "1080P (1920×1080)", value: "1920x1080" },
              ]}
            />
          </Field>
          <Field label="Frame Rate">
            <Select value={local.fps} onChange={(v) => update("fps", Number(v))}
              options={[{ label: "24 FPS", value: 24 }, { label: "30 FPS", value: 30 }, { label: "60 FPS", value: 60 }]} />
          </Field>
          <Field label="Mode">
            <Select value={local.recording_mode} onChange={(v) => update("recording_mode", v as AppConfig["recording_mode"])}
              options={[{ label: "FULLSCREEN", value: "FullScreen" }, { label: "REGION", value: "Region" }, { label: "WINDOW", value: "Window" }]} />
          </Field>
        </Section>

        {/* Audio */}
        <Section
          id="audio"
          title="Audio"
          expanded={expandedSections.has("audio")}
          onToggle={() => toggleSection("audio")}
          index={1}
        >
          <Field label="Enabled">
            <Toggle checked={local.audio_enabled} onChange={(v) => update("audio_enabled", v)} />
          </Field>
          {local.audio_enabled && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              className="space-y-3 overflow-hidden"
            >
              <Field label="Source">
                <Select value={local.audio_source} onChange={(v) => update("audio_source", v as AppConfig["audio_source"])}
                  options={[{ label: "Mic", value: "Mic" }, { label: "System", value: "System" }, { label: "Both", value: "Both" }]} />
              </Field>
              <Field label="Sample Rate">
                <Select value={local.audio_sample_rate} onChange={(v) => update("audio_sample_rate", Number(v))}
                  options={[{ label: "22050 HZ", value: 22050 }, { label: "44100 HZ", value: 44100 }, { label: "48000 HZ", value: 48000 }]} />
              </Field>
              <Field label="Device">
                <Select value={local.audio_device} onChange={(v) => update("audio_device", v)}
                  options={[{ label: "DEFAULT", value: "default" }, ...audioDevices.map((d) => ({ label: d.toUpperCase(), value: d }))]} />
              </Field>
            </motion.div>
          )}
        </Section>

        {/* Auto-Zoom */}
        <Section
          id="zoom"
          title="Auto-Zoom"
          expanded={expandedSections.has("zoom")}
          onToggle={() => toggleSection("zoom")}
          index={2}
        >
          <Field label="Enabled">
            <Toggle checked={local.auto_zoom_enabled} onChange={(v) => update("auto_zoom_enabled", v)} />
          </Field>
          {local.auto_zoom_enabled && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              className="space-y-3 overflow-hidden"
            >
              <Field label="Zoom Level">
                <Select value={local.zoom_level} onChange={(v) => update("zoom_level", Number(v))}
                  options={[{ label: "1.5×", value: 1.5 }, { label: "2×", value: 2 }, { label: "2.5×", value: 2.5 }, { label: "3×", value: 3 }]} />
              </Field>
              <Field label="Dwell Time">
                <Select value={local.zoom_dwell_ms} onChange={(v) => update("zoom_dwell_ms", Number(v))}
                  options={[{ label: "1S", value: 1000 }, { label: "1.5S", value: 1500 }, { label: "2S", value: 2000 }, { label: "3S", value: 3000 }]} />
              </Field>
              <div className="font-mono text-xs px-1" style={{ color: "var(--text-muted)", fontSize: "0.6rem" }}>
                ZOOMS TOWARD CLICK POSITIONS DURING RECORDING
              </div>
            </motion.div>
          )}
        </Section>

        {/* Cursor Effects */}
        <Section
          id="cursor"
          title="Cursor Effects"
          expanded={expandedSections.has("cursor")}
          onToggle={() => toggleSection("cursor")}
          index={3}
        >
          <Field label="Trail">
            <Toggle checked={local.cursor_trail_enabled} onChange={(v) => update("cursor_trail_enabled", v)} />
          </Field>
          {local.cursor_trail_enabled && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              className="space-y-3 overflow-hidden"
            >
              <Field label="Color">
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
              </Field>
              <Field label="Size">
                <Select value={local.cursor_size_multiplier} onChange={(v) => update("cursor_size_multiplier", Number(v))}
                  options={[{ label: "1× NORMAL", value: 1 }, { label: "1.5×", value: 1.5 }, { label: "2×", value: 2 }, { label: "3×", value: 3 }]} />
              </Field>
              <Field label="Smoothing">
                <Toggle checked={local.cursor_smoothing} onChange={(v) => update("cursor_smoothing", v)} />
              </Field>
            </motion.div>
          )}
        </Section>

        {/* Webcam */}
        <Section
          id="webcam"
          title="Webcam"
          expanded={expandedSections.has("webcam")}
          onToggle={() => toggleSection("webcam")}
          index={4}
        >
          <Field label="Enabled">
            <Toggle checked={local.webcam_enabled} onChange={(v) => update("webcam_enabled", v)} />
          </Field>
          {local.webcam_enabled && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              className="space-y-3 overflow-hidden"
            >
              <Field label="Position">
                <Select value={local.webcam_position} onChange={(v) => update("webcam_position", v as AppConfig["webcam_position"])}
                  options={[{ label: "TOP LEFT", value: "TopLeft" }, { label: "TOP RIGHT", value: "TopRight" }, { label: "BOTTOM LEFT", value: "BottomLeft" }, { label: "BOTTOM RIGHT", value: "BottomRight" }]} />
              </Field>
              <Field label="Size">
                <Select value={local.webcam_size} onChange={(v) => update("webcam_size", Number(v))}
                  options={[{ label: "150PX", value: 150 }, { label: "200PX", value: 200 }, { label: "250PX", value: 250 }, { label: "300PX", value: 300 }]} />
              </Field>
            </motion.div>
          )}
        </Section>

        {/* Hotkeys */}
        <Section
          id="hotkeys"
          title="Hotkeys"
          expanded={expandedSections.has("hotkeys")}
          onToggle={() => toggleSection("hotkeys")}
          index={5}
        >
          <Field label="Start"><Input value={local.hotkey_start} onChange={(v) => update("hotkey_start", v)} /></Field>
          <Field label="Stop"><Input value={local.hotkey_stop} onChange={(v) => update("hotkey_stop", v)} /></Field>
          <Field label="Pause"><Input value={local.hotkey_pause} onChange={(v) => update("hotkey_pause", v)} /></Field>
        </Section>

        {/* General */}
        <Section
          id="general"
          title="General"
          expanded={expandedSections.has("general")}
          onToggle={() => toggleSection("general")}
          index={6}
        >
          <Field label="Output Directory"><Input value={local.output_dir} onChange={(v) => update("output_dir", v)} /></Field>
          <Field label="Minimize to Tray"><Toggle checked={local.minimize_to_tray} onChange={(v) => update("minimize_to_tray", v)} /></Field>
          <Field label="Copy Path on Save"><Toggle checked={local.copy_path_on_save} onChange={(v) => update("copy_path_on_save", v)} /></Field>
        </Section>
      </div>

      {/* Save Button */}
      <div className="px-6 py-4" style={{ borderTop: "var(--border-width) solid var(--border-default)" }}>
        <motion.button
          onClick={handleSave}
          className="w-full py-3 font-mono text-sm font-bold uppercase cursor-pointer"
          style={{
            border: "var(--border-width) solid var(--accent-primary)",
            background: "var(--accent-primary)",
            color: "white",
            letterSpacing: "0.08em",
          }}
          whileHover={{ scale: 1.01, y: -1 }}
          whileTap={{ scale: 0.98 }}
        >
          "Save Settings"
        </motion.button>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════
   PRIMITIVES
   ═══════════════════════════════════════════════════════ */

function Section({
  title,
  expanded,
  onToggle,
  children,
  index,
}: {
  id: string;
  title: string;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
  index: number;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.04, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-surface)",
      }}
    >
      <motion.button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 cursor-pointer"
        whileHover={{ backgroundColor: "var(--bg-elevated)" }}
      >
        <span
          className="font-mono text-xs font-bold uppercase"
          style={{ color: "var(--text-primary)", letterSpacing: "0.1em" }}
        >
          {title}
        </span>
        <motion.span
          animate={{ rotate: expanded ? 90 : 0 }}
          transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
          className="font-mono text-xs"
          style={{ color: "var(--text-muted)" }}
        >
          →
        </motion.span>
      </motion.button>
      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
            className="overflow-hidden"
          >
            <div
              className="px-4 pb-4 space-y-3"
              style={{ borderTop: "var(--border-thin) solid var(--border-default)" }}
            >
              <div className="pt-3 space-y-3">{children}</div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <label
        className="font-mono text-xs uppercase shrink-0"
        style={{ color: "var(--text-secondary)", letterSpacing: "0.05em", fontSize: "0.65rem" }}
      >
        {label}
      </label>
      <div className="w-48">{children}</div>
    </div>
  );
}

function Select({ value, onChange, options }: { value: string | number; onChange: (v: string) => void; options: { label: string; value: string | number }[] }) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2.5 py-1.5 font-mono text-xs cursor-pointer"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-base)",
        color: "var(--text-primary)",
        letterSpacing: "0.02em",
      }}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  );
}

function Input({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2.5 py-1.5 font-mono text-xs"
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
        style={{
          width: 18,
          height: 18,
          background: "white",
        }}
        animate={{ left: checked ? 22 : 2 }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}
