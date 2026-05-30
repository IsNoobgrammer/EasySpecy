import { useEffect, useState } from "react";
import { useRecordingStore } from "../stores/recording";
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
  } = useRecordingStore();

  const [elapsed, setElapsed] = useState("00:00:00");
  const [version, setVersion] = useState("0.1.0");

  useEffect(() => {
    invoke<string>("get_version").then(setVersion);
  }, []);

  useEffect(() => {
    if (!isRecording || !recordingStartTime) return;
    const interval = setInterval(() => {
      setElapsed(formatDuration(Date.now() - recordingStartTime));
    }, 1000);
    return () => clearInterval(interval);
  }, [isRecording, recordingStartTime]);

  const resolutionLabel = config
    ? `${config.resolution_width}x${config.resolution_height}`
    : "1920x1080";
  const fpsLabel = config ? `${config.fps} fps` : "30 fps";

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--border)]">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-[var(--accent-green)] flex items-center justify-center text-sm font-bold text-black">
            ES
          </div>
          <span className="text-lg font-semibold">EasySpecy</span>
          <span className="text-xs text-[var(--text-secondary)]">
            v{version}
          </span>
        </div>
        <button
          onClick={onOpenSettings}
          className="px-3 py-1.5 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-[var(--border)] rounded-md hover:bg-[var(--bg-card)] transition-colors"
        >
          Settings
        </button>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col items-center justify-center gap-8 px-6">
        {/* Recording Status */}
        {isRecording && (
          <div className="flex items-center gap-3 px-4 py-2 rounded-lg bg-[var(--bg-card)] border border-[var(--border)]">
            <div
              className={`w-3 h-3 rounded-full ${
                isPaused
                  ? "bg-yellow-400"
                  : "bg-[var(--accent-red)] animate-pulse"
              }`}
            />
            <span className="font-mono text-xl">{elapsed}</span>
            {isPaused && (
              <span className="text-xs text-yellow-400 ml-2">PAUSED</span>
            )}
          </div>
        )}

        {/* Record Button */}
        <button
          onClick={isRecording ? stopRecording : startRecording}
          className={`
            w-24 h-24 rounded-full flex items-center justify-center text-sm font-semibold
            transition-all duration-200 cursor-pointer
            ${
              isRecording
                ? "bg-[var(--accent-red)] hover:bg-red-600 text-white scale-100"
                : "bg-[var(--accent-green)] hover:bg-green-500 text-black hover:scale-105"
            }
          `}
        >
          {isRecording ? (
            <div className="w-6 h-6 rounded-sm bg-white" />
          ) : (
            <div className="w-6 h-6 rounded-full bg-black" />
          )}
        </button>

        {/* Pause Button */}
        {isRecording && (
          <button
            onClick={pauseRecording}
            className="px-4 py-2 text-sm border border-[var(--border)] rounded-md hover:bg-[var(--bg-card)] transition-colors"
          >
            {isPaused ? "Resume" : "Pause"}
          </button>
        )}

        {/* Settings Summary */}
        <div className="grid grid-cols-3 gap-4 w-full max-w-md">
          <div className="text-center px-3 py-2 rounded-md bg-[var(--bg-card)] border border-[var(--border)]">
            <div className="text-xs text-[var(--text-secondary)]">
              Resolution
            </div>
            <div className="text-sm font-medium mt-1">{resolutionLabel}</div>
          </div>
          <div className="text-center px-3 py-2 rounded-md bg-[var(--bg-card)] border border-[var(--border)]">
            <div className="text-xs text-[var(--text-secondary)]">FPS</div>
            <div className="text-sm font-medium mt-1">{fpsLabel}</div>
          </div>
          <div className="text-center px-3 py-2 rounded-md bg-[var(--bg-card)] border border-[var(--border)]">
            <div className="text-xs text-[var(--text-secondary)]">Audio</div>
            <div className="text-sm font-medium mt-1">
              {config?.audio_enabled ? `${config.audio_sample_rate} Hz` : "Off"}
            </div>
          </div>
        </div>

        {/* Hotkey hint */}
        {!isRecording && (
          <div className="text-xs text-[var(--text-secondary)]">
            Press{" "}
            <kbd className="px-1.5 py-0.5 rounded bg-[var(--bg-card)] border border-[var(--border)] text-[var(--text-primary)]">
              {config?.hotkey_start || "Ctrl+Shift+R"}
            </kbd>{" "}
            to start recording
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="px-6 py-3 border-t border-[var(--border)] text-xs text-[var(--text-secondary)] text-center">
        Output: {config?.output_dir || "~/Videos/EasySpecy"}
      </div>
    </div>
  );
}
