import { useEffect, useState, useRef } from "react";
import { useShallow } from "zustand/react/shallow";
import { motion, AnimatePresence } from "motion/react";
import { useStore, type RecordingEntry } from "../stores/recording";
import { RegionSelector, WindowPicker } from "./RegionSelector";
import { AudioPanel } from "./AudioPanel";
import { PreviewModal } from "./PreviewModal";
import { MAIN_ENCODERS } from "../lib/encoders";
import { QualityPreview } from "./QualityPreview";
import { Footer } from "./StatusBar";
import { useRecordingContextMenu } from "./ContextMenu";

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
  const recordingCtx = useRecordingContextMenu();
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
            onContextMenu={(e) => recordingCtx.onContextMenu(e, entry)}
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
  // Shallow-select what this page uses — a bare useStore() re-rendered it on every meter poll
  const {
    config, recordingPhase, isPaused, recordingStartTime, lastRecording, history, startRecording, stopRecording, pauseRecording, resumeRecording, openPath, updateField, loadHistory, clearHistory, loadEstimatedSize, selectorMode, setCaptureRegion, setSelectorMode, regionShot, windows, recordWindow, encodingProgress, encodingStage, estimatedMbPerMin, pausedMs, pauseStartTime, startPreview, loadEncoderScan,
  } = useStore(useShallow((s) => ({
    config: s.config, recordingPhase: s.recordingPhase, isPaused: s.isPaused, recordingStartTime: s.recordingStartTime, lastRecording: s.lastRecording, history: s.history, startRecording: s.startRecording, stopRecording: s.stopRecording, pauseRecording: s.pauseRecording, resumeRecording: s.resumeRecording, openPath: s.openPath, updateField: s.updateField, loadHistory: s.loadHistory, clearHistory: s.clearHistory, loadEstimatedSize: s.loadEstimatedSize, selectorMode: s.selectorMode, setCaptureRegion: s.setCaptureRegion, setSelectorMode: s.setSelectorMode, regionShot: s.regionShot, windows: s.windows, recordWindow: s.recordWindow, encodingProgress: s.encodingProgress, encodingStage: s.encodingStage, estimatedMbPerMin: s.estimatedMbPerMin, pausedMs: s.pausedMs, pauseStartTime: s.pauseStartTime, startPreview: s.startPreview, loadEncoderScan: s.loadEncoderScan,
  })));

  // The WebView flags that keep the overlay rendering in the background also stop Chromium
  // from throttling this window while it's hidden in the tray — so stop decorative animation
  // ourselves (it cost ~17% of a core during every recording).
  const visible = useDocumentVisible();
  const [elapsed, setElapsed] = useState("00:00:00");
  const [pauseElapsed, setPauseElapsed] = useState("00:00:00");

  useEffect(() => {
    loadHistory();
    loadEstimatedSize();
    loadEncoderScan();
  }, []);

  useEffect(() => {
    if (recordingPhase !== "recording" || !recordingStartTime) return;

    const tick = () => {
      if (!useStore.getState().mainVisible) return; // hidden in the tray while recording
      if (isPaused) {
        const currentPauseStart = pauseStartTime || Date.now();
        const activeDuration = currentPauseStart - recordingStartTime - pausedMs;
        setElapsed(formatDuration(activeDuration));

        const currentPauseDuration = Date.now() - currentPauseStart;
        setPauseElapsed(formatDuration(currentPauseDuration));
      } else {
        const activeDuration = Date.now() - recordingStartTime - pausedMs;
        setElapsed(formatDuration(activeDuration));
        setPauseElapsed("00:00:00");
      }
    };

    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, [recordingPhase, recordingStartTime, pausedMs, isPaused, pauseStartTime, visible]); // `visible`: re-tick the moment it's shown

  const isRecording = recordingPhase === "recording";
  const isEncoding = recordingPhase === "encoding";
  const isIdle = recordingPhase === "idle";
  const isStarting = recordingPhase === "starting";

  const fpsValue = config ? `${config.fps}` : "30";
  const modeValue = config?.recording_mode === "Window" ? "WINDOW" : config?.recording_mode === "Region" ? "REGION" : "FULL";
  const ENC_NAME: Record<string, string> = { H264: "H.264", H265: "H.265", AV1: "AV1 (adv.)", VP9: "VP9 (adv.)", MobileShareable: "Mobile (adv.)", H264_NVENC: "H.264 GPU", H265_NVENC: "H.265 GPU", AV1_NVENC: "AV1 GPU" };
  const encoderValue = ENC_NAME[config?.video_encoder || "H264"] ?? config?.video_encoder;
  const qualityValue = config?.video_quality || "Medium";
  const [showQuality, setShowQuality] = useState(false);

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
        <div className="fixed inset-0 pointer-events-none z-0" style={{
          background: "radial-gradient(40% 40% at 85% 5%, oklch(0.64 0.20 25 / 0.05), transparent 70%)",
        }} />
      )}
      {/* ═══ TOP BAR (minimal — nav is in sidebar) ═══ */}
      <header className="flex items-center justify-between px-6 py-2.5" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
        <div className="flex items-center gap-3">
          {isRecording ? (
            <div className="flex items-center gap-2 px-3 py-1.5" style={{ border: "var(--border-thin) solid var(--accent-record)", background: "oklch(0.64 0.20 25 / 0.08)", borderRadius: "var(--radius-sm)" }}>
              <span className={`w-2 h-2 rounded-full ${visible ? "animate-pulse-dot" : ""}`} style={{ background: "var(--accent-record)", boxShadow: "0 0 8px oklch(0.64 0.20 25 / 0.6)" }} />
              <span className="font-mono uppercase" style={{ color: "var(--accent-record)", fontSize: "0.6rem", letterSpacing: "0.08em", fontWeight: 700 }}>RECORDING</span>
            </div>
          ) : (
            <div className="flex items-center gap-2 px-3 py-1.5" style={{ border: "var(--border-thin) solid var(--border-default)", background: "var(--bg-surface)", borderRadius: "var(--radius-sm)" }}>
              <span className="w-2 h-2 rounded-full" style={{ background: "var(--accent-primary)", boxShadow: "0 0 8px oklch(0.78 0.18 160 / 0.6)" }} />
              <span className="font-mono uppercase" style={{ color: "var(--accent-primary)", fontSize: "0.6rem", letterSpacing: "0.08em", fontWeight: 700 }}>READY</span>
            </div>
          )}
          <span className="font-mono" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
            {config?.recording_mode === "FullScreen" ? "Full screen" : config?.recording_mode} · {config?.fps}fps
          </span>
        </div>
        <div className="flex items-center gap-3">
          {/* Audio meter moved to footer StatusBar */}
        </div>
      </header>

      {/* ═══ MAIN CONTENT ═══ */}
      {/* "safe center": plain centring pushed overflowing content above the scroll origin, out of reach */}
      <div className="flex-1 flex flex-col items-center gap-5 px-6 overflow-y-auto py-4" style={{ justifyContent: "safe center" }}>

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
                animate={isPaused || !visible ? {} : { opacity: [1, 0.4, 1], scale: [1, 0.92, 1] }}
                transition={{ duration: 1.5, repeat: Infinity, ease: "easeInOut" }}
              />
              <span className="font-mono text-2xl font-bold tracking-widest" style={{ color: isPaused ? "var(--text-muted)" : "var(--text-primary)" }}>
                {elapsed}
              </span>
              {isPaused && (
                <>
                  <span style={{ width: "1px", height: "20px", background: "var(--border-default)" }} />
                  <div className="flex items-center gap-2">
                    <motion.span 
                      className="w-2 h-2 rounded-full" 
                      style={{ 
                        background: "var(--accent-warning)", 
                        boxShadow: "0 0 6px var(--accent-warning)" 
                      }}
                      animate={visible ? { opacity: [1, 0.4, 1] } : {}}
                      transition={{ duration: 1.0, repeat: Infinity, ease: "easeInOut" }}
                    />
                    <span className="font-mono text-lg font-bold" style={{ color: "var(--accent-warning)" }}>
                      {pauseElapsed}
                    </span>
                    <span className="font-mono text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5" style={{ color: "var(--accent-warning)", border: "1px solid var(--accent-warning)", borderRadius: "var(--radius-xs)" }}>
                      PAUSED
                    </span>
                  </div>
                </>
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


        {/* ═══ RECORD BUTTON ═══ */}
        <motion.button
          onClick={isRecording ? stopRecording : isIdle ? startRecording : undefined}
          disabled={isEncoding || isStarting}
          aria-label={isRecording ? "Stop recording" : isStarting ? "Starting recording" : isEncoding ? "Saving recording" : "Start recording"}
          aria-busy={isStarting || isEncoding}
          className="relative flex items-center justify-center cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed shadow-lg"
          style={{
            width: 110, height: 110,
            border: `3px solid ${isRecording ? "var(--accent-record)" : isEncoding ? "var(--accent-info)" : "var(--accent-primary)"}`,
            background: "var(--bg-surface)",
            borderRadius: "var(--radius-full)",
          }}

          whileHover={isIdle ? { scale: 1.05, y: -2, boxShadow: "var(--shadow-lg)" } : isRecording ? { scale: 1.03 } : {}}
          whileTap={!isEncoding ? { scale: 0.94 } : {}}
          transition={{ type: "spring", stiffness: 400, damping: 20 }}
        >
          {/* Recording pulse: static glow ring, only opacity animates (box-shadow keyframes
              repainted every frame for the whole recording) */}
          {isRecording && !isPaused && visible && (
            <motion.span
              aria-hidden
              className="absolute inset-[-3px] rounded-full pointer-events-none"
              style={{ boxShadow: "0 0 28px var(--accent-record-glow), inset 0 0 16px var(--accent-record-glow)" }}
              animate={{ opacity: [0.35, 1, 0.35] }}
              transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
            />
          )}
          <AnimatePresence mode="wait">
            {isRecording ? (
              <motion.div key="stop" initial={{ scale: 0, rotate: 90 }} animate={{ scale: 1, rotate: 0 }} exit={{ scale: 0, rotate: -90 }} transition={{ type: "spring", stiffness: 500, damping: 25 }} className="w-7 h-7" style={{ background: "var(--accent-record)", borderRadius: "var(--radius-xs)" }} />
            ) : isEncoding || isStarting ? (
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

        {/* Preview: 10 s throwaway recording, played back in the app */}
        {isIdle && (
          <button
            type="button"
            onClick={startPreview}
            className="px-4 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer -mt-2 hover:text-[var(--text-primary)] focus-visible:outline-2 focus-visible:outline-offset-2"
            style={{ border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", color: "var(--text-secondary)", letterSpacing: "0.05em" }}
            title="Records 10 seconds to a temporary file and plays it back, so you can check webcam, cursor and audio"
          >
            ▶ Preview 10s
          </button>
        )}

        {/* Last Recording — stays until the next recording, with the actions people want next */}
        <AnimatePresence>
          {lastRecording && isIdle && <SavedCard rec={lastRecording} />}
        </AnimatePresence>

        {/* ═══ INLINE PRESET CARDS ═══ */}
        <motion.div
          className="grid grid-cols-2 sm:grid-cols-4 gap-3 w-full max-w-xl overflow-visible"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: isRecording ? 0.5 : 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
          style={{ pointerEvents: isRecording ? "none" : "auto" }}
        >
          <PresetCard
            label="Mode" value={modeValue} index={0} disabled={isRecording}
            options={[
              { label: "FULLSCREEN", value: "FullScreen" },
              { label: "REGION SELECT", value: "Region" },
              { label: "WINDOW", value: "Window" },
            ]}
            onSelect={(v) => updateField("recording_mode", v)}
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
            label="Encoder" value={encoderValue} index={3} disabled={isRecording}
            options={MAIN_ENCODERS /* AV1 / VP9 / GPU encoders: Settings → Video */}
            onSelect={(v) => { updateField("video_encoder", v); loadEstimatedSize(); }}
          />
          <PresetCard
            label="Quality" value={qualityValue} index={2} disabled={isRecording}
            options={[
              { label: "LOW", value: "Low" },
              { label: "MEDIUM", value: "Medium" },
              { label: "HIGH (default)", value: "High" },
              { label: "ULTRA", value: "Ultra" },
              { label: "👁 SEE THE DIFFERENCE…", value: "__compare" },
            ]}
            onSelect={(v) => { if (v === "__compare") { setShowQuality(true); return; } updateField("video_quality", v); loadEstimatedSize(); }}
          />
        </motion.div>

        {/* Smaller file: quality-based re-encode after stopping instead of the live stream */}
        {config && (
          <label className="w-full max-w-xl flex items-center gap-3 px-4 py-2.5 cursor-pointer"
            style={{ border: `var(--border-thin) solid ${config.compact_output ? "var(--accent-primary)" : "var(--border-default)"}`, borderRadius: "var(--radius-md)", background: "var(--bg-surface)", opacity: isIdle ? 1 : 0.5 }}>
            <input type="checkbox" className="accent-[var(--accent-primary)] w-4 h-4" checked={config.compact_output} disabled={!isIdle}
              onChange={(e) => { updateField("compact_output", e.target.checked); loadEstimatedSize(); }} />
            <span className="flex-1">
              <span className="font-mono text-[11px] font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>Smaller file</span>
              <span className="block text-[11px]" style={{ color: "var(--text-muted)" }}>Smaller video, takes longer to save</span>
            </span>
            <button type="button" onClick={(e) => { e.preventDefault(); setShowQuality(true); }} className="font-mono text-[10px] underline cursor-pointer shrink-0" style={{ color: "var(--text-secondary)" }}>
              Compare quality
            </button>
          </label>
        )}
        {showQuality && <QualityPreview onClose={() => { setShowQuality(false); loadEstimatedSize(); }} />}

        {/* What audio gets recorded — visible before you hit record */}
        <AudioPanel disabled={!isIdle} />

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

      {/* ═══ FOOTER ═══ */}
      <Footer isRecording={isRecording} />

      <PreviewModal />

      {/* ═══ REGION SELECTOR OVERLAY ═══ */}
      <AnimatePresence>
        {selectorMode === "region" && (
          <RegionSelector
            shot={regionShot}
            onComplete={(region) => setCaptureRegion(region)}
            onCancel={() => setSelectorMode("none")}
          />
        )}
        {selectorMode === "window" && (
          <WindowPicker
            windows={windows}
            onPick={(i) => recordWindow(windows[i])}
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

function SavedCard({ rec }: { rec: { output_path: string; duration_secs: number; file_size_bytes: number; has_audio: boolean; audio_silent: boolean } }) {
  const { openPath, revealInExplorer, copyToClipboard, discardLastRecording } = useStore(useShallow((s) => ({
    openPath: s.openPath, revealInExplorer: s.revealInExplorer, copyToClipboard: s.copyToClipboard, discardLastRecording: s.discardLastRecording,
  })));
  const [confirmDelete, setConfirmDelete] = useState(false);
  const name = normalizePath(rec.output_path).split(/[\\/]/).pop();
  const action = "px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer transition-colors focus-visible:outline-2 focus-visible:outline-offset-2";
  const actionStyle = { border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", color: "var(--text-secondary)", letterSpacing: "0.05em" };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 8 }}
      transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
      className="w-full max-w-md px-5 py-4 shadow-md"
      style={{ border: "var(--border-width) solid var(--accent-success)", background: "var(--bg-surface)", borderRadius: "var(--radius-md)" }}
      role="status"
      aria-label="Recording saved"
    >
      <div className="flex items-center gap-3">
        <span className="font-mono text-lg font-bold" style={{ color: "var(--accent-success)" }} aria-hidden>✓</span>
        <div className="flex-1 min-w-0">
          <div className="font-mono text-xs font-bold" style={{ color: "var(--text-primary)", letterSpacing: "0.03em" }}>
            Saved · {rec.duration_secs.toFixed(1)}s · {formatBytes(rec.file_size_bytes)}{!rec.has_audio ? " · no audio" : rec.audio_silent ? " · audio was silent" : ""}
          </div>
          <div className="text-[10px] truncate mt-1 font-mono" style={{ color: "var(--text-muted)" }} title={normalizePath(rec.output_path)}>{name}</div>
        </div>
      </div>
      <div className="flex flex-wrap gap-2 mt-3">
        <button className={action} onClick={() => openPath(rec.output_path)}
          style={{ ...actionStyle, background: "var(--accent-primary)", color: "#003d22", borderColor: "var(--accent-primary)" }}>
          ▶ Open
        </button>
        <button className={`${action} hover:text-[var(--text-primary)]`} style={actionStyle} onClick={() => revealInExplorer(rec.output_path)}>
          Show in folder
        </button>
        <button className={`${action} hover:text-[var(--text-primary)]`} style={actionStyle} onClick={() => copyToClipboard(rec.output_path)}>
          Copy path
        </button>
        <button
          className={`${action} ml-auto`}
          style={{ ...actionStyle, color: "var(--accent-danger)", borderColor: confirmDelete ? "var(--accent-danger)" : "var(--border-default)" }}
          onClick={() => (confirmDelete ? discardLastRecording() : setConfirmDelete(true))}
          onBlur={() => setConfirmDelete(false)}
        >
          {confirmDelete ? "Confirm delete" : "Delete"}
        </button>
      </div>
    </motion.div>
  );
}

function useDocumentVisible() {
  return useStore((s) => s.mainVisible);
}
