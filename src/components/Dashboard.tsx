import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore } from "../stores/recording";
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

export function Dashboard({ onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    config,
    recordingPhase,
    isPaused,
    recordingStartTime,
    lastRecording,
    startRecording,
    stopRecording,
    pauseRecording,
    resumeRecording,
    openPath,
  } = useStore();

  const { theme, toggleTheme } = useThemeStore();
  const [elapsed, setElapsed] = useState("00:00:00");
  const [version, setVersion] = useState("0.1.0");

  useEffect(() => {
    invoke<string>("get_version").then(setVersion).catch(() => {});
  }, []);

  useEffect(() => {
    if (recordingPhase !== "recording" || !recordingStartTime) return;
    const interval = setInterval(() => {
      setElapsed(formatDuration(Date.now() - recordingStartTime));
    }, 1000);
    return () => clearInterval(interval);
  }, [recordingPhase, recordingStartTime]);

  const isRecording = recordingPhase === "recording";
  const isEncoding = recordingPhase === "encoding";
  const isIdle = recordingPhase === "idle";

  const resolutionLabel = config ? `${config.resolution_width}×${config.resolution_height}` : "1920×1080";
  const fpsLabel = config ? `${config.fps} FPS` : "30 FPS";
  const audioLabel = config?.audio_enabled
    ? config.audio_source === "both" ? "MIC+SYS" : config.audio_source === "system" ? "SYSTEM" : "MIC"
    : "OFF";

  return (
    <div
      className="flex flex-col h-full relative"
      style={{
        background: "var(--bg-base)",
        borderLeft: isRecording ? "3px solid var(--accent-danger)" : "3px solid transparent",
        transition: "border-color var(--duration-normal) var(--ease-out-expo)",
      }}
    >
      {/* ═══ HEADER ═══ */}
      <header
        className="flex items-center justify-between px-6 py-3"
        style={{ borderBottom: "var(--border-width) solid var(--border-default)" }}
      >
        <div className="flex items-center gap-3">
          {/* Logo — brutalist block */}
          <motion.div
            className="flex items-center justify-center font-mono text-sm font-bold"
            style={{
              width: 36,
              height: 36,
              background: "var(--accent-primary)",
              color: "white",
              border: "var(--border-width) solid var(--border-strong)",
            }}
            whileHover={{ scale: 1.05, rotate: -2 }}
            whileTap={{ scale: 0.95 }}
          >
            ES
          </motion.div>
          <div className="flex flex-col">
            <span className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
              EASYSPECY
            </span>
            <span className="font-mono text-xs" style={{ color: "var(--text-muted)", letterSpacing: "0.05em" }}>
              V{version}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Theme Toggle */}
          <motion.button
            onClick={toggleTheme}
            className="flex items-center justify-center cursor-pointer"
            style={{
              width: 36,
              height: 36,
              border: "var(--border-width) solid var(--border-default)",
              background: "var(--bg-surface)",
              color: "var(--text-secondary)",
            }}
            whileHover={{ scale: 1.05, borderColor: "var(--border-strong)" }}
            whileTap={{ scale: 0.95 }}
            title={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
          >
            <motion.span
              key={theme}
              initial={{ rotate: -90, opacity: 0 }}
              animate={{ rotate: 0, opacity: 1 }}
              transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
              className="text-base"
            >
              {theme === "dark" ? "☀" : "●"}
            </motion.span>
          </motion.button>

          {/* Settings Button */}
          <motion.button
            onClick={onOpenSettings}
            disabled={isRecording || isEncoding}
            className="flex items-center gap-2 px-3 py-2 text-sm font-mono uppercase cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
            style={{
              border: "var(--border-width) solid var(--border-default)",
              background: "var(--bg-surface)",
              color: "var(--text-secondary)",
              letterSpacing: "0.05em",
              fontSize: "var(--text-xs)",
            }}
            whileHover={{ scale: 1.02, y: -1 }}
            whileTap={{ scale: 0.97 }}
          >
            CONFIG
          </motion.button>
        </div>
      </header>

      {/* ═══ MAIN CONTENT ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6 px-6">

        {/* Recording Timer */}
        <AnimatePresence>
          {isRecording && (
            <motion.div
              initial={{ opacity: 0, y: -20, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -20, scale: 0.95 }}
              transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
              className="flex items-center gap-4 px-6 py-3"
              style={{
                border: "var(--border-width) solid var(--accent-danger)",
                background: "var(--bg-surface)",
              }}
            >
              <motion.div
                className="w-3 h-3"
                style={{
                  background: isPaused ? "var(--accent-warning)" : "var(--accent-danger)",
                  animation: isPaused ? "none" : "pulse-dot 1.5s ease-in-out infinite",
                }}
              />
              <span
                className="font-mono text-2xl font-bold tracking-widest"
                style={{ color: "var(--text-primary)" }}
              >
                {elapsed}
              </span>
              {isPaused && (
                <motion.span
                  initial={{ opacity: 0, x: -8 }}
                  animate={{ opacity: 1, x: 0 }}
                  className="font-mono text-xs font-bold uppercase"
                  style={{ color: "var(--accent-warning)", letterSpacing: "0.1em" }}
                >
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
              style={{
                border: "var(--border-width) solid var(--accent-info)",
                background: "var(--bg-surface)",
              }}
            >
              <div className="flex items-center gap-3">
                <motion.div
                  animate={{ rotate: 360 }}
                  transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
                  className="w-4 h-4 border-2 border-t-transparent rounded-full"
                  style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }}
                />
                <span className="font-mono text-sm uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>
                  Encoding
                </span>
              </div>
              <div className="w-full h-1 overflow-hidden" style={{ background: "var(--bg-elevated)" }}>
                <motion.div
                  className="h-full w-1/3"
                  style={{ background: "var(--accent-info)" }}
                  animate={{ x: ["-100%", "300%"] }}
                  transition={{ duration: 1.8, repeat: Infinity, ease: "easeInOut" }}
                />
              </div>
              <span className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>
                MERGING VIDEO + AUDIO
              </span>
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
              style={{
                border: "var(--border-width) solid var(--accent-success)",
                background: "var(--bg-surface)",
              }}
            >
              <motion.div
                className="font-mono text-lg font-bold"
                style={{ color: "var(--accent-success)" }}
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                transition={{ delay: 0.2, type: "spring", stiffness: 400, damping: 15 }}
              >
                ✓
              </motion.div>
              <div className="flex-1 min-w-0">
                <div className="font-mono text-xs uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.03em" }}>
                  {lastRecording.duration_secs.toFixed(1)}s • {(lastRecording.file_size_bytes / 1_048_576).toFixed(1)}MB • {lastRecording.has_audio ? "VIDEO+AUDIO" : "VIDEO"}
                </div>
                <div className="text-xs truncate mt-1" style={{ color: "var(--text-muted)" }}>
                  {normalizePath(lastRecording.output_path)}
                </div>
              </div>
              <motion.span
                className="font-mono text-xs font-bold uppercase"
                style={{ color: "var(--accent-info)" }}
                whileHover={{ x: 3 }}
              >
                OPEN →
              </motion.span>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ RECORD BUTTON ═══ */}
        <motion.button
          onClick={isRecording ? stopRecording : isIdle ? startRecording : undefined}
          disabled={isEncoding}
          className="relative flex items-center justify-center cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          style={{
            width: 120,
            height: 120,
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
              <motion.div
                key="stop"
                initial={{ scale: 0, rotate: 90 }}
                animate={{ scale: 1, rotate: 0 }}
                exit={{ scale: 0, rotate: -90 }}
                transition={{ type: "spring", stiffness: 500, damping: 25 }}
                className="w-8 h-8"
                style={{ background: "var(--accent-danger)" }}
              />
            ) : isEncoding ? (
              <motion.div
                key="encoding"
                animate={{ rotate: 360 }}
                transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                className="w-8 h-8 border-3 border-t-transparent rounded-full"
                style={{ borderColor: "var(--accent-info)", borderTopColor: "transparent" }}
              />
            ) : (
              <motion.div
                key="record"
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                exit={{ scale: 0 }}
                transition={{ type: "spring", stiffness: 500, damping: 25 }}
                className="w-8 h-8 rounded-full"
                style={{ background: "var(--accent-primary)" }}
              />
            )}
          </AnimatePresence>
        </motion.button>

        {/* Action Buttons */}
        <AnimatePresence>
          {isRecording && (
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 12 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              className="flex items-center gap-3"
            >
              <motion.button
                onClick={isPaused ? resumeRecording : pauseRecording}
                className="px-5 py-2 font-mono text-xs uppercase cursor-pointer"
                style={{
                  border: "var(--border-width) solid var(--border-default)",
                  background: "var(--bg-surface)",
                  color: "var(--text-primary)",
                  letterSpacing: "0.05em",
                }}
                whileHover={{ scale: 1.03, y: -1 }}
                whileTap={{ scale: 0.96 }}
              >
                {isPaused ? "▶ RESUME" : "⏸ PAUSE"}
              </motion.button>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ═══ STATUS GRID ═══ */}
        <motion.div
          className="grid grid-cols-4 gap-3 w-full max-w-xl mt-2"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
        >
          <StatusCard label="RES" value={resolutionLabel} index={0} />
          <StatusCard label="FPS" value={fpsLabel} index={1} />
          <StatusCard label="AUDIO" value={audioLabel} index={2} />
          <StatusCard label="MODE" value={config?.recording_mode?.toUpperCase() || "FULL"} index={3} />
        </motion.div>

        {/* Feature Badges */}
        <motion.div
          className="flex items-center gap-2"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.3 }}
        >
          {config?.auto_zoom_enabled && <Badge text="AUTO-ZOOM" />}
          {config?.cursor_trail_enabled && <Badge text="CURSOR-FX" />}
          {config?.webcam_enabled && <Badge text="WEBCAM" />}
        </motion.div>

        {/* Hotkey Hint */}
        {isIdle && (
          <motion.div
            className="font-mono text-xs"
            style={{ color: "var(--text-muted)" }}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4 }}
          >
            <kbd
              className="px-2 py-1 font-mono text-xs"
              style={{
                border: "var(--border-thin) solid var(--border-default)",
                background: "var(--bg-surface)",
                color: "var(--text-secondary)",
              }}
            >
              {config?.hotkey_start || "Ctrl+Shift+R"}
            </kbd>
            <span className="ml-2">TO RECORD</span>
          </motion.div>
        )}
      </div>

      {/* ═══ FOOTER ═══ */}
      <footer
        className="px-6 py-2.5 flex items-center justify-between"
        style={{ borderTop: "var(--border-width) solid var(--border-default)" }}
      >
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
          PHASE 1
        </span>
      </footer>
    </div>
  );
}

/* ─── Status Card ─── */
function StatusCard({ label, value, index }: { label: string; value: string; index: number }) {
  return (
    <motion.div
      className="text-center px-2 py-3"
      style={{
        border: "var(--border-width) solid var(--border-default)",
        background: "var(--bg-surface)",
      }}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.15 + index * 0.05, duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
      whileHover={{ y: -2, borderColor: "var(--border-strong)" }}
    >
      <div
        className="font-mono text-xs uppercase"
        style={{ color: "var(--text-muted)", letterSpacing: "0.1em", fontSize: "0.6rem" }}
      >
        {label}
      </div>
      <div
        className="font-mono text-sm font-bold mt-1"
        style={{ color: "var(--text-primary)" }}
      >
        {value}
      </div>
    </motion.div>
  );
}

/* ─── Badge ─── */
function Badge({ text }: { text: string }) {
  return (
    <motion.span
      className="font-mono text-xs px-2 py-1"
      style={{
        border: "var(--border-thin) solid var(--border-default)",
        color: "var(--text-secondary)",
        letterSpacing: "0.05em",
        fontSize: "0.6rem",
      }}
      whileHover={{ borderColor: "var(--accent-primary)", color: "var(--accent-primary)" }}
    >
      {text}
    </motion.span>
  );
}
