import { useEffect, useState, useRef } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore, type RecordingEntry } from "../stores/recording";
import { useThemeStore } from "../lib/theme";
import { invoke } from "@tauri-apps/api/core";

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  return `${h.toString().padStart(2, "0")}:${(m % 60).toString().padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;
}

function normalizePath(p: string): string {
  return p.replace(/\//g, "\\");
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(0)}KB`;
  return `${(bytes / 1_048_576).toFixed(1)}MB`;
}

// ═══ INLINE PRESET CARD ═══
function PresetCard({
  label, value, options, onSelect, index, disabled,
}: {
  label: string;
  value: string;
  options: { label: string; value: string }[];
  onSelect: (v: string) => void;
  index: number;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <motion.div
      ref={ref}
      className="relative text-center px-2 py-3 cursor-pointer select-none"
      style={{
        border: `var(--border-width) solid ${open ? "var(--accent-primary)" : "var(--border-default)"}`,
        background: "var(--bg-surface)",
      }}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.1 + index * 0.05, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      whileHover={disabled ? {} : { y: -2, borderColor: "var(--border-strong)" }}
      onClick={() => !disabled && setOpen(!open)}
    >
      <div className="font-mono uppercase" style={{ color: "var(--text-muted)", letterSpacing: "0.08em", fontSize: "0.55rem" }}>
        {label}
      </div>
      <div className="font-mono text-sm font-bold mt-1" style={{ color: "var(--text-primary)" }}>
        {value}
      </div>
      <div className="font-mono" style={{ color: "var(--text-muted)", fontSize: "0.5rem", marginTop: 2 }}>
        {disabled ? "" : "▼"}
      </div>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -4, scale: 0.97 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -4, scale: 0.97 }}
            transition={{ duration: 0.15, ease: [0.16, 1, 0.3, 1] }}
            className="absolute left-0 right-0 z-50 mt-1"
            style={{
              top: "100%",
              border: "var(--border-width) solid var(--border-strong)",
              background: "var(--bg-elevated)",
              boxShadow: "var(--shadow-md)",
            }}
          >
            {options.map((opt) => (
              <motion.button
                key={opt.value}
                className="w-full px-3 py-2 text-left font-mono text-xs cursor-pointer"
                style={{
                  color: opt.value === value ? "var(--accent-primary)" : "var(--text-secondary)",
                  background: opt.value === value ? "var(--bg-surface)" : "transparent",
                  borderBottom: "var(--border-thin) solid var(--border-default)",
                  letterSpacing: "0.03em",
                }}
                whileHover={{ background: "var(--bg-surface)", color: "var(--text-primary)" }}
                onClick={(e) => {
                  e.stopPropagation();
                  onSelect(opt.value);
                  setOpen(false);
                }}
              >
                {opt.label}
                {opt.value === value && <span className="ml-2" style={{ color: "var(--accent-primary)" }}>●</span>}
              </motion.button>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

// ═══ RECORDING HISTORY ═══
function HistoryPanel({ entries, onOpen, onClear }: {
  entries: RecordingEntry[];
  onOpen: (path: string) => void;
  onClear: () => void;
}) {
  if (entries.length === 0) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.3, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      className="w-full max-w-xl"
    >
      <div className="flex items-center justify-between mb-2">
        <span className="font-mono text-xs uppercase" style={{ color: "var(--text-muted)", letterSpacing: "0.08em" }}>
          Recent Recordings
        </span>
        <motion.button
          className="font-mono text-xs cursor-pointer"
          style={{ color: "var(--text-muted)" }}
          whileHover={{ color: "var(--accent-danger)" }}
          onClick={onClear}
        >
          Clear
        </motion.button>
      </div>
      <div className="space-y-1 max-h-32 overflow-y-auto">
        {entries.slice(0, 5).map((entry, i) => (
          <motion.div
            key={entry.id}
            className="flex items-center gap-3 px-3 py-2 cursor-pointer group"
            style={{
              border: "var(--border-thin) solid var(--border-default)",
              background: "var(--bg-surface)",
            }}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: i * 0.03 }}
            whileHover={{ borderColor: "var(--accent-info)" }}
            onClick={() => onOpen(entry.output_path)}
          >
            <div className="flex-1 min-w-0">
              <div className="font-mono text-xs" style={{ color: "var(--text-primary)" }}>
                {entry.duration_secs.toFixed(1)}s · {formatBytes(entry.file_size_bytes)} · {entry.resolution}
              </div>
              <div className="font-mono truncate" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
                {normalizePath(entry.output_path)}
              </div>
            </div>
            <span className="font-mono text-xs opacity-0 group-hover:opacity-100 transition-opacity" style={{ color: "var(--accent-info)" }}>
              OPEN
            </span>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

// ═══ AUDIO LEVEL METER ═══
function AudioMeter({ active }: { active: boolean }) {
  const [level, setLevel] = useState(0);
  const animRef = useRef<number>(0);

  useEffect(() => {
    if (!active) { setLevel(0); return; }
    const tick = () => {
      // Simulated mic level — real implementation would use Web Audio API
      setLevel(Math.random() * 0.6 + 0.1);
      animRef.current = requestAnimationFrame(tick);
    };
    animRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(animRef.current);
  }, [active]);

  return (
    <div className="flex items-center gap-1.5">
      <span className="font-mono" style={{ color: "var(--text-muted)", fontSize: "0.5rem" }}>MIC</span>
      <div className="flex gap-0.5 h-3">
        {Array.from({ length: 8 }).map((_, i) => (
          <div
            key={i}
            className="w-1 transition-all duration-75"
            style={{
              height: 12,
              background: i / 8 < level
                ? i < 5 ? "var(--accent-success)" : i < 7 ? "var(--accent-warning)" : "var(--accent-danger)"
                : "var(--bg-elevated)",
            }}
          />
        ))}
      </div>
    </div>
  );
}

// ═══ MAIN DASHBOARD ═══
export function Dashboard({ onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    config, recordingPhase, isPaused, recordingStartTime, lastRecording, history,
    startRecording, stopRecording, pauseRecording, resumeRecording,
    openPath, updateField, loadHistory, clearHistory,
  } = useStore();

  const { theme, toggleTheme } = useThemeStore();
  const [elapsed, setElapsed] = useState("00:00:00");
  const [version, setVersion] = useState("0.1.0");

  useEffect(() => {
    invoke<string>("get_version").then(setVersion).catch(() => {});
    loadHistory();
  }, []);

  useEffect(() => {
    if (recordingPhase !== "recording" || !recordingStartTime) return;
    const interval = setInterval(() => setElapsed(formatDuration(Date.now() - recordingStartTime)), 1000);
    return () => clearInterval(interval);
  }, [recordingPhase, recordingStartTime]);

  const isRecording = recordingPhase === "recording";
  const isEncoding = recordingPhase === "encoding";
  const isIdle = recordingPhase === "idle";

  const resValue = config ? `${config.resolution_width}×${config.resolution_height}` : "1920×1080";
  const fpsValue = config ? `${config.fps}` : "30";
  const audioValue = config?.audio_enabled
    ? config.audio_source === "Both" ? "M+S" : config.audio_source === "System" ? "SYS" : "MIC"
    : "OFF";
  const modeValue = config?.recording_mode === "FullScreen" ? "FULL" : config?.recording_mode === "Region" ? "REGION" : "WINDOW";

  const recordingBorder = isRecording
    ? isPaused ? "3px solid var(--accent-warning)" : "3px solid var(--accent-danger)"
    : "3px solid transparent";

  return (
    <div
      className="flex flex-col h-full relative"
      style={{
        background: "var(--bg-base)",
        borderLeft: recordingBorder,
        transition: "border-color var(--duration-normal) var(--ease-out-expo)",
      }}
    >
      {/* ═══ HEADER ═══ */}
      <header className="flex items-center justify-between px-6 py-3" style={{ borderBottom: "var(--border-width) solid var(--border-default)" }}>
        <div className="flex items-center gap-3">
          <motion.div
            className="flex items-center justify-center font-mono text-sm font-bold"
            style={{
              width: 36, height: 36,
              background: isRecording ? "var(--accent-danger)" : "var(--accent-primary)",
              color: "white",
              border: "var(--border-width) solid var(--border-strong)",
              transition: "background var(--duration-normal) var(--ease-out-expo)",
            }}
            whileHover={{ scale: 1.05, rotate: -2 }}
            whileTap={{ scale: 0.95 }}
          >
            ES
          </motion.div>
          <div className="flex flex-col">
            <span className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>EASYSPECY</span>
            <span className="font-mono text-xs" style={{ color: "var(--text-muted)", letterSpacing: "0.05em" }}>V{version}</span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {isRecording && <AudioMeter active={!isPaused} />}
          <motion.button
            onClick={toggleTheme}
            className="flex items-center justify-center cursor-pointer"
            style={{ width: 36, height: 36, border: "var(--border-width) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-secondary)" }}
            whileHover={{ scale: 1.05, borderColor: "var(--border-strong)" }}
            whileTap={{ scale: 0.95 }}
            title={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
          >
            <motion.span key={theme} initial={{ rotate: -90, opacity: 0 }} animate={{ rotate: 0, opacity: 1 }} transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }} className="text-base">
              {theme === "dark" ? "☀" : "●"}
            </motion.span>
          </motion.button>
          <motion.button
            onClick={onOpenSettings}
            disabled={isRecording || isEncoding}
            className="flex items-center gap-2 px-3 py-2 text-sm font-mono uppercase cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
            style={{ border: "var(--border-width) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-secondary)", letterSpacing: "0.05em", fontSize: "var(--text-xs)" }}
            whileHover={{ scale: 1.02, y: -1 }}
            whileTap={{ scale: 0.97 }}
          >
            CONFIG
          </motion.button>
        </div>
      </header>

      {/* ═══ MAIN CONTENT ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center gap-5 px-6 overflow-y-auto py-4">

        {/* Recording Timer */}
        <AnimatePresence>
          {isRecording && (
            <motion.div
              initial={{ opacity: 0, y: -20, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -20, scale: 0.95 }}
              transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
              className="flex items-center gap-4 px-6 py-3"
              style={{ border: `var(--border-width) solid ${isPaused ? "var(--accent-warning)" : "var(--accent-danger)"}`, background: "var(--bg-surface)" }}
            >
              <motion.div
                className="w-3 h-3"
                style={{ background: isPaused ? "var(--accent-warning)" : "var(--accent-danger)", animation: isPaused ? "none" : "pulse-dot 1.5s ease-in-out infinite" }}
              />
              <span className="font-mono text-2xl font-bold tracking-widest" style={{ color: "var(--text-primary)" }}>{elapsed}</span>
              {isPaused && (
                <motion.span initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} className="font-mono text-xs font-bold uppercase" style={{ color: "var(--accent-warning)", letterSpacing: "0.1em" }}>
                  PAUSED
                </motion.span>
              )}
            </motion.div>
          )}
        </AnimatePresence>

        {/* Encoding Progress */}
        <AnimatePresence>
          {isEncoding && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
              className="flex flex-col items-center gap-3 px-8 py-5 min-w-[280px]"
              style={{ border: "var(--border-width) solid var(--accent-info)", background: "var(--bg-surface)" }}
            >
              <div className="flex items-center gap-3">
                <motion.div animate={{ rotate: 360 }} transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }} className="w-4 h-4 border-2 border-t-transparent rounded-full" style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }} />
                <span className="font-mono text-sm uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>Encoding</span>
              </div>
              <div className="w-full h-1 overflow-hidden" style={{ background: "var(--bg-elevated)" }}>
                <motion.div className="h-full w-1/3" style={{ background: "var(--accent-info)" }} animate={{ x: ["-100%", "300%"] }} transition={{ duration: 1.8, repeat: Infinity, ease: "easeInOut" }} />
              </div>
              <span className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>MERGING VIDEO + AUDIO</span>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Last Recording */}
        <AnimatePresence>
          {lastRecording && isIdle && (
            <motion.div
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 16 }}
              transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
              onClick={() => openPath(lastRecording.output_path)}
              className="flex items-center gap-4 px-5 py-3 cursor-pointer w-full max-w-md group"
              style={{ border: "var(--border-width) solid var(--accent-success)", background: "var(--bg-surface)" }}
            >
              <motion.div className="font-mono text-lg font-bold" style={{ color: "var(--accent-success)" }} initial={{ scale: 0 }} animate={{ scale: 1 }} transition={{ delay: 0.2, type: "spring", stiffness: 400, damping: 15 }}>✓</motion.div>
              <div className="flex-1 min-w-0">
                <div className="font-mono text-xs uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.03em" }}>
                  {lastRecording.duration_secs.toFixed(1)}s · {formatBytes(lastRecording.file_size_bytes)} · {lastRecording.has_audio ? "VIDEO+AUDIO" : "VIDEO"}
                </div>
                <div className="text-xs truncate mt-1" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>{normalizePath(lastRecording.output_path)}</div>
              </div>
              <motion.span className="font-mono text-xs font-bold uppercase" style={{ color: "var(--accent-info)" }} whileHover={{ x: 3 }}>OPEN →</motion.span>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ RECORD BUTTON ═══ */}
        <motion.button
          onClick={isRecording ? stopRecording : isIdle ? startRecording : undefined}
          disabled={isEncoding}
          className="relative flex items-center justify-center cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          style={{
            width: 120, height: 120,
            border: `3px solid ${isRecording ? "var(--accent-danger)" : isEncoding ? "var(--accent-info)" : "var(--accent-primary)"}`,
            background: "var(--bg-surface)",
            animation: isRecording ? "breathe 2s ease-in-out infinite" : "none",
          }}
          whileHover={isIdle ? { scale: 1.06, y: -2 } : isRecording ? { scale: 1.03 } : {}}
          whileTap={!isEncoding ? { scale: 0.94 } : {}}
          transition={{ type: "spring", stiffness: 400, damping: 20 }}
        >
          <AnimatePresence mode="wait">
            {isRecording ? (
              <motion.div key="stop" initial={{ scale: 0, rotate: 90 }} animate={{ scale: 1, rotate: 0 }} exit={{ scale: 0, rotate: -90 }} transition={{ type: "spring", stiffness: 500, damping: 25 }} className="w-8 h-8" style={{ background: "var(--accent-danger)" }} />
            ) : isEncoding ? (
              <motion.div key="encoding" animate={{ rotate: 360 }} transition={{ duration: 2, repeat: Infinity, ease: "linear" }} className="w-8 h-8 border-3 border-t-transparent rounded-full" style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }} />
            ) : (
              <motion.div key="record" initial={{ scale: 0 }} animate={{ scale: 1 }} exit={{ scale: 0 }} transition={{ type: "spring", stiffness: 500, damping: 25 }} className="w-8 h-8 rounded-full" style={{ background: "var(--accent-primary)" }} />
            )}
          </AnimatePresence>
        </motion.button>

        {/* Pause Button */}
        <AnimatePresence>
          {isRecording && (
            <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 12 }} transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}>
              <motion.button
                onClick={isPaused ? resumeRecording : pauseRecording}
                className="px-5 py-2 font-mono text-xs uppercase cursor-pointer"
                style={{ border: "var(--border-width) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-primary)", letterSpacing: "0.05em" }}
                whileHover={{ scale: 1.03, y: -1 }}
                whileTap={{ scale: 0.96 }}
              >
                {isPaused ? "▶ RESUME" : "⏸ PAUSE"}
              </motion.button>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ INLINE PRESET CARDS ═══ */}
        <motion.div
          className="grid grid-cols-4 gap-3 w-full max-w-xl"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
        >
          <PresetCard
            label="RES" value={resValue} index={0} disabled={isRecording}
            options={[
              { label: "480P (854×480)", value: "854×480" },
              { label: "720P (1280×720)", value: "1280×720" },
              { label: "1080P (1920×1080)", value: "1920×1080" },
            ]}
            onSelect={(v) => updateField("resolution", v.replace("×", "x"))}
          />
          <PresetCard
            label="FPS" value={fpsValue} index={1} disabled={isRecording}
            options={[
              { label: "24 FPS", value: "24" },
              { label: "30 FPS", value: "30" },
              { label: "60 FPS", value: "60" },
            ]}
            onSelect={(v) => updateField("fps", Number(v))}
          />
          <PresetCard
            label="AUDIO" value={audioValue} index={2} disabled={isRecording}
            options={[
              { label: "MICROPHONE", value: "Mic" },
              { label: "SYSTEM AUDIO", value: "System" },
              { label: "BOTH", value: "Both" },
              { label: "OFF", value: "Off" },
            ]}
            onSelect={(v) => {
              if (v === "Off") {
                updateField("audio_enabled", false);
              } else {
                updateField("audio_enabled", true);
                updateField("audio_source", v);
              }
            }}
          />
          <PresetCard
            label="MODE" value={modeValue} index={3} disabled={isRecording}
            options={[
              { label: "FULLSCREEN", value: "FullScreen" },
              { label: "REGION SELECT", value: "Region" },
              { label: "WINDOW", value: "Window" },
            ]}
            onSelect={(v) => updateField("recording_mode", v)}
          />
        </motion.div>

        {/* Feature Badges */}
        <motion.div className="flex items-center gap-2" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.3 }}>
          {config?.auto_zoom_enabled && <Badge text="AUTO-ZOOM" />}
          {config?.cursor_trail_enabled && <Badge text="CURSOR-FX" />}
          {config?.webcam_enabled && <Badge text="WEBCAM" />}
        </motion.div>

        {/* Hotkey Hint */}
        {isIdle && (
          <motion.div className="font-mono text-xs" style={{ color: "var(--text-muted)" }} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.4 }}>
            <kbd className="px-2 py-1 font-mono text-xs" style={{ border: "var(--border-thin) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-secondary)" }}>
              {config?.hotkey_start || "Ctrl+Shift+R"}
            </kbd>
            <span className="ml-2">TO RECORD</span>
          </motion.div>
        )}

        {/* ═══ RECORDING HISTORY ═══ */}
        <HistoryPanel entries={history} onOpen={openPath} onClear={clearHistory} />
      </div>

      {/* ═══ FOOTER ═══ */}
      <footer className="px-6 py-2.5 flex items-center justify-between" style={{ borderTop: "var(--border-width) solid var(--border-default)" }}>
        <motion.button
          onClick={() => openPath(config?.output_dir || "~/Videos/EasySpecy")}
          className="font-mono text-xs cursor-pointer flex items-center gap-2"
          style={{ color: "var(--text-muted)" }}
          whileHover={{ color: "var(--accent-info)", x: 2 }}
        >
          <span>→</span>
          <span>{normalizePath(config?.output_dir || "~/Videos/EasySpecy")}</span>
        </motion.button>
        <span className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>
          {history.length} RECORDINGS
        </span>
      </footer>
    </div>
  );
}

function Badge({ text }: { text: string }) {
  return (
    <motion.span
      className="font-mono text-xs px-2 py-1"
      style={{ border: "var(--border-thin) solid var(--border-default)", color: "var(--text-secondary)", letterSpacing: "0.05em", fontSize: "0.6rem" }}
      whileHover={{ borderColor: "var(--accent-primary)", color: "var(--accent-primary)" }}
    >
      {text}
    </motion.span>
  );
}
