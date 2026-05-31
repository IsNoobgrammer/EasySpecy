import { useState, useEffect } from "react";
import { useStore, AppConfig } from "../stores/recording";

export function Settings({ onBack }: { onBack: () => void }) {
  const { config, saveConfig, loadAudioDevices, audioDevices } = useStore();
  const [local, setLocal] = useState<AppConfig | null>(null);

  useEffect(() => {
    if (config) setLocal({ ...config });
    loadAudioDevices();
  }, [config]);

  const handleSave = async () => {
    if (!local) return;
    await saveConfig(local);
    onBack();
  };

  if (!local) return <div className="flex items-center justify-center h-full bg-[#0d1117]"><div className="text-[#8b949e] text-sm">Loading...</div></div>;

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setLocal((prev) => (prev ? { ...prev, [key]: value } : prev));

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* Header */}
      <div className="flex items-center gap-3 px-6 py-3 border-b border-[#21262d]">
        <button onClick={onBack} className="flex items-center gap-1 text-[#8b949e] hover:text-[#e6edf3] text-sm transition-colors">
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" /></svg>
          Back
        </button>
        <span className="text-base font-semibold text-[#e6edf3]">Settings</span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-5">
        <Section title="Video" icon="🎬">
          <Field label="Resolution">
            <Select value={`${local.resolution_width}x${local.resolution_height}`}
              onChange={(v) => { const [w, h] = v.split("x").map(Number); update("resolution_width", w); update("resolution_height", h); }}
              options={[{ label: "480p (854×480)", value: "854x480" }, { label: "720p (1280×720)", value: "1280x720" }, { label: "1080p (1920×1080)", value: "1920x1080" }]} />
          </Field>
          <Field label="Frame Rate">
            <Select value={local.fps} onChange={(v) => update("fps", Number(v))}
              options={[{ label: "24 fps", value: 24 }, { label: "30 fps", value: 30 }, { label: "60 fps", value: 60 }]} />
          </Field>
          <Field label="Mode">
            <Select value={local.recording_mode} onChange={(v) => update("recording_mode", v as AppConfig["recording_mode"])}
              options={[{ label: "Full Screen", value: "FullScreen" }, { label: "Region Select", value: "Region" }, { label: "Window", value: "Window" }]} />
          </Field>
        </Section>

        <Section title="Audio" icon="🎤">
          <Field label="Enabled">
            <Toggle checked={local.audio_enabled} onChange={(v) => update("audio_enabled", v)} />
          </Field>
          {local.audio_enabled && (
            <>
              <Field label="Source">
                <Select value={local.audio_source} onChange={(v) => update("audio_source", v as AppConfig["audio_source"])}
                  options={[{ label: "Microphone only", value: "Mic" }, { label: "System audio only", value: "System" }, { label: "Both (mic + system)", value: "Both" }]} />
              </Field>
              <Field label="Sample Rate">
                <Select value={local.audio_sample_rate} onChange={(v) => update("audio_sample_rate", Number(v))}
                  options={[{ label: "22050 Hz", value: 22050 }, { label: "44100 Hz", value: 44100 }, { label: "48000 Hz", value: 48000 }]} />
              </Field>
              <Field label="Mic Device">
                <Select value={local.audio_device} onChange={(v) => update("audio_device", v)}
                  options={[{ label: "Default", value: "default" }, ...audioDevices.map((d) => ({ label: d, value: d }))]} />
              </Field>
            </>
          )}
        </Section>

        <Section title="Auto-Zoom" icon="🔍">
          <Field label="Enabled">
            <Toggle checked={local.auto_zoom_enabled} onChange={(v) => update("auto_zoom_enabled", v)} />
          </Field>
          {local.auto_zoom_enabled && (
            <>
              <Field label="Zoom Level">
                <Select value={local.zoom_level} onChange={(v) => update("zoom_level", Number(v))}
                  options={[{ label: "1.5×", value: 1.5 }, { label: "2×", value: 2 }, { label: "2.5×", value: 2.5 }, { label: "3×", value: 3 }]} />
              </Field>
              <Field label="Dwell Time">
                <Select value={local.zoom_dwell_ms} onChange={(v) => update("zoom_dwell_ms", Number(v))}
                  options={[{ label: "1s", value: 1000 }, { label: "1.5s", value: 1500 }, { label: "2s", value: 2000 }, { label: "3s", value: 3000 }]} />
              </Field>
              <div className="text-xs text-[#484f58] px-1">Zooms toward click positions during recording</div>
            </>
          )}
        </Section>

        <Section title="Cursor Effects" icon="✨">
          <Field label="Trail Enabled">
            <Toggle checked={local.cursor_trail_enabled} onChange={(v) => update("cursor_trail_enabled", v)} />
          </Field>
          {local.cursor_trail_enabled && (
            <>
              <Field label="Trail Color">
                <div className="flex items-center gap-2">
                  <input type="color" value={local.cursor_trail_color} onChange={(e) => update("cursor_trail_color", e.target.value)}
                    className="w-8 h-8 rounded border border-[#30363d] bg-transparent cursor-pointer" />
                  <span className="text-xs text-[#8b949e] font-mono">{local.cursor_trail_color}</span>
                </div>
              </Field>
              <Field label="Cursor Size">
                <Select value={local.cursor_size_multiplier} onChange={(v) => update("cursor_size_multiplier", Number(v))}
                  options={[{ label: "1× (normal)", value: 1 }, { label: "1.5×", value: 1.5 }, { label: "2×", value: 2 }, { label: "3×", value: 3 }]} />
              </Field>
              <Field label="Smoothing">
                <Toggle checked={local.cursor_smoothing} onChange={(v) => update("cursor_smoothing", v)} />
              </Field>
            </>
          )}
        </Section>

        <Section title="Webcam" icon="📷">
          <Field label="Enabled">
            <Toggle checked={local.webcam_enabled} onChange={(v) => update("webcam_enabled", v)} />
          </Field>
          {local.webcam_enabled && (
            <>
              <Field label="Position">
                <Select value={local.webcam_position} onChange={(v) => update("webcam_position", v as AppConfig["webcam_position"])}
                  options={[{ label: "Top Left", value: "TopLeft" }, { label: "Top Right", value: "TopRight" }, { label: "Bottom Left", value: "BottomLeft" }, { label: "Bottom Right", value: "BottomRight" }]} />
              </Field>
              <Field label="Size">
                <Select value={local.webcam_size} onChange={(v) => update("webcam_size", Number(v))}
                  options={[{ label: "150px", value: 150 }, { label: "200px", value: 200 }, { label: "250px", value: 250 }, { label: "300px", value: 300 }]} />
              </Field>
            </>
          )}
        </Section>

        <Section title="Hotkeys" icon="⌨️">
          <Field label="Start"><Input value={local.hotkey_start} onChange={(v) => update("hotkey_start", v)} /></Field>
          <Field label="Stop"><Input value={local.hotkey_stop} onChange={(v) => update("hotkey_stop", v)} /></Field>
          <Field label="Pause"><Input value={local.hotkey_pause} onChange={(v) => update("hotkey_pause", v)} /></Field>
        </Section>

        <Section title="General" icon="⚙️">
          <Field label="Output Dir"><Input value={local.output_dir} onChange={(v) => update("output_dir", v)} /></Field>
          <Field label="Minimize to Tray"><Toggle checked={local.minimize_to_tray} onChange={(v) => update("minimize_to_tray", v)} /></Field>
          <Field label="Copy Path on Save"><Toggle checked={local.copy_path_on_save} onChange={(v) => update("copy_path_on_save", v)} /></Field>
        </Section>
      </div>

      {/* Save */}
      <div className="px-6 py-4 border-t border-[#21262d]">
        <button onClick={handleSave} className="w-full py-2.5 rounded-md text-sm font-semibold bg-[#238636] hover:bg-[#2ea043] text-white transition-colors">
          Save Settings
        </button>
      </div>
    </div>
  );
}

function Section({ title, icon, children }: { title: string; icon: string; children: React.ReactNode }) {
  return (
    <div className="bg-[#161b22] border border-[#21262d] rounded-lg p-4">
      <h3 className="text-sm font-semibold text-[#e6edf3] mb-3 flex items-center gap-2"><span>{icon}</span> {title}</h3>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between">
      <label className="text-sm text-[#8b949e]">{label}</label>
      <div className="w-48">{children}</div>
    </div>
  );
}

function Select({ value, onChange, options }: { value: string | number; onChange: (v: string) => void; options: { label: string; value: string | number }[] }) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value)}
      className="w-full px-2.5 py-1.5 text-sm bg-[#0d1117] border border-[#30363d] rounded-md text-[#e6edf3] focus:outline-none focus:border-[#58a6ff] transition-colors">
      {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );
}

function Input({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input type="text" value={value} onChange={(e) => onChange(e.target.value)}
      className="w-full px-2.5 py-1.5 text-sm bg-[#0d1117] border border-[#30363d] rounded-md text-[#e6edf3] focus:outline-none focus:border-[#58a6ff] transition-colors font-mono" />
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button onClick={() => onChange(!checked)}
      className={`w-10 h-5 rounded-full transition-colors duration-200 relative cursor-pointer ${checked ? "bg-[#238636]" : "bg-[#30363d]"}`}>
      <div className={`w-4 h-4 rounded-full bg-white absolute top-0.5 transition-transform duration-200 shadow-sm ${checked ? "translate-x-5" : "translate-x-0.5"}`} />
    </button>
  );
}
