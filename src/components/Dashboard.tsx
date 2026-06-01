import { useEffect, useState, useRef } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore, type RecordingEntry } from "../stores/recording";
import { RegionSelector } from "./RegionSelector";
import { StatusBar } from "./StatusBar";

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
      className="relative text-center px-3 py-3.5 cursor-pointer select-none"
      style={{
        border: `var(--border-width) solid ${open ? "var(--accent-primary)" : "var(--border-default)"}`,
        background: "var(--bg-surface)",
        borderRadius: "var(--radius-md)",
        boxShadow: "var(--shadow-sm)",
        zIndex: open ? 100 : 1,
      }}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.1 + index * 0.05, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      whileHover={disabled ? {} : { y: -2, borderColor: "var(--border-strong)", boxShadow: "var(--shadow-md)" }}
      onClick={() => !disabled && setOpen(!open)}
    >
      <div className="font-mono uppercase" style={{ color: "var(--text-muted)", letterSpacing: "0.08em", fontSize: "0.55rem" }}>
        {label}
      </div>
      <div className="font-mono text-xs font-bold mt-1.5 flex items-center justify-center gap-1" style={{ color: "var(--text-primary)" }}>
        <span>{value}</span>
        {!disabled && (
          <span className="transition-transform duration-200" style={{ transform: open ? "rotate(180deg)" : "rotate(0deg)", fontSize: "0.5rem", color: "var(--text-muted)" }}>
            ▼
          </span>
        )}
      </div>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: 4, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.95 }}
            transition={{ duration: 0.18, ease: [0.16, 1, 0.3, 1] }}
            className="absolute left-0 right-0 z-50 mt-2 overflow-hidden"
            style={{
              top: "100%",
              border: "var(--border-thin) solid var(--border-strong)",
              background: "var(--bg-elevated)",
              boxShadow: "var(--shadow-lg)",
              borderRadius: "var(--radius-sm)",
            }}
          >
            {options.map((opt) => (
              <motion.button
                key={opt.value}
                className="w-full px-3 py-2.5 text-left font-mono text-[10px] cursor-pointer flex items-center justify-between"
                style={{
                  color: opt.value === value ? "var(--text-primary)" : "var(--text-secondary)",
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
                <span>{opt.label}</span>
                {opt.value === value && <span style={{ color: "var(--accent-primary)" }}>✓</span>}
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
  if (entries.length === 0) {
    return (
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.3, duration: 0.3 }}
        className="w-full max-w-xl text-center p-6 border border-dashed rounded-lg mt-4"
        style={{
          borderColor: "var(--border-default)",
          background: "var(--bg-surface)",
          borderRadius: "var(--radius-md)",
        }}
      >
        <span className="text-xl mb-1.5 block">🎬</span>
        <h3 className="font-semibold text-xs mb-1" style={{ color: "var(--text-primary)" }}>No recordings yet</h3>
        <p className="text-[10px] max-w-xs mx-auto mb-3" style={{ color: "var(--text-secondary)" }}>
          Ready to capture your screen? Press the start hotkey or click the record button below to begin.
        </p>
        <div className="flex items-center justify-center gap-2 text-[10px] font-mono" style={{ color: "var(--text-muted)" }}>
          <span>Hotkey:</span>
          <kbd className="px-1.5 py-0.5" style={{ border: "var(--border-thin) solid var(--border-default)", background: "var(--bg-base)", borderRadius: "var(--radius-xs)" }}>Ctrl+Shift+R</kbd>
        </div>
      </motion.div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.3, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      className="w-full max-w-xl mt-4"
    >
      <div className="flex items-center justify-between mb-2">
        <span className="font-mono text-xs uppercase" style={{ color: "var(--text-muted)", letterSpacing: "0.08em" }}>
          Recent Recordings
        </span>
        <motion.button
          className="font-mono text-xs cursor-pointer px-2 py-0.5 font-semibold"
          style={{ color: "var(--text-muted)", border: "var(--border-thin) solid transparent" }}
          whileHover={{ color: "var(--accent-danger)", borderColor: "var(--border-default)", borderRadius: "var(--radius-xs)" }}
          onClick={onClear}
        >
          Clear
        </motion.button>
      </div>
      <div className="space-y-1.5 max-h-36 overflow-y-auto pr-1">
        {entries.slice(0, 5).map((entry, i) => (
          <motion.div
            key={entry.id}
            className="flex items-center gap-3 px-3 py-2.5 cursor-pointer group"
            style={{
              border: "var(--border-thin) solid var(--border-default)",
              background: "var(--bg-surface)",
              borderRadius: "var(--radius-sm)",
              boxShadow: "var(--shadow-sm)",
            }}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: i * 0.03 }}
            whileHover={{ borderColor: "var(--accent-info)", y: -1, boxShadow: "var(--shadow-md)" }}
            onClick={() => onOpen(entry.output_path)}
          >
            <div className="flex-1 min-w-0">
              <div className="font-mono text-xs font-semibold flex items-center gap-2" style={{ color: "var(--text-primary)" }}>
                <span>📁 {entry.resolution}</span>
                <span style={{ color: "var(--text-muted)" }}>·</span>
                <span>{entry.duration_secs.toFixed(1)}s</span>
                <span style={{ color: "var(--text-muted)" }}>·</span>
                <span>{formatBytes(entry.file_size_bytes)}</span>
              </div>
              <div className="font-mono truncate mt-1" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
                {normalizePath(entry.output_path)}
              </div>
            </div>
            <span className="font-mono text-[10px] opacity-0 group-hover:opacity-100 transition-opacity px-2 py-0.5" style={{ color: "var(--accent-info)", border: "var(--border-thin) solid var(--accent-info)", borderRadius: "var(--radius-xs)", background: "oklch(from var(--accent-info) l c h / 0.1)" }}>
              OPEN
            </span>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

// ═══ AUDIO LEVEL METER (REPLACED BY STATUSBAR) ═══
// Old AudioMeter removed — now using canvas-based StatusBar component

// ═══ MAIN DASHBOARD ═══
export function Dashboard({ onOpenSettings: _onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    config, recordingPhase, isPaused, recordingStartTime, lastRecording, history,
    startRecording, stopRecording, pauseRecording, resumeRecording,
    openPath, updateField, loadHistory, clearHistory, loadEstimatedSize,
    selectorMode, setCaptureRegion, setSelectorMode,
    encodingProgress, encodingStage, estimatedMbPerMin,
    audioLevels, startAudioMonitor, stopAudioMonitor, pollAudioLevels,
  } = useStore();

  const [elapsed, setElapsed] = useState("00:00:00");

  useEffect(() => {
    loadHistory();
    loadEstimatedSize();
    // Start audio level monitoring on mount
    startAudioMonitor();
    return () => { stopAudioMonitor(); };
  }, []);

  // Poll audio levels at ~20Hz
  useEffect(() => {
    const interval = setInterval(pollAudioLevels, 50);
    return () => clearInterval(interval);
  }, [pollAudioLevels]);

  // Stop monitor before recording, restart after
  useEffect(() => {
    if (recordingPhase === "recording") {
      stopAudioMonitor();
    } else if (recordingPhase === "idle" && config?.audio_enabled) {
      startAudioMonitor();
    }
  }, [recordingPhase]);

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
  const modeValue = config?.recording_mode === "FullScreen" ? "FULL" : "REGION";
  const encoderValue = config?.video_encoder || "H265";
  const qualityValue = config?.video_quality || "Medium";

  return (
    <div
      className="flex flex-col h-full relative"
      style={{
        background: "transparent",
        borderLeft: isRecording ? isPaused ? "3px solid var(--accent-warning)" : "3px solid var(--accent-record)" : "none",
        boxShadow: isRecording && !isPaused ? "inset 0 0 80px oklch(0.64 0.20 25 / 0.04)" : "none",
        transition: "border-color var(--duration-normal) var(--ease-out-expo), box-shadow var(--duration-slow) var(--ease-out-expo)",
      }}
    >
      {/* ═══ RECORDING ATMOSPHERE (red aurora shift) ═══ */}
      {isRecording && !isPaused && (
        <div className="fixed inset-0 pointer-events-none z-0" style={{ opacity: 0.6 }}>
          <div className="absolute rounded-full" style={{
            top: "-10%", right: "-10%", width: "40%", height: "40%",
            background: "oklch(0.64 0.20 25 / 0.04)",
            filter: "blur(120px)",
          }} />
        </div>
      )}
      {/* ═══ TOP BAR (minimal — nav is in sidebar) ═══ */}
      <header className="flex items-center justify-between px-6 py-2.5" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
        <div className="flex items-center gap-3">
          {isRecording ? (
            <div className="flex items-center gap-2 px-3 py-1.5" style={{ border: "var(--border-thin) solid var(--accent-record)", background: "oklch(0.64 0.20 25 / 0.08)", borderRadius: "var(--radius-sm)" }}>
              <span className="w-2 h-2 rounded-full animate-pulse-dot" style={{ background: "var(--accent-record)", boxShadow: "0 0 8px oklch(0.64 0.20 25 / 0.6)" }} />
              <span className="font-mono uppercase" style={{ color: "var(--accent-record)", fontSize: "0.6rem", letterSpacing: "0.08em", fontWeight: 700 }}>RECORDING</span>
            </div>
          ) : (
            <div className="flex items-center gap-2 px-3 py-1.5" style={{ border: "var(--border-thin) solid var(--border-default)", background: "var(--bg-surface)", borderRadius: "var(--radius-sm)" }}>
              <span className="w-2 h-2 rounded-full" style={{ background: "var(--accent-primary)", boxShadow: "0 0 8px oklch(0.78 0.18 160 / 0.6)" }} />
              <span className="font-mono uppercase" style={{ color: "var(--accent-primary)", fontSize: "0.6rem", letterSpacing: "0.08em", fontWeight: 700 }}>READY</span>
            </div>
          )}
          <span className="font-mono" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
            {config?.resolution_width}×{config?.resolution_height} · {config?.fps}fps
          </span>
        </div>
        <div className="flex items-center gap-3">
          {/* Audio meter moved to footer StatusBar */}
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
              className="flex items-center gap-4 px-6 py-3 shadow-md"
              style={{ 
                border: `var(--border-width) solid ${isPaused ? "var(--accent-warning)" : "var(--accent-record)"}`, 
                background: "var(--bg-surface)",
                borderRadius: "var(--radius-md)"
              }}
            >
              <motion.div
                className="w-3 h-3 rounded-full"
                style={{ 
                  background: isPaused ? "var(--accent-warning)" : "var(--accent-record)", 
                  boxShadow: isPaused ? "none" : "0 0 10px var(--accent-record-glow)"
                }}
                animate={isPaused ? {} : { opacity: [1, 0.4, 1], scale: [1, 0.92, 1] }}
                transition={{ duration: 1.5, repeat: Infinity, ease: "easeInOut" }}
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
              className="flex flex-col items-center gap-3 px-8 py-5 min-w-[280px] shadow-lg"
              style={{ border: "var(--border-width) solid var(--accent-info)", background: "var(--bg-surface)", borderRadius: "var(--radius-md)" }}
            >
              <div className="flex items-center gap-3">
                <motion.div animate={{ rotate: 360 }} transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }} className="w-4 h-4 border-2 border-t-transparent rounded-full" style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }} />
                <span className="font-mono text-xs font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>Encoding Video</span>
              </div>
              <div className="w-full h-1.5 overflow-hidden" style={{ background: "var(--bg-elevated)", borderRadius: "var(--radius-full)" }}>
                <motion.div
                  className="h-full"
                  style={{ background: "var(--accent-info)", borderRadius: "var(--radius-full)", width: `${encodingProgress}%` }}
                  animate={{ width: `${encodingProgress}%` }}
                  transition={{ duration: 0.3, ease: "easeOut" }}
                />
              </div>
              <div className="flex items-center justify-between w-full">
                <span className="font-mono text-[9px] font-bold" style={{ color: "var(--text-muted)" }}>
                  {encodingStage || "Processing..."}
                </span>
                <span className="font-mono text-[9px] font-bold" style={{ color: "var(--accent-info)" }}>
                  {encodingProgress}%
                </span>
              </div>
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
              className="flex items-center gap-4 px-5 py-3.5 cursor-pointer w-full max-w-md group shadow-md"
              style={{ border: "var(--border-width) solid var(--accent-success)", background: "var(--bg-surface)", borderRadius: "var(--radius-md)" }}
              whileHover={{ y: -2, borderColor: "var(--accent-info)", boxShadow: "var(--shadow-lg)" }}
            >
              <motion.div className="font-mono text-lg font-bold" style={{ color: "var(--accent-success)" }} initial={{ scale: 0 }} animate={{ scale: 1 }} transition={{ delay: 0.2, type: "spring", stiffness: 400, damping: 15 }}>✓</motion.div>
              <div className="flex-1 min-w-0">
                <div className="font-mono text-xs font-bold" style={{ color: "var(--text-primary)", letterSpacing: "0.03em" }}>
                  {lastRecording.duration_secs.toFixed(1)}s · {formatBytes(lastRecording.file_size_bytes)} · {lastRecording.has_audio ? "VIDEO+AUDIO" : "VIDEO"}
                </div>
                <div className="text-[10px] truncate mt-1 font-mono" style={{ color: "var(--text-muted)" }}>{normalizePath(lastRecording.output_path)}</div>
              </div>
              <motion.span className="font-mono text-[10px] font-bold uppercase" style={{ color: "var(--accent-info)" }} whileHover={{ x: 3 }}>OPEN →</motion.span>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ RECORD BUTTON ═══ */}
        <motion.button
          onClick={isRecording ? stopRecording : isIdle ? startRecording : undefined}
          disabled={isEncoding}
          className="relative flex items-center justify-center cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shadow-lg"
          style={{
            width: 110, height: 110,
            border: `3px solid ${isRecording ? "var(--accent-record)" : isEncoding ? "var(--accent-info)" : "var(--accent-primary)"}`,
            background: "var(--bg-surface)",
            borderRadius: "var(--radius-full)",
          }}
          animate={isRecording ? "breathe" : ""}
          variants={{
            breathe: {
              boxShadow: [
                "0 0 12px var(--accent-record-glow), inset 0 0 8px var(--accent-record-glow)",
                "0 0 28px var(--accent-record-glow), inset 0 0 16px var(--accent-record-glow)",
                "0 0 12px var(--accent-record-glow), inset 0 0 8px var(--accent-record-glow)"
              ],
              transition: { duration: 2, repeat: Infinity, ease: "easeInOut" }
            }
          }}
          whileHover={isIdle ? { scale: 1.05, y: -2, boxShadow: "var(--shadow-lg)" } : isRecording ? { scale: 1.03 } : {}}
          whileTap={!isEncoding ? { scale: 0.94 } : {}}
          transition={{ type: "spring", stiffness: 400, damping: 20 }}
        >
          <AnimatePresence mode="wait">
            {isRecording ? (
              <motion.div key="stop" initial={{ scale: 0, rotate: 90 }} animate={{ scale: 1, rotate: 0 }} exit={{ scale: 0, rotate: -90 }} transition={{ type: "spring", stiffness: 500, damping: 25 }} className="w-7 h-7" style={{ background: "var(--accent-record)", borderRadius: "var(--radius-xs)" }} />
            ) : isEncoding ? (
              <motion.div key="encoding" animate={{ rotate: 360 }} transition={{ duration: 2, repeat: Infinity, ease: "linear" }} className="w-7 h-7 border-3 border-t-transparent rounded-full" style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }} />
            ) : (
              <motion.div key="record" initial={{ scale: 0 }} animate={{ scale: 1 }} exit={{ scale: 0 }} transition={{ type: "spring", stiffness: 500, damping: 25 }} className="w-7 h-7 rounded-full" style={{ background: "var(--accent-primary)" }} />
            )}
          </AnimatePresence>
        </motion.button>

        {/* Pause Button */}
        <AnimatePresence>
          {isRecording && (
            <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 12 }} transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}>
              <motion.button
                onClick={isPaused ? resumeRecording : pauseRecording}
                className="px-5 py-2 font-mono text-[10px] font-bold uppercase cursor-pointer shadow-sm"
                style={{ border: "var(--border-width) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-primary)", borderRadius: "var(--radius-sm)", letterSpacing: "0.05em" }}
                whileHover={{ scale: 1.03, y: -1, borderColor: "var(--border-strong)" }}
                whileTap={{ scale: 0.96 }}
              >
                {isPaused ? "▶ Resume" : "⏸ Pause"}
              </motion.button>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ INLINE PRESET CARDS ═══ */}
        <motion.div
          className="grid grid-cols-3 gap-3 w-full max-w-xl overflow-visible"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: isRecording ? 0.5 : 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
          style={{ pointerEvents: isRecording ? "none" : "auto" }}
        >
          <PresetCard
            label="Resolution" value={resValue} index={0} disabled={isRecording}
            options={[
              { label: "480P (854×480)", value: "854×480" },
              { label: "720P (1280×720)", value: "1280×720" },
              { label: "1080P (1920×1080)", value: "1920×1080" },
            ]}
            onSelect={(v) => { updateField("resolution", v.replace("×", "x")); loadEstimatedSize(); }}
          />
          <PresetCard
            label="Frame Rate" value={fpsValue} index={1} disabled={isRecording}
            options={[
              { label: "24 FPS", value: "24" },
              { label: "30 FPS", value: "30" },
              { label: "60 FPS", value: "60" },
            ]}
            onSelect={(v) => { updateField("fps", Number(v)); loadEstimatedSize(); }}
          />
          <PresetCard
            label="Audio" value={audioValue} index={2} disabled={isRecording}
            options={[
              { label: "MICROPHONE", value: "Mic" },
              { label: "SYSTEM AUDIO", value: "System" },
              { label: "BOTH (MIC+SYS)", value: "Both" },
              { label: "OFF", value: "Off" },
            ]}
            onSelect={(v) => {
              if (v === "Off") {
                updateField("audio_enabled", false);
              } else {
                updateField("audio_enabled", true);
                updateField("audio_source", v);
              }
              loadEstimatedSize();
            }}
          />
          <PresetCard
            label="Encoder" value={encoderValue} index={3} disabled={isRecording}
            options={[
              { label: "AV1 — Best compression", value: "AV1" },
              { label: "AV1 NVENC — GPU (RTX 40+)", value: "AV1_NVENC" },
              { label: "H.265 — Great compression", value: "H265" },
              { label: "H.265 NVENC — GPU", value: "H265_NVENC" },
              { label: "H.264 — Fast, universal", value: "H264" },
              { label: "H.264 NVENC — GPU", value: "H264_NVENC" },
              { label: "VP9 — Web-friendly", value: "VP9" },
            ]}
            onSelect={(v) => { updateField("video_encoder", v); loadEstimatedSize(); }}
          />
          <PresetCard
            label="Quality" value={qualityValue} index={4} disabled={isRecording}
            options={[
              { label: "INSANE (~1 MB/min, AV1)", value: "Insane" },
              { label: "LOW (~3 MB/min)", value: "Low" },
              { label: "MEDIUM (~5 MB/min)", value: "Medium" },
              { label: "HIGH (~12 MB/min)", value: "High" },
              { label: "ULTRA (~25 MB/min)", value: "Ultra" },
            ]}
            onSelect={(v) => { updateField("video_quality", v); loadEstimatedSize(); }}
          />
          <PresetCard
            label="Mode" value={modeValue} index={5} disabled={isRecording}
            options={[
              { label: "FULLSCREEN", value: "FullScreen" },
              { label: "REGION SELECT", value: "Region" },
            ]}
            onSelect={(v) => updateField("recording_mode", v)}
          />
        </motion.div>

        {/* Estimated file size */}
        {isIdle && estimatedMbPerMin > 0 && (
          <motion.div
            className="font-mono text-[10px] px-3 py-1.5"
            style={{ color: "var(--text-muted)", border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", background: "var(--bg-surface)" }}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
          >
            ~{estimatedMbPerMin.toFixed(1)} MB/min · {config?.video_encoder} · {config?.video_quality}
          </motion.div>
        )}

        {/* Feature Badges */}
        <motion.div className="flex items-center gap-2" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.3 }}>
          {config?.auto_zoom_enabled && <Badge text="AUTO-ZOOM" />}
          {config?.cursor_trail_enabled && <Badge text="CURSOR-FX" />}
          {config?.webcam_enabled && <Badge text="WEBCAM" />}
        </motion.div>

        {/* Hotkey Hint */}
        {isIdle && (
          <motion.div className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.4 }}>
            <kbd className="px-2 py-1 font-mono text-[10px] font-bold" style={{ border: "var(--border-thin) solid var(--border-default)", background: "var(--bg-surface)", color: "var(--text-secondary)", borderRadius: "var(--radius-xs)", boxShadow: "var(--shadow-sm)" }}>
              {config?.hotkey_start || "Ctrl+Shift+R"}
            </kbd>
            <span className="ml-2 font-semibold">TO RECORD</span>
          </motion.div>
        )}

        {/* ═══ RECORDING HISTORY ═══ */}
        <HistoryPanel entries={history} onOpen={openPath} onClear={clearHistory} />
      </div>

      {/* ═══ STATUS BAR (Audio Visualization + System Info) ═══ */}
      <StatusBar
        audioSource={config?.audio_source || "Mic"}
        audioEnabled={config?.audio_enabled || false}
        levels={audioLevels}
        isRecording={isRecording}
        fps={config?.fps || 30}
        resolution={config ? `${config.resolution_width}×${config.resolution_height}` : "1920×1080"}
        sampleRate={config?.audio_sample_rate || 44100}
      />

      {/* ═══ REGION SELECTOR OVERLAY ═══ */}
      <AnimatePresence>
        {selectorMode === "region" && (
          <RegionSelector
            onComplete={(region) => setCaptureRegion(region)}
            onCancel={() => setSelectorMode("none")}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

function Badge({ text }: { text: string }) {
  return (
    <motion.span
      className="font-mono text-[9px] font-bold px-2 py-1"
      style={{ border: "var(--border-thin) solid var(--border-default)", color: "var(--text-secondary)", letterSpacing: "0.05em", borderRadius: "var(--radius-xs)", background: "var(--bg-surface)" }}
      whileHover={{ borderColor: "var(--accent-primary)", color: "var(--accent-primary)" }}
    >
      {text}
    </motion.span>
  );
}
