import { useEffect, useState } from "react";
import { useStore } from "../stores/recording";
import { invoke } from "@tauri-apps/api/core";

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  return `${h.toString().padStart(2, "0")}:${(m % 60)
    .toString()
    .padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;
}

export function Dashboard({ onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    config,
    isRecording,
    isPaused,
    recordingStartTime,
    startRecording,
    stopRecording,
    pauseRecording,
  } = useStore();

  const [elapsed, setElapsed] = useState("00:00:00");
  const [version, setVersion] = useState("0.1.0");

  useEffect(() => {
    invoke<string>("get_version").then(setVersion).catch(() => {});
  }, []);

  useEffect(() => {
    if (!isRecording || !recordingStartTime) return;
    const interval = setInterval(() => {
      setElapsed(formatDuration(Date.now() - recordingStartTime));
    }, 1000);
    return () => clearInterval(interval);
  }, [isRecording, recordingStartTime]);

  const resolutionLabel = config
    ? `${config.resolution_width}×${config.resolution_height}`
    : "1920×1080";
  const fpsLabel = config ? `${config.fps} fps` : "30 fps";

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-[#21262d]">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-md bg-gradient-to-br from-[#3fb950] to-[#238636] flex items-center justify-center text-sm font-bold text-white shadow-md">
            ES
          </div>
          <div>
            <span className="text-base font-semibold text-[#e6edf3]">EasySpecy</span>
            <span className="text-xs text-[#484f58] ml-2">v{version}</span>
          </div>
        </div>
        <button
          onClick={onOpenSettings}
          className="flex items-center gap-2 px-3 py-1.5 text-sm text-[#8b949e] hover:text-[#e6edf3] bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded-md transition-all"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
          Settings
        </button>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6 px-6">
        {/* Recording Status */}
        {isRecording && (
          <div className="flex items-center gap-3 px-5 py-2.5 rounded-lg bg-[#161b22] border border-[#30363d]">
            <div className={`w-2.5 h-2.5 rounded-full ${isPaused ? "bg-[#d29922]" : "bg-[#f85149] animate-pulse"}`} />
            <span className="font-mono text-2xl text-[#e6edf3] tracking-wider">{elapsed}</span>
            {isPaused && <span className="text-xs text-[#d29922] font-medium ml-1">PAUSED</span>}
          </div>
        )}

        {/* Record Button */}
        <button
          onClick={isRecording ? stopRecording : startRecording}
          className={`
            group relative w-28 h-28 rounded-full flex items-center justify-center
            transition-all duration-200 cursor-pointer shadow-lg
            ${isRecording
              ? "bg-[#f85149] hover:bg-[#da3633] shadow-[0_0_30px_rgba(248,81,73,0.3)]"
              : "bg-[#3fb950] hover:bg-[#238636] shadow-[0_0_30px_rgba(63,185,80,0.3)] hover:scale-105"
            }
          `}
        >
          {isRecording ? (
            <div className="w-7 h-7 rounded-sm bg-white" />
          ) : (
            <div className="w-7 h-7 rounded-full bg-white ml-1" />
          )}
        </button>

        {/* Action Buttons Row */}
        <div className="flex items-center gap-3">
          {isRecording && (
            <button
              onClick={pauseRecording}
              className="px-5 py-2 text-sm font-medium border border-[#30363d] rounded-md bg-[#21262d] hover:bg-[#30363d] text-[#e6edf3] transition-all"
            >
              {isPaused ? "▶ Resume" : "⏸ Pause"}
            </button>
          )}
        </div>

        {/* Settings Summary Cards */}
        <div className="grid grid-cols-3 gap-3 w-full max-w-lg mt-2">
          <Card label="Resolution" value={resolutionLabel} />
          <Card label="Frame Rate" value={fpsLabel} />
          <Card label="Audio" value={config?.audio_enabled ? `${config.audio_sample_rate} Hz` : "Off"} />
        </div>

        {/* Features indicator */}
        <div className="flex items-center gap-4 text-xs text-[#484f58]">
          {config?.auto_zoom_enabled && <Badge text="Auto-Zoom" color="blue" />}
          {config?.cursor_trail_enabled && <Badge text="Cursor Trail" color="purple" />}
          {config?.webcam_enabled && <Badge text="Webcam" color="green" />}
          <Badge text={config?.recording_mode || "FullScreen"} color="gray" />
        </div>

        {/* Hotkey hint */}
        {!isRecording && (
          <div className="text-xs text-[#484f58] mt-2">
            Press{" "}
            <kbd className="px-2 py-0.5 rounded bg-[#21262d] border border-[#30363d] text-[#8b949e] font-mono text-xs">
              {config?.hotkey_start || "Ctrl+Shift+R"}
            </kbd>{" "}
            to start recording
          </div>
        )}
      </div>

      {/* Footer */}
      <Footer outputDir={config?.output_dir || "~/Videos/EasySpecy"} />
    </div>
  );
}

function Card({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-center px-3 py-3 rounded-lg bg-[#161b22] border border-[#21262d]">
      <div className="text-[10px] text-[#484f58] uppercase tracking-wider">{label}</div>
      <div className="text-sm font-semibold text-[#e6edf3] mt-1">{value}</div>
    </div>
  );
}

function Badge({ text, color }: { text: string; color: string }) {
  const colors: Record<string, string> = {
    blue: "bg-[#1f3a5f] text-[#58a6ff] border-[#1f3a5f]",
    purple: "bg-[#2d1f5e] text-[#bc8cff] border-[#2d1f5e]",
    green: "bg-[#0d2818] text-[#3fb950] border-[#0d2818]",
    gray: "bg-[#21262d] text-[#8b949e] border-[#30363d]",
  };
  return (
    <span className={`px-2 py-0.5 rounded-full text-[10px] font-medium border ${colors[color] || colors.gray}`}>
      {text}
    </span>
  );
}

function Footer({ outputDir }: { outputDir: string }) {
  const openPath = useStore((s) => s.openPath);
  return (
    <div className="px-6 py-2.5 border-t border-[#21262d] text-xs text-[#484f58] flex items-center justify-between">
      <button
        onClick={() => openPath(outputDir)}
        className="hover:text-[#58a6ff] transition-colors cursor-pointer flex items-center gap-1"
      >
        📁 {outputDir}
        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" /></svg>
      </button>
      <span>Phase 0 — UI Preview</span>
    </div>
  );
}
