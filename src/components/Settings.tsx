import { useState, useEffect } from "react";
import { useRecordingStore, AppConfig } from "../stores/recording";

export function Settings({ onBack }: { onBack: () => void }) {
  const { config, saveConfig, loadAudioDevices, audioDevices } =
    useRecordingStore();
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
    setTimeout(() => setSaved(false), 2000);
  };

  if (!local)
    return (
      <div className="p-6 text-[var(--text-secondary)]">Loading config...</div>
    );

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setLocal((prev) => (prev ? { ...prev, [key]: value } : prev));

  const Section = ({
    title,
    children,
  }: {
    title: string;
    children: React.ReactNode;
  }) => (
    <div className="mb-6">
      <h3 className="text-sm font-semibold text-[var(--text-secondary)] uppercase tracking-wider mb-3">
        {title}
      </h3>
      <div className="space-y-3">{children}</div>
    </div>
  );

  const Field = ({
    label,
    children,
  }: {
    label: string;
    children: React.ReactNode;
  }) => (
    <div className="flex items-center justify-between">
      <label className="text-sm text-[var(--text-primary)]">{label}</label>
      <div className="w-48">{children}</div>
    </div>
  );

  const Select = ({
    value,
    onChange,
    options,
  }: {
    value: string | number;
    onChange: (v: string) => void;
    options: { label: string; value: string | number }[];
  }) => (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2 py-1.5 text-sm bg-[var(--bg-card)] border border-[var(--border)] rounded-md text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent-blue)]"
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );

  const Input = ({
    value,
    onChange,
    type = "text",
  }: {
    value: string | number;
    onChange: (v: string) => void;
    type?: string;
  }) => (
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2 py-1.5 text-sm bg-[var(--bg-card)] border border-[var(--border)] rounded-md text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent-blue)]"
    />
  );

  const Toggle = ({
    checked,
    onChange,
  }: {
    checked: boolean;
    onChange: (v: boolean) => void;
  }) => (
    <button
      onClick={() => onChange(!checked)}
      className={`
        w-10 h-5 rounded-full transition-colors duration-200 relative
        ${checked ? "bg-[var(--accent-green)]" : "bg-[var(--border)]"}
      `}
    >
      <div
        className={`
          w-4 h-4 rounded-full bg-white absolute top-0.5 transition-transform duration-200
          ${checked ? "translate-x-5" : "translate-x-0.5"}
        `}
      />
    </button>
  );

  const resolutions = [
    { label: "480p (854x480)", value: "854x480" },
    { label: "720p (1280x720)", value: "1280x720" },
    { label: "1080p (1920x1080)", value: "1920x1080" },
  ];

  const fpsOptions = [
    { label: "24 fps", value: 24 },
    { label: "30 fps", value: 30 },
    { label: "60 fps", value: 60 },
  ];

  const sampleRates = [
    { label: "22050 Hz", value: 22050 },
    { label: "44100 Hz", value: 44100 },
    { label: "48000 Hz", value: 48000 },
  ];

  const currentRes = `${local.resolution_width}x${local.resolution_height}`;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 px-6 py-4 border-b border-[var(--border)]">
        <button
          onClick={onBack}
          className="text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        >
          ← Back
        </button>
        <span className="text-lg font-semibold">Settings</span>
      </div>

      {/* Settings Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <Section title="Video">
          <Field label="Resolution">
            <Select
              value={currentRes}
              onChange={(v) => {
                const [w, h] = v.split("x").map(Number);
                update("resolution_width", w);
                update("resolution_height", h);
              }}
              options={resolutions}
            />
          </Field>
          <Field label="Frame Rate">
            <Select
              value={local.fps}
              onChange={(v) => update("fps", Number(v))}
              options={fpsOptions}
            />
          </Field>
          <Field label="Recording Mode">
            <Select
              value={local.recording_mode}
              onChange={(v) =>
                update("recording_mode", v as AppConfig["recording_mode"])
              }
              options={[
                { label: "Full Screen", value: "FullScreen" },
                { label: "Region Select", value: "Region" },
                { label: "Window", value: "Window" },
              ]}
            />
          </Field>
        </Section>

        <Section title="Audio">
          <Field label="Enabled">
            <Toggle
              checked={local.audio_enabled}
              onChange={(v) => update("audio_enabled", v)}
            />
          </Field>
          <Field label="Sample Rate">
            <Select
              value={local.audio_sample_rate}
              onChange={(v) => update("audio_sample_rate", Number(v))}
              options={sampleRates}
            />
          </Field>
          <Field label="Device">
            <Select
              value={local.audio_device}
              onChange={(v) => update("audio_device", v)}
              options={[
                { label: "Default", value: "default" },
                ...audioDevices.map((d) => ({ label: d, value: d })),
              ]}
            />
          </Field>
        </Section>

        <Section title="Auto-Zoom">
          <Field label="Enabled">
            <Toggle
              checked={local.auto_zoom_enabled}
              onChange={(v) => update("auto_zoom_enabled", v)}
            />
          </Field>
          <Field label="Zoom Level">
            <Select
              value={local.zoom_level}
              onChange={(v) => update("zoom_level", Number(v))}
              options={[
                { label: "1.5x", value: 1.5 },
                { label: "2x", value: 2 },
                { label: "2.5x", value: 2.5 },
                { label: "3x", value: 3 },
              ]}
            />
          </Field>
          <Field label="Dwell Time">
            <Select
              value={local.zoom_dwell_ms}
              onChange={(v) => update("zoom_dwell_ms", Number(v))}
              options={[
                { label: "1s", value: 1000 },
                { label: "1.5s", value: 1500 },
                { label: "2s", value: 2000 },
                { label: "3s", value: 3000 },
              ]}
            />
          </Field>
        </Section>

        <Section title="Cursor Effects">
          <Field label="Trail Enabled">
            <Toggle
              checked={local.cursor_trail_enabled}
              onChange={(v) => update("cursor_trail_enabled", v)}
            />
          </Field>
          <Field label="Trail Color">
            <Input
              value={local.cursor_trail_color}
              onChange={(v) => update("cursor_trail_color", v)}
            />
          </Field>
          <Field label="Cursor Size">
            <Select
              value={local.cursor_size_multiplier}
              onChange={(v) => update("cursor_size_multiplier", Number(v))}
              options={[
                { label: "1x (normal)", value: 1 },
                { label: "1.5x", value: 1.5 },
                { label: "2x", value: 2 },
                { label: "3x", value: 3 },
              ]}
            />
          </Field>
          <Field label="Smoothing">
            <Toggle
              checked={local.cursor_smoothing}
              onChange={(v) => update("cursor_smoothing", v)}
            />
          </Field>
        </Section>

        <Section title="Hotkeys">
          <Field label="Start">
            <Input
              value={local.hotkey_start}
              onChange={(v) => update("hotkey_start", v)}
            />
          </Field>
          <Field label="Stop">
            <Input
              value={local.hotkey_stop}
              onChange={(v) => update("hotkey_stop", v)}
            />
          </Field>
          <Field label="Pause">
            <Input
              value={local.hotkey_pause}
              onChange={(v) => update("hotkey_pause", v)}
            />
          </Field>
        </Section>

        <Section title="Webcam">
          <Field label="Enabled">
            <Toggle
              checked={local.webcam_enabled}
              onChange={(v) => update("webcam_enabled", v)}
            />
          </Field>
          <Field label="Position">
            <Select
              value={local.webcam_position}
              onChange={(v) =>
                update("webcam_position", v as AppConfig["webcam_position"])
              }
              options={[
                { label: "Top Left", value: "TopLeft" },
                { label: "Top Right", value: "TopRight" },
                { label: "Bottom Left", value: "BottomLeft" },
                { label: "Bottom Right", value: "BottomRight" },
              ]}
            />
          </Field>
          <Field label="Size (px)">
            <Select
              value={local.webcam_size}
              onChange={(v) => update("webcam_size", Number(v))}
              options={[
                { label: "150px", value: 150 },
                { label: "200px", value: 200 },
                { label: "250px", value: 250 },
                { label: "300px", value: 300 },
              ]}
            />
          </Field>
        </Section>

        <Section title="General">
          <Field label="Output Directory">
            <Input
              value={local.output_dir}
              onChange={(v) => update("output_dir", v)}
            />
          </Field>
          <Field label="Minimize to Tray">
            <Toggle
              checked={local.minimize_to_tray}
              onChange={(v) => update("minimize_to_tray", v)}
            />
          </Field>
          <Field label="Copy Path on Save">
            <Toggle
              checked={local.copy_path_on_save}
              onChange={(v) => update("copy_path_on_save", v)}
            />
          </Field>
        </Section>
      </div>

      {/* Save Button */}
      <div className="px-6 py-4 border-t border-[var(--border)]">
        <button
          onClick={handleSave}
          className={`
            w-full py-2 rounded-md text-sm font-semibold transition-colors
            ${
              saved
                ? "bg-[var(--accent-green)] text-black"
                : "bg-[var(--accent-blue)] text-white hover:bg-blue-500"
            }
          `}
        >
          {saved ? "Saved!" : "Save Settings"}
        </button>
      </div>
    </div>
  );
}
