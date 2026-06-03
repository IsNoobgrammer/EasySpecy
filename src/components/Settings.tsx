import { Icon } from "./Icon";
import { useState, useEffect, useId, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore, AppConfig } from "../stores/recording";
import { useThemeStore } from "../lib/theme";
import { WebcamPreview } from "./WebcamPreview";
import { KeyboardPreview } from "./KeyboardPreview";
import { invoke } from "@tauri-apps/api/core";
import {
  TrailRenderer, ClickEffectRenderer, drawPreviewBackground,
  type TrailStyle, type ClickEffect,
} from "../lib/effects";

interface CursorPackInfo {
  id: string;
  name: string;
  description: string;
  author: string;
  is_builtin: boolean;
}

const TRAIL_STYLES: { id: TrailStyle; label: string; desc: string }[] = [
  { id: "glow", label: "Glow", desc: "Soft luminous bloom that reacts to speed" },
  { id: "particles", label: "Spark", desc: "Embers and fireflies that orbit the cursor" },
  { id: "ribbon", label: "Ribbon", desc: "Multi-layered flowing band with shimmer" },
  { id: "dots", label: "Dots", desc: "Connected halos with white-hot cores" },
  { id: "aurora", label: "Aurora", desc: "Rainbow wave with flowing light layers" },
  { id: "none", label: "Off", desc: "No trail effect" },
];

const CLICK_EFFECTS: { id: ClickEffect; label: string; desc: string }[] = [
  { id: "ripple", label: "Ripple", desc: "Expanding water rings with flash" },
  { id: "spotlight", label: "Spotlight", desc: "Radial flare with cross rays" },
  { id: "ring", label: "Ring", desc: "Double ring — expand and contract" },
  { id: "pulse", label: "Pulse", desc: "Breathing energy waves" },
  { id: "confetti", label: "Confetti", desc: "Burst of mixed-shape particles" },
  { id: "none", label: "None", desc: "No click effect" },
];

export function Settings({ onBack }: { onBack: () => void }) {
  const {
    config, saveConfig, loadAudioDevices, audioDevices,
    audioLevels, startAudioMonitor, stopAudioMonitor, pollAudioLevels
  } = useStore();
  const { theme, toggleTheme } = useThemeStore();
  const [local, setLocal] = useState<AppConfig | null>(null);
  const [saved, setSaved] = useState(false);
  const [showWebcamPreview, setShowWebcamPreview] = useState(false);
  const [showKeyboardPreview, setShowKeyboardPreview] = useState(false);
  
  // Lists
  const [cursorPacks, setCursorPacks] = useState<CursorPackInfo[]>([]);
  const [previewActive, setPreviewActive] = useState(false);

  // Load configuration and systems
  useEffect(() => {
    if (config) setLocal({ ...config });
    loadAudioDevices();
    
    invoke<CursorPackInfo[]>("get_cursor_packs")
      .then(setCursorPacks)
      .catch((err) => console.error("Failed to load cursor packs:", err));
  }, [config]);

  // Handle active audio monitoring VU meter
  useEffect(() => {
    let active = true;
    if (local?.audio_enabled) {
      startAudioMonitor();
      const interval = setInterval(() => {
        if (active) pollAudioLevels();
      }, 100);
      return () => {
        active = false;
        clearInterval(interval);
        stopAudioMonitor();
      };
    }
  }, [local?.audio_enabled]);

  const handleSave = async () => {
    if (!local) return;
    await saveConfig(local);
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setLocal((prev) => (prev ? { ...prev, [key]: value } : prev));

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

  // Live 30s cursor preview trigger
  const handleCursorPreview = async () => {
    if (previewActive) {
      await invoke("restore_cursors").catch(() => {});
      setPreviewActive(false);
    } else {
      await invoke("apply_cursor_pack", { packId: local.cursor_pack }).catch(() => {});
      setPreviewActive(true);
      setTimeout(async () => {
        await invoke("restore_cursors").catch(() => {});
        setPreviewActive(false);
      }, 30000);
    }
  };

  return (
    <div className="flex flex-col h-full relative overflow-hidden" style={{ background: "var(--bg-base)", color: "var(--text-primary)" }}>
      {/* Noise layer background */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.02'/%3E%3C/svg%3E")`,
          mixBlendMode: "overlay",
          opacity: 0.1,
        }}
      />

      {/* ═══ HEADER ═══ */}
      <header
        className="flex items-center justify-between px-6 py-3 sticky top-0 z-50 backdrop-blur-md border-b"
        style={{ background: "var(--bg-overlay)", borderColor: "var(--border-default)" }}
      >
        <motion.button
          onClick={onBack}
          className="flex items-center gap-2 font-mono text-xs cursor-pointer"
          style={{ color: "var(--text-secondary)" }}
          whileHover={{ x: -3, color: "var(--text-primary)" }}
          whileTap={{ scale: 0.95 }}
        >
          <Icon name="arrow_back" size={16} /> Back to Dashboard
        </motion.button>
        <span className="font-mono text-sm font-semibold uppercase tracking-widest" style={{ color: "var(--text-primary)" }}>
          Settings
        </span>
        <motion.button
          onClick={toggleTheme}
          className="font-mono text-xs px-2.5 py-1 cursor-pointer flex items-center gap-1.5 border rounded"
          style={{ borderColor: "var(--border-default)", color: "var(--text-secondary)" }}
          whileHover={{ scale: 1.05, borderColor: "var(--border-strong)" }}
          whileTap={{ scale: 0.95 }}
        >
          <Icon name={theme === "dark" ? "light_mode" : "dark_mode"} size={14} style={{ color: "var(--accent-primary)" }} />
          {theme === "dark" ? "Light" : "Dark"}
        </motion.button>
      </header>

      {/* ═══ CONTENT ═══ */}
      <div className="flex-1 overflow-y-auto px-6 py-6 scrollbar-thin">
        <div className="max-w-3xl mx-auto space-y-6">

          {/* ── Video Section ── */}
          <Card title="Video" icon="videocam" index={0}>
            <Row label="Resolution" desc="Output video dimensions">
              <Select
                value={`${local.resolution_width}x${local.resolution_height}`}
                onChange={(v) => {
                  const [w, h] = v.split("x").map(Number);
                  update("resolution_width", w);
                  update("resolution_height", h);
                }}
                options={[
                  { label: "480p — 854×480", value: "854x480" },
                  { label: "720p — 1280×720", value: "1280x720" },
                  { label: "1080p — 1920×1080", value: "1920x1080" },
                ]}
              />
            </Row>
            <Row label="Frame Rate" desc="Frames per second of the output video">
              <Segmented
                value={local.fps}
                options={[{ label: "24", value: 24 }, { label: "30", value: 30 }, { label: "60", value: 60 }]}
                onChange={(v) => update("fps", v)}
              />
            </Row>
            <Row label="Capture Mode" desc="Full display recording or region-specific bounds">
              <Select
                value={local.recording_mode}
                onChange={(v) => update("recording_mode", v as AppConfig["recording_mode"])}
                options={[
                  { label: "Full Screen", value: "FullScreen" },
                  { label: "Region Select", value: "Region" },
                ]}
              />
            </Row>
            
            <div className="pt-2 border-t space-y-4" style={{ borderColor: "var(--border-default)" }}>
              <Row label="Video Encoder" desc="Select video codec format (libx264 is default)">
                <Select
                  value={local.video_encoder}
                  onChange={(v) => update("video_encoder", v as any)}
                  options={[
                    { label: "AV1 (SVT-AV1)", value: "AV1" },
                    { label: "AV1 NVENC (RTX 40xx)", value: "AV1_NVENC" },
                    { label: "H.264 CPU", value: "H264" },
                    { label: "H.264 NVENC GPU", value: "H264_NVENC" },
                    { label: "H.265 CPU", value: "H265" },
                    { label: "H.265 NVENC GPU", value: "H265_NVENC" },
                    { label: "VP9 CPU", value: "VP9" },
                  ]}
                />
              </Row>
              <Row label="Encoding Quality" desc="Quality preset level for encoding complexity">
                <Select
                  value={local.video_quality}
                  onChange={(v) => update("video_quality", v as any)}
                  options={[
                    { label: "Low (Speed)", value: "Low" },
                    { label: "Medium (Balanced)", value: "Medium" },
                    { label: "High (HQ)", value: "High" },
                    { label: "Ultra (Lossless)", value: "Ultra" },
                    { label: "Insane (AV1 Max)", value: "Insane" },
                    { label: "Custom Bitrate", value: "Custom" },
                  ]}
                />
              </Row>
              {local.video_quality === "Custom" && (
                <Row label="Custom Bitrate" desc="Specify output target video bitrate">
                  <Slider
                    value={local.video_bitrate_kbps}
                    min={500}
                    max={30000}
                    step={100}
                    onChange={(v) => update("video_bitrate_kbps", v)}
                    suffix=" kbps"
                  />
                </Row>
              )}
            </div>
          </Card>

          {/* ── Audio Section ── */}
          <Card title="Audio" icon="mic" index={1}>
            <Row label="Record Audio" desc="Toggle to capture system or microphone inputs">
              <Toggle checked={local.audio_enabled} onChange={(v) => update("audio_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.audio_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden space-y-4 pt-4 border-t"
                  style={{ borderColor: "var(--border-default)" }}
                >
                  <Row label="Audio Source" desc="Capture microphone, system output, or both">
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
                  <Row label="Sample Rate" desc="Bit rate quality of the captured sound streams">
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
                  <Row label="Microphone Device" desc="Select system microphone input">
                    <Select
                      value={local.audio_device}
                      onChange={(v) => update("audio_device", v)}
                      options={[
                        { label: "System Default", value: "default" },
                        ...audioDevices.map((d) => ({ label: d, value: d })),
                      ]}
                    />
                  </Row>
                  <Row label="Microphone Gain" desc="Volume level multiplier boost (1.0 = standard)">
                    <Slider value={local.mic_gain} min={0} max={3} step={0.1} onChange={(v) => update("mic_gain", v)} suffix="×" />
                  </Row>
                  {(local.audio_source === "Both" || local.audio_source === "System") && (
                    <Row label="System Volume" desc="Gain level multiplier for system sound outputs">
                      <Slider value={local.system_volume} min={0} max={1} step={0.05} onChange={(v) => update("system_volume", v)} suffix="" />
                    </Row>
                  )}
                  
                  {/* Advanced Noise Controls */}
                  <div className="pt-3 border-t space-y-4" style={{ borderColor: "var(--border-default)" }}>
                    <Row label="Noise Gate Threshold" desc="Mutes mic when signal goes below threshold (0 = off)">
                      <Slider value={local.noise_gate_threshold} min={0} max={1} step={0.02} onChange={(v) => update("noise_gate_threshold", v)} suffix="" />
                    </Row>
                    <Row label="Denoising Algorithm" desc="Noise reduction algorithm type">
                      <Select
                        value={local.noise_reduction_mode}
                        onChange={(v) => update("noise_reduction_mode", v as any)}
                        options={[
                          { label: "Off — No processing", value: "Off" },
                          { label: "Gate — Energy gate only", value: "Gate" },
                          { label: "Spectral — Gate + hiss removal", value: "Spectral" },
                          { label: "RNN — Gate + AI denoising (best)", value: "RNN" },
                          { label: "Full — Gate + RNN + spectral", value: "Full" },
                        ]}
                      />
                    </Row>
                    <Row label="Noise Reduction Strength" desc="Level of background hiss suppression (0 = off, 1 = max)">
                      <Slider value={local.noise_reduction} min={0} max={1} step={0.05} onChange={(v) => update("noise_reduction", v)} suffix="" />
                    </Row>
                  </div>

                  {/* Active Loudness Meter Visualization */}
                  <div className="p-3 rounded-lg space-y-3 border" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
                    <div className="space-y-1">
                      <div className="flex justify-between font-mono text-[9px]" style={{ color: "var(--text-secondary)" }}>
                        <span>MIC LEVEL MONITOR</span>
                        <span className={audioLevels.micDb > -12 ? "text-red-400 font-bold" : audioLevels.micDb > -24 ? "text-yellow-400 font-bold" : "text-[var(--accent-primary)] font-bold"}>
                          {audioLevels.micDb > -60 ? `${audioLevels.micDb.toFixed(0)} dB` : "Silent"}
                        </span>
                      </div>
                      <div className="h-2 w-full rounded-full overflow-hidden relative border" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
                        <div
                          className="h-full rounded-full transition-all duration-75"
                          style={{
                            width: `${Math.max(0, Math.min(100, ((audioLevels.micDb + 60) / 60) * 100))}%`,
                            background: "linear-gradient(to right, var(--accent-primary) 65%, #ffd000 85%, #ff4444 100%)",
                          }}
                        />
                      </div>
                    </div>
                    
                    {(local.audio_source === "Both" || local.audio_source === "System") && (
                      <div className="space-y-1">
                        <div className="flex justify-between font-mono text-[9px]" style={{ color: "var(--text-secondary)" }}>
                          <span>SYSTEM LEVEL MONITOR</span>
                          <span className={audioLevels.sysDb > -12 ? "text-red-400 font-bold" : audioLevels.sysDb > -24 ? "text-yellow-400 font-bold" : "text-[var(--accent-info)] font-bold"}>
                            {audioLevels.sysDb > -60 ? `${audioLevels.sysDb.toFixed(0)} dB` : "Silent"}
                          </span>
                        </div>
                        <div className="h-2 w-full rounded-full overflow-hidden relative border" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
                          <div
                            className="h-full rounded-full transition-all duration-75"
                            style={{
                              width: `${Math.max(0, Math.min(100, ((audioLevels.sysDb + 60) / 60) * 100))}%`,
                              background: "linear-gradient(to right, var(--accent-info) 65%, #ffcc00 85%, #ff4444 100%)",
                            }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Webcam Section ── */}
          <Card title="Webcam Overlay" icon="photo_camera" index={2}>
            <Row label="Enable Webcam Overlay" desc="Picture-in-Picture webcam overlay on the final recording">
              <Toggle checked={local.webcam_enabled} onChange={(v) => update("webcam_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.webcam_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden space-y-4 pt-4 border-t"
                  style={{ borderColor: "var(--border-default)" }}
                >
                  <motion.button
                    onClick={() => setShowWebcamPreview(true)}
                    className="w-full py-3 font-mono text-xs uppercase font-extrabold cursor-pointer flex items-center justify-center gap-2 border rounded shadow-lg"
                    style={{
                      background: "var(--accent-primary)",
                      borderColor: "var(--accent-primary-hover)",
                      color: "var(--on-primary)"
                    }}
                    whileHover={{ scale: 1.01, boxShadow: "0 0 15px rgba(0, 232, 138, 0.15)" }}
                    whileTap={{ scale: 0.98 }}
                  >
                    <Icon name="visibility" size={14} /> Open Webcam Preview & Settings
                  </motion.button>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Keyboard Overlay Section ── */}
          <Card title="Keyboard Overlay" icon="keyboard" index={3}>
            <Row label="Enable Keyboard Overlay" desc="Show real-time keys and shortcut bubbles during recording">
              <Toggle checked={local.keyboard_overlay_enabled} onChange={(v) => update("keyboard_overlay_enabled", v)} />
            </Row>
            {local.keyboard_overlay_enabled && (
              <Row label="Record Games (Keyboard Capture)" desc="Allows key capture inside fullscreen games (relaunches app as Administrator)">
                <Toggle checked={local.keyboard_game_capture} onChange={(v) => update("keyboard_game_capture", v)} />
              </Row>
            )}
            <AnimatePresence>
              {local.keyboard_overlay_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden space-y-4 pt-4 border-t"
                  style={{ borderColor: "var(--border-default)" }}
                >
                  <motion.button
                    onClick={() => setShowKeyboardPreview(true)}
                    className="w-full py-3 font-mono text-xs uppercase font-extrabold cursor-pointer flex items-center justify-center gap-2 border rounded shadow-lg"
                    style={{
                      background: "var(--accent-primary)",
                      borderColor: "var(--accent-primary-hover)",
                      color: "var(--on-primary)"
                    }}
                    whileHover={{ scale: 1.01, boxShadow: "0 0 15px rgba(0, 232, 138, 0.15)" }}
                    whileTap={{ scale: 0.98 }}
                  >
                    <Icon name="visibility" size={14} /> Configure Keyboard Preview & Glass Styles
                  </motion.button>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Auto-Zoom Section ── */}
          <Card title="Auto-Zoom" icon="zoom_in" index={4} badge="Post-processing">
            <Row label="Enable Auto-Zoom" desc="Automatically pan and zoom towards mouse click positions in post-processing">
              <Toggle checked={local.auto_zoom_enabled} onChange={(v) => update("auto_zoom_enabled", v)} />
            </Row>
            <AnimatePresence>
              {local.auto_zoom_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden space-y-4 pt-4 border-t"
                  style={{ borderColor: "var(--border-default)" }}
                >
                  <Row label="Zoom Factor" desc="How close to zoom in on targets">
                    <Segmented
                      value={local.zoom_level}
                      options={[
                        { label: "1.5×", value: 1.5 },
                        { label: "2.0×", value: 2.0 },
                        { label: "2.5×", value: 2.5 },
                        { label: "3.0×", value: 3.0 },
                      ]}
                      onChange={(v) => update("zoom_level", v as number)}
                    />
                  </Row>
                  <Row label="Dwell Hold Duration" desc="Time window to hold zoom at target click coordinates">
                    <Select
                      value={local.zoom_dwell_ms}
                      onChange={(v) => update("zoom_dwell_ms", Number(v))}
                      options={[
                        { label: "500ms — Quick snap", value: 500 },
                        { label: "1.0s — Standard", value: 1000 },
                        { label: "1.5s — Longer hold", value: 1500 },
                        { label: "2.0s — Continuous", value: 2000 },
                      ]}
                    />
                  </Row>
                  <Row label="Pan Smoothing Speed" desc="How quickly the camera glides to zoom targets">
                    <Slider value={local.zoom_speed} min={0.5} max={3.0} step={0.1} onChange={(v) => update("zoom_speed", v)} suffix=" s" />
                  </Row>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Cursor & Click Effects Redesign ── */}
          <Card title="Cursor & Click Effects" icon="mouse" index={5} badge="Post-processing">
            <Row label="Cursor Custom Pack" desc="Visual pointer pack override used in post-processing">
              <Select
                value={local.cursor_pack || "default"}
                onChange={(v) => update("cursor_pack", v)}
                options={[
                  { label: "System OS Default", value: "default" },
                  ...cursorPacks.map((p) => ({ label: p.name, value: p.id })),
                ]}
              />
            </Row>
            
            <div className="flex justify-between items-center py-2.5 px-4 rounded-lg border" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
              <div className="flex flex-col">
                <span className="font-mono text-xs font-semibold">Test Cursor Swaps Live</span>
                <span className="font-mono text-[9px]" style={{ color: "var(--text-secondary)" }}>Applies selected pack to cursor for 30 seconds</span>
              </div>
              <motion.button
                onClick={handleCursorPreview}
                className="font-mono text-[10px] px-3.5 py-1.5 cursor-pointer font-bold uppercase rounded border transition-colors"
                style={{
                  borderColor: previewActive ? "#ff44cc" : "var(--accent-primary)",
                  color: previewActive ? "#ff44cc" : "var(--accent-primary)",
                  background: previewActive ? "rgba(255, 68, 204, 0.08)" : "rgba(0, 232, 138, 0.04)"
                }}
                whileHover={{ scale: 1.03 }}
                whileTap={{ scale: 0.97 }}
              >
                {previewActive ? "■ STOP PREVIEW" : "▶ TEST SYSTEM CURSOR"}
              </motion.button>
            </div>

            <Row label="Enable Trail Effect" desc="Render neon spline trail behind cursor tracks">
              <Toggle checked={local.cursor_trail_enabled} onChange={(v) => update("cursor_trail_enabled", v)} />
            </Row>
            
            <AnimatePresence>
              {local.cursor_trail_enabled && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                  className="overflow-hidden space-y-4 pt-4 border-t"
                  style={{ borderColor: "var(--border-default)" }}
                >
                  <Row label="Trail Style Pattern" desc="Visual effect render style for the trail path">
                    <Select
                      value={local.trail_style || "glow"}
                      onChange={(v) => update("trail_style", v)}
                      options={TRAIL_STYLES.map((s) => ({ label: s.label, value: s.id }))}
                    />
                  </Row>
                  <Row label="Trail Primary Color" desc="Color of the cursor movement trail">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="color"
                        value={local.cursor_trail_color}
                        onChange={(e) => update("cursor_trail_color", e.target.value)}
                        className="w-7 h-7 cursor-pointer bg-transparent border"
                        style={{ borderColor: "var(--border-default)" }}
                      />
                      <span className="font-mono text-xs uppercase" style={{ color: local.cursor_trail_color }}>
                        {local.cursor_trail_color}
                      </span>
                    </div>
                  </Row>
                  <Row label="Secondary Right-Click Color" desc="Color for right clicks and glow gradients">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="color"
                        value={local.cursor_secondary_color || "#ff4488"}
                        onChange={(e) => update("cursor_secondary_color", e.target.value)}
                        className="w-7 h-7 cursor-pointer bg-transparent border"
                        style={{ borderColor: "var(--border-default)" }}
                      />
                      <span className="font-mono text-xs uppercase" style={{ color: local.cursor_secondary_color || "#ff4488" }}>
                        {local.cursor_secondary_color || "#ff4488"}
                      </span>
                    </div>
                  </Row>
                  <Row label="Trail Length" desc="Adjust duration length of cursor trail spline visibility">
                    <Slider value={local.trail_length || 0.5} min={0.1} max={1.0} step={0.05} onChange={(v) => update("trail_length", v)} suffix="" />
                  </Row>
                  <Row label="Cursor Scale Multiplier" desc="Scale factor sizes of cursor pointer in output">
                    <Segmented
                      value={local.cursor_size_multiplier}
                      options={[
                        { label: "1.0×", value: 1.0 },
                        { label: "1.5×", value: 1.5 },
                        { label: "2.0×", value: 2.0 },
                        { label: "3.0×", value: 3.0 },
                      ]}
                      onChange={(v) => update("cursor_size_multiplier", v as number)}
                    />
                  </Row>
                  <Row label="Motion Path Smoothing" desc="Filters mouse jitter for fluid post-processed moves">
                    <Toggle checked={local.cursor_smoothing} onChange={(v) => update("cursor_smoothing", v)} />
                  </Row>
                  <Row label="Click Highlight Effect" desc="Render animation wave on mouse clicks">
                    <Select
                      value={local.click_effect || "ripple"}
                      onChange={(v) => update("click_effect", v)}
                      options={CLICK_EFFECTS.map((s) => ({ label: s.label, value: s.id }))}
                    />
                  </Row>

                  {/* Interactive Cursor Trail Canvas Preview */}
                  <div className="space-y-2">
                    <div className="flex justify-between font-mono text-[9px]" style={{ color: "var(--text-secondary)" }}>
                      <span>INTERACTIVE CANVAS EFFECT PREVIEW</span>
                      <span>{local.trail_style.toUpperCase()} · {local.click_effect.toUpperCase()}</span>
                    </div>
                    <div className="relative overflow-hidden border border-dashed rounded-xl" style={{ borderColor: "var(--border-default)", background: "var(--surface-container-low)" }}>
                      <MiniPreview
                        trailStyle={local.trail_style as TrailStyle}
                        clickEffect={local.click_effect as ClickEffect}
                        color={local.cursor_trail_color}
                      />
                    </div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>

          {/* ── Hotkeys Section ── */}
          <Card title="Global Hotkeys" icon="keyboard" index={6}>
            <Row label="Start Recording" desc="Global keyboard shortcut to trigger capture starting">
              <HotkeyRecorder value={local.hotkey_start} onChange={(v) => update("hotkey_start", v)} />
            </Row>
            <Row label="Stop Recording" desc="Global keyboard shortcut to trigger capture stopping">
              <HotkeyRecorder value={local.hotkey_stop} onChange={(v) => update("hotkey_stop", v)} />
            </Row>
            <Row label="Pause / Resume" desc="Global keyboard shortcut to trigger capture pausing">
              <HotkeyRecorder value={local.hotkey_pause} onChange={(v) => update("hotkey_pause", v)} />
            </Row>
          </Card>

          {/* ── General Section ── */}
          <Card title="General Settings" icon="settings" index={7}>
            <Row label="Output Capture Directory" desc="Absolute directory path where recorded video packages save to">
              <Input value={local.output_dir} onChange={(v) => update("output_dir", v)} />
            </Row>
            <Row label="Minimize to System Tray" desc="Toggles hiding dashboard UI into taskbar tray on recording start">
              <Toggle checked={local.minimize_to_tray} onChange={(v) => update("minimize_to_tray", v)} />
            </Row>
            <Row label="Copy Path to Clipboard" desc="Auto-copy absolute target file path after stopping recordings">
              <Toggle checked={local.copy_path_on_save} onChange={(v) => update("copy_path_on_save", v)} />
            </Row>
          </Card>
        </div>
      </div>

      {/* Webcam Preview Overlay Modal Wrapper */}
      <AnimatePresence>
        {showWebcamPreview && (
          <WebcamPreview
            initial={{
              x: local.webcam_x,
              y: local.webcam_y,
              size: local.webcam_size,
              shape: local.webcam_shape.toLowerCase() as any,
              borderColor: local.webcam_border_color,
              borderWidth: local.webcam_border_width,
              opacity: local.webcam_opacity,
              device: local.webcam_device,
              sharpen: local.webcam_sharpen,
              brightness: local.webcam_brightness,
              contrast: local.webcam_contrast,
            }}
            recordingWidth={local.resolution_width}
            recordingHeight={local.resolution_height}
            onSave={(cfg) => {
              update("webcam_x", cfg.x);
              update("webcam_y", cfg.y);
              update("webcam_size", cfg.size);
              update("webcam_shape", cfg.shape.charAt(0).toUpperCase() + cfg.shape.slice(1) as any);
              update("webcam_border_color", cfg.borderColor);
              update("webcam_border_width", cfg.borderWidth);
              update("webcam_opacity", cfg.opacity);
              update("webcam_device", cfg.device);
              update("webcam_sharpen", cfg.sharpen);
              update("webcam_brightness", cfg.brightness);
              update("webcam_contrast", cfg.contrast);
              setShowWebcamPreview(false);
            }}
            onCancel={() => setShowWebcamPreview(false)}
          />
        )}
      </AnimatePresence>

      {/* Keyboard Preview Overlay Modal Wrapper */}
      <AnimatePresence>
        {showKeyboardPreview && (
          <KeyboardPreview
            initial={{
              enabled: local.keyboard_overlay_enabled,
              fontFamily: local.keyboard_overlay_font_family,
              fontSize: local.keyboard_overlay_font_size,
              opacity: local.keyboard_overlay_opacity,
              x: local.keyboard_overlay_x,
              y: local.keyboard_overlay_y,
              width: local.keyboard_overlay_width,
              cornerRadius: local.keyboard_overlay_corner_radius,
              borderWidth: local.keyboard_overlay_border_width,
              borderColor: local.keyboard_overlay_border_color,
              backgroundColor: local.keyboard_overlay_background_color,
              textColor: local.keyboard_overlay_text_color,
              theme: local.keyboard_overlay_theme,
              keyMappings: local.keyboard_overlay_key_mappings,
              maxBubbles: local.keyboard_overlay_max_bubbles,
              bubbleTimeoutMs: local.keyboard_overlay_bubble_timeout_ms,
            }}
            recordingWidth={local.resolution_width}
            recordingHeight={local.resolution_height}
            onSave={(cfg) => {
              update("keyboard_overlay_enabled", cfg.enabled);
              update("keyboard_overlay_font_family", cfg.fontFamily);
              update("keyboard_overlay_font_size", cfg.fontSize);
              update("keyboard_overlay_opacity", cfg.opacity);
              update("keyboard_overlay_x", cfg.x);
              update("keyboard_overlay_y", cfg.y);
              update("keyboard_overlay_width", cfg.width);
              update("keyboard_overlay_corner_radius", cfg.cornerRadius);
              update("keyboard_overlay_border_width", cfg.borderWidth);
              update("keyboard_overlay_border_color", cfg.borderColor);
              update("keyboard_overlay_background_color", cfg.backgroundColor);
              update("keyboard_overlay_text_color", cfg.textColor);
              update("keyboard_overlay_theme", cfg.theme);
              update("keyboard_overlay_key_mappings", cfg.keyMappings);
              update("keyboard_overlay_max_bubbles", cfg.maxBubbles);
              update("keyboard_overlay_bubble_timeout_ms", cfg.bubbleTimeoutMs);
              setShowKeyboardPreview(false);
            }}
            onCancel={() => setShowKeyboardPreview(false)}
          />
        )}
      </AnimatePresence>

      {/* ═══ SAVE BUTTON FOOTER ═══ */}
      <div className="px-6 py-4 border-t flex justify-center z-40 shadow-2xl" style={{ background: "var(--bg-surface)", borderColor: "var(--border-default)" }}>
        <motion.button
          onClick={handleSave}
          className="max-w-3xl w-full py-3 font-mono text-xs uppercase font-extrabold cursor-pointer flex items-center justify-center gap-2 shadow-lg rounded"
          style={{
            border: "none",
            background: saved ? "var(--accent-primary-container)" : "linear-gradient(135deg, var(--accent-primary-container), var(--accent-primary))",
            color: "var(--on-primary)"
          }}
          whileHover={{ scale: 1.01, boxShadow: "0 0 25px rgba(0, 232, 138, 0.15)" }}
          whileTap={{ scale: 0.98 }}
        >
          {saved ? (
            <span className="flex items-center gap-1.5">
              <Icon name="check_circle" size={16} /> CONFIGURATION SAVED SUCCESSFULLY
            </span>
          ) : (
            <span className="flex items-center gap-1.5">
              <Icon name="save" size={16} /> SAVE ALL SETTINGS
            </span>
          )}
        </motion.button>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════
   SETTINGS SUB-COMPONENTS
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
      transition={{ delay: index * 0.05, duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
      className="border-2 rounded-xl overflow-hidden transition-colors"
      style={{ background: "var(--bg-surface)", borderColor: "var(--border-default)" }}
      whileHover={{ borderColor: "var(--border-strong)" }}
    >
      {/* Card Header */}
      <div className="flex items-center justify-between px-5 py-3 border-b bg-black/5" style={{ borderColor: "var(--border-default)" }}>
        <div className="flex items-center gap-2">
          <Icon name={icon} size={18} style={{ color: "var(--accent-primary)" }} />
          <span className="font-mono text-xs font-bold uppercase tracking-wider" style={{ color: "var(--text-primary)" }}>
            {title}
          </span>
        </div>
        {badge && (
          <span className="font-mono text-[9px] px-2 py-0.5 rounded-full font-bold border" style={{ color: "var(--accent-primary)", background: "rgba(0, 232, 138, 0.08)", borderColor: "rgba(0, 232, 138, 0.2)" }}>
            {badge}
          </span>
        )}
      </div>

      {/* Card Body */}
      <div className="px-5 py-4 space-y-4">
        {children}
      </div>
    </motion.div>
  );
}

function Row({ label, desc, children }: { label: string; desc?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <div className="font-mono text-xs font-semibold" style={{ color: "var(--text-primary)" }}>{label}</div>
        {desc && (
          <div className="font-mono mt-1 text-[10px] leading-relaxed" style={{ color: "var(--text-secondary)" }}>
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
      className="px-3 py-1.5 font-mono text-xs cursor-pointer min-w-[170px] outline-none border rounded focus:border-[var(--accent-primary)]"
      style={{
        borderColor: "var(--border-default)",
        background: "var(--surface-container-low)",
        color: "var(--text-primary)"
      }}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value} style={{ background: "var(--bg-surface)", color: "var(--text-primary)" }}>{o.label}</option>
      ))}
    </select>
  );
}

function Segmented<T extends string | number>({ value, options, onChange }: {
  value: T; options: { label: string; value: T }[]; onChange: (v: T) => void;
}) {
  const layoutId = useId();
  return (
    <div className="flex p-1 rounded-lg border gap-1" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
      {options.map((opt) => (
        <motion.button
          key={String(opt.value)}
          onClick={() => onChange(opt.value)}
          className="px-3.5 py-1 font-mono text-[10px] cursor-pointer relative z-10 font-bold uppercase rounded"
          style={{
            color: opt.value === value ? "var(--on-primary)" : "var(--text-secondary)",
            background: "transparent",
          }}
          whileHover={opt.value !== value ? { color: "var(--text-primary)" } : {}}
          whileTap={{ scale: 0.96 }}
        >
          <span className="relative z-20">{opt.label}</span>
          {opt.value === value && (
            <motion.div
              layoutId={layoutId}
              className="absolute inset-0 z-0 rounded"
              style={{ background: "var(--accent-primary)" }}
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
      className="px-3 py-1.5 font-mono text-xs min-w-[200px] outline-none border rounded focus:border-[var(--accent-primary)]"
      style={{
        borderColor: "var(--border-default)",
        background: "var(--surface-container-low)",
        color: "var(--text-primary)"
      }}
    />
  );
}

function Slider({ value, min, max, step, onChange, suffix }: {
  value: number; min: number; max: number; step: number; onChange: (v: number) => void; suffix?: string;
}) {
  return (
    <div className="flex items-center gap-2.5 min-w-[170px]">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="flex-1 h-1.5 cursor-pointer rounded-full"
        style={{
          accentColor: "var(--accent-primary)",
          background: "var(--border-default)"
        }}
      />
      <span className="font-mono text-[10px] min-w-[40px] text-right" style={{ color: "var(--text-secondary)" }}>
        {value.toFixed(step < 0.1 ? 2 : 0)}{suffix}
      </span>
    </div>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <motion.button
      onClick={() => onChange(!checked)}
      className="relative cursor-pointer w-11 h-6 rounded-full border"
      style={{
        borderColor: checked ? "var(--accent-primary)" : "var(--border-default)",
        background: checked ? "var(--accent-primary)" : "var(--surface-container-low)",
      }}
      whileTap={{ scale: 0.95 }}
    >
      <motion.div
        className="absolute top-0.5 w-4.5 h-4.5 rounded-full"
        style={{ background: checked ? "var(--on-primary)" : "var(--text-primary)" }}
        animate={{ left: checked ? 22 : 2 }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}

function HotkeyRecorder({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [isRecording, setIsRecording] = useState(false);
  const [tempModifiers, setTempModifiers] = useState({
    ctrl: false, shift: false, alt: false, super: false
  });
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!isRecording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;
      const isModifier = ["Control", "Shift", "Alt", "Meta", "OS"].includes(key);

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
        const parts: string[] = [];
        if (newModifiers.ctrl) parts.push("Ctrl");
        if (newModifiers.shift) parts.push("Shift");
        if (newModifiers.alt) parts.push("Alt");
        if (newModifiers.super) parts.push("Super");

        let keyName = key;
        if (key === " ") {
          keyName = "Space";
        } else if (key.length === 1) {
          keyName = key.toUpperCase();
        } else if (key.startsWith("Arrow")) {
          keyName = key.replace("Arrow", "");
        }

        parts.push(keyName);
        onChange(parts.join("+"));
        setIsRecording(false);
        buttonRef.current?.blur();
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
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

  return (
    <div className="relative">
      <motion.button
        ref={buttonRef}
        onClick={() => { setIsRecording(true); setTempModifiers({ ctrl: false, shift: false, alt: false, super: false }); }}
        onBlur={() => setIsRecording(false)}
        className="px-3.5 py-1.5 font-mono text-xs cursor-pointer min-w-[170px] text-center border rounded outline-none transition-colors"
        style={{
          borderColor: isRecording ? "var(--accent-primary)" : "var(--border-default)",
          background: isRecording ? "rgba(0, 232, 138, 0.08)" : "var(--surface-container-low)",
          color: isRecording ? "var(--accent-primary)" : "var(--text-primary)",
        }}
        whileTap={{ scale: 0.98 }}
      >
        {isRecording ? "Listening..." : value || "None"}
      </motion.button>

      <AnimatePresence>
        {isRecording && (
          <motion.div
            initial={{ opacity: 0, y: 4, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.95 }}
            className="absolute top-full mt-2 left-1/2 -translate-x-1/2 flex gap-1 p-1 shadow-2xl z-50 rounded border"
            style={{ background: "var(--bg-surface)", borderColor: "var(--border-default)" }}
          >
            {["Ctrl", "Shift", "Alt", "Win"].map((mod) => {
              const active = 
                (mod === "Ctrl" && tempModifiers.ctrl) ||
                (mod === "Shift" && tempModifiers.shift) ||
                (mod === "Alt" && tempModifiers.alt) ||
                (mod === "Win" && tempModifiers.super);
              return (
                <span
                  key={mod}
                  className="px-1.5 py-0.5 rounded text-[9px] font-bold border transition-colors duration-150"
                  style={{
                    background: active ? "var(--accent-primary)" : "transparent",
                    color: active ? "var(--on-primary)" : "var(--text-secondary)",
                    borderColor: active ? "var(--accent-primary)" : "var(--border-default)"
                  }}
                >
                  {mod}
                </span>
              );
            })}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ─── Mini preview canvas for effect cards ────────────────────────
function MiniPreview({
  trailStyle, clickEffect, color,
}: {
  trailStyle?: TrailStyle;
  clickEffect?: ClickEffect;
  color: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const trailRef = useRef(new TrailRenderer());
  const clickRef = useRef(new ClickEffectRenderer());
  const timeRef = useRef(0);
  const hasInteracted = useRef(false);

  useEffect(() => {
    if (trailStyle) { trailRef.current.setStyle(trailStyle); trailRef.current.setColor(color); }
    if (clickEffect) { clickRef.current.setStyle(clickEffect); clickRef.current.setColor(color); }
  }, [trailStyle, clickEffect, color]);

  useEffect(() => {
    if (!clickEffect || clickEffect === "none") return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    clickRef.current.startAutoSimulate(canvas);
    return () => { clickRef.current.stopAutoSimulate(); };
  }, [clickEffect]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    let raf: number;

    const animate = () => {
      timeRef.current++;
      const w = canvas.width;
      const h = canvas.height;

      ctx.clearRect(0, 0, w, h);
      drawPreviewBackground(ctx, w, h);

      trailRef.current.update();
      trailRef.current.draw(ctx, w, h);

      if (trailStyle && trailStyle !== "none" && !hasInteracted.current) {
        const pulse = 0.4 + Math.sin(timeRef.current * 0.04) * 0.15;
        ctx.font = "11px 'JetBrains Mono', monospace";
        ctx.fillStyle = `rgba(255,255,255,${pulse})`;
        ctx.textAlign = "center";
        ctx.fillText("move cursor here to test trail", w / 2, h / 2 + 4);
        ctx.textAlign = "left";
      }

      clickRef.current.update();
      clickRef.current.draw(ctx, w, h);

      raf = requestAnimationFrame(animate);
    };
    animate();
    return () => cancelAnimationFrame(raf);
  }, [trailStyle, clickEffect]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!trailStyle || trailStyle === "none") return;
    hasInteracted.current = true;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    trailRef.current.addPoint(x, y);
  }, [trailStyle]);

  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!clickEffect || clickEffect === "none") return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    clickRef.current.addClick(x, y);
  }, [clickEffect]);

  return (
    <canvas
      ref={canvasRef}
      width={700}
      height={140}
      className="w-full cursor-crosshair block rounded-lg h-[140px]"
      onMouseMove={handleMouseMove}
      onClick={handleClick}
    />
  );
}
