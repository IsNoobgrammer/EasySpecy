import { useEffect, useState } from "react";
import { useStore } from "../stores/recording";
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
  const fpsLabel = config ? `${config.fps} fps` : "30 fps";
  const audioLabel = config?.audio_enabled
    ? config.audio_source === "both" ? "Mic + System" : config.audio_source === "system" ? "System" : "Mic"
    : "Off";

  return (
    <div className="flex flex-col h-full bg-[#0d1117]">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-[#21262d]">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-md bg-gradient-to-br from-[#3fb950] to-[#238636] flex items-center justify-center text-sm font-bold text-white shadow-md">ES</div>
          <div>
            <span className="text-base font-semibold text-[#e6edf3]">EasySpecy</span>
            <span className="text-xs text-[#484f58] ml-2">v{version}</span>
          </div>
        </div>
        <button
          onClick={onOpenSettings}
          disabled={isRecording || isEncoding}
          className="flex items-center gap-2 px-3 py-1.5 text-sm text-[#8b949e] hover:text-[#e6edf3] bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded-md transition-all disabled:opacity-40 disabled:cursor-not-allowed"
        >
          ⚙ Settings
        </button>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6 px-6">

        {/* Recording Timer */}
        {isRecording && (
          <div className="flex items-center gap-3 px-5 py-2.5 rounded-lg bg-[#161b22] border border-[#30363d]">
            <div className={`w-2.5 h-2.5 rounded-full ${isPaused ? "bg-[#d29922]" : "bg-[#f85149] animate-pulse"}`} />
            <span className="font-mono text-2xl text-[#e6edf3] tracking-wider">{elapsed}</span>
            {isPaused && <span className="text-xs text-[#d29922] font-medium ml-1">PAUSED</span>}
          </div>
        )}

        {/* Encoding Progress */}
        {isEncoding && (
          <div className="flex flex-col items-center gap-3 px-8 py-5 rounded-lg bg-[#161b22] border border-[#30363d] min-w-[280px]">
            <div className="flex items-center gap-3">
              <svg className="animate-spin w-5 h-5 text-[#58a6ff]" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              <span className="text-sm text-[#e6edf3] font-medium">Encoding video...</span>
            </div>
            <div className="w-full bg-[#21262d] rounded-full h-1.5 overflow-hidden">
              <div className="h-full bg-[#58a6ff] rounded-full animate-[progress_2s_ease-in-out_infinite]" style={{
                animation: "progress 2s ease-in-out infinite",
                width: "60%",
              }} />
            </div>
            <span className="text-xs text-[#484f58]">Merging video + audio, please wait...</span>
          </div>
        )}

        {/* Last Recording */}
        {lastRecording && isIdle && (
          <div
            className="flex items-center gap-4 px-5 py-3 rounded-lg bg-[#0d2818] border border-[#238636] cursor-pointer hover:bg-[#123322] transition-colors w-full max-w-md"
            onClick={() => openPath(lastRecording.output_path)}
          >
            <div className="text-[#3fb950] text-lg">✓</div>
            <div className="flex-1 min-w-0">
              <div className="text-sm text-[#e6edf3] font-medium">
                {lastRecording.duration_secs.toFixed(1)}s • {(lastRecording.file_size_bytes / 1_048_576).toFixed(1)}MB • {lastRecording.has_audio ? "video+audio" : "video only"}
              </div>
              <div className="text-xs text-[#8b949e] truncate">{normalizePath(lastRecording.output_path)}</div>
            </div>
            <div className="text-xs text-[#58a6ff] whitespace-nowrap">Open →</div>
          </div>
        )}

        {/* Record Button */}
        <button
          onClick={isRecording ? stopRecording : isIdle ? startRecording : undefined}
          disabled={isEncoding}
          className={`
            group relative w-28 h-28 rounded-full flex items-center justify-center
            transition-all duration-200 cursor-pointer shadow-lg disabled:opacity-50 disabled:cursor-not-allowed
            ${isRecording
              ? "bg-[#f85149] hover:bg-[#da3633] shadow-[0_0_30px_rgba(248,81,73,0.3)]"
              : isEncoding
                ? "bg-[#58a6ff]"
                : "bg-[#3fb950] hover:bg-[#238636] shadow-[0_0_30px_rgba(63,185,80,0.3)] hover:scale-105"
            }
          `}
        >
          {isRecording ? (
            <div className="w-7 h-7 rounded-sm bg-white" />
          ) : isEncoding ? (
            <svg className="animate-spin w-8 h-8 text-white" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          ) : (
            <div className="w-7 h-7 rounded-full bg-white ml-1" />
          )}
        </button>

        {/* Action Buttons */}
        <div className="flex items-center gap-3">
          {isRecording && (
            <button
              onClick={isPaused ? resumeRecording : pauseRecording}
              className="px-5 py-2 text-sm font-medium border border-[#30363d] rounded-md bg-[#21262d] hover:bg-[#30363d] text-[#e6edf3] transition-all"
            >
              {isPaused ? "▶ Resume" : "⏸ Pause"}
            </button>
          )}
        </div>

        {/* Settings Summary Cards */}
        <div className="grid grid-cols-4 gap-3 w-full max-w-xl mt-2">
          <Card label="Resolution" value={resolutionLabel} />
          <Card label="FPS" value={fpsLabel} />
          <Card label="Audio" value={audioLabel} />
          <Card label="Mode" value={config?.recording_mode || "FullScreen"} />
        </div>

        {/* Feature badges */}
        <div className="flex items-center gap-2 text-xs text-[#484f58]">
          {config?.auto_zoom_enabled && <Badge text="Auto-Zoom" color="blue" />}
          {config?.cursor_trail_enabled && <Badge text="Cursor Trail" color="purple" />}
          {config?.webcam_enabled && <Badge text="Webcam" color="green" />}
        </div>

        {/* Hotkey hint */}
        {isIdle && (
          <div className="text-xs text-[#484f58]">
            Press <kbd className="px-1.5 py-0.5 rounded bg-[#21262d] border border-[#30363d] text-[#8b949e] font-mono text-xs">{config?.hotkey_start || "Ctrl+Shift+R"}</kbd> to record
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
    <div className="text-center px-2 py-3 rounded-lg bg-[#161b22] border border-[#21262d]">
      <div className="text-[10px] text-[#484f58] uppercase tracking-wider">{label}</div>
      <div className="text-sm font-semibold text-[#e6edf3] mt-1">{value}</div>
    </div>
  );
}

function Badge({ text, color }: { text: string; color: string }) {
  const colors: Record<string, string> = {
    blue: "bg-[#1f3a5f] text-[#58a6ff]",
    purple: "bg-[#2d1f5e] text-[#bc8cff]",
    green: "bg-[#0d2818] text-[#3fb950]",
  };
  return <span className={`px-2 py-0.5 rounded-full text-[10px] font-medium ${colors[color] || ""}`}>{text}</span>;
}

function Footer({ outputDir }: { outputDir: string }) {
  const openPath = useStore((s) => s.openPath);
  return (
    <div className="px-6 py-2.5 border-t border-[#21262d] text-xs text-[#484f58] flex items-center justify-between">
      <button onClick={() => openPath(outputDir)} className="hover:text-[#58a6ff] transition-colors cursor-pointer flex items-center gap-1">
        📁 {normalizePath(outputDir)}
      </button>
      <span>v{useStore.getState().config ? "1" : "?"} • Phase 1</span>
    </div>
  );
}
