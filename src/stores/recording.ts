import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  output_dir: string;
  resolution_width: number;
  resolution_height: number;
  fps: number;
  audio_enabled: boolean;
  audio_sample_rate: number;
  audio_device: string;
  webcam_enabled: boolean;
  webcam_device: string;
  webcam_position: "TopLeft" | "TopRight" | "BottomLeft" | "BottomRight";
  webcam_size: number;
  auto_zoom_enabled: boolean;
  zoom_level: number;
  zoom_dwell_ms: number;
  cursor_trail_enabled: boolean;
  cursor_trail_color: string;
  cursor_trail_size: number;
  cursor_smoothing: boolean;
  cursor_size_multiplier: number;
  hotkey_start: string;
  hotkey_stop: string;
  hotkey_pause: string;
  minimize_to_tray: boolean;
  copy_path_on_save: boolean;
  recording_mode: "FullScreen" | "Region" | "Window";
}

export interface RecordingResult {
  output_path: string;
  duration_secs: number;
  frame_count: number;
  file_size_bytes: number;
}

export interface Toast {
  id: number;
  message: string;
  type: "info" | "success" | "error";
  action?: { label: string; onClick: () => void };
}

interface AppState {
  config: AppConfig | null;
  configLoaded: boolean;
  isRecording: boolean;
  isPaused: boolean;
  recordingStartTime: number | null;
  lastRecording: RecordingResult | null;
  audioDevices: string[];
  toasts: Toast[];
  toastId: number;

  loadConfig: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  pauseRecording: () => Promise<void>;
  resumeRecording: () => Promise<void>;
  loadAudioDevices: () => Promise<void>;
  addToast: (message: string, type: Toast["type"], action?: Toast["action"]) => void;
  removeToast: (id: number) => void;
  openPath: (path: string) => Promise<void>;
}

export const useStore = create<AppState>((set, get) => ({
  config: null,
  configLoaded: false,
  isRecording: false,
  isPaused: false,
  recordingStartTime: null,
  lastRecording: null,
  audioDevices: [],
  toasts: [],
  toastId: 0,

  addToast: (message, type, action) => {
    const id = get().toastId + 1;
    set((s) => ({
      toasts: [...s.toasts, { id, message, type, action }],
      toastId: id,
    }));
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, 4000);
  },

  removeToast: (id) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },

  loadConfig: async () => {
    try {
      const config = await invoke<AppConfig>("get_config");
      set({ config, configLoaded: true });
    } catch (e) {
      console.error("Failed to load config:", e);
      get().addToast(`Failed to load config: ${e}`, "error");
    }
  },

  saveConfig: async (config: AppConfig) => {
    const { addToast } = get();
    try {
      addToast("Saving...", "info");
      await invoke("save_config", { config });
      set({ config });
      addToast("Settings saved!", "success");
    } catch (e) {
      addToast(`Save failed: ${e}`, "error");
    }
  },

  startRecording: async () => {
    const { addToast } = get();
    try {
      addToast("Starting recording...", "info");
      await invoke("start_recording", { outputPath: null });
      set({ isRecording: true, isPaused: false, recordingStartTime: Date.now() });
      addToast("Recording!", "success");
    } catch (e) {
      addToast(`Recording failed: ${e}`, "error");
    }
  },

  stopRecording: async () => {
    const { addToast } = get();
    try {
      addToast("Stopping & encoding...", "info");
      const result = await invoke<RecordingResult>("stop_recording");
      set({
        isRecording: false,
        isPaused: false,
        recordingStartTime: null,
        lastRecording: result,
      });
      const sizeMB = (result.file_size_bytes / 1_048_576).toFixed(1);
      const duration = result.duration_secs.toFixed(1);
      addToast(
        `Saved! ${duration}s, ${sizeMB}MB`,
        "success",
        { label: "Open file", onClick: () => get().openPath(result.output_path) }
      );
    } catch (e) {
      set({ isRecording: false, isPaused: false, recordingStartTime: null });
      addToast(`Stop failed: ${e}`, "error");
    }
  },

  pauseRecording: async () => {
    try {
      await invoke("pause_recording_cmd");
      set({ isPaused: true });
      get().addToast("Paused", "info");
    } catch (e) {
      get().addToast(`Pause failed: ${e}`, "error");
    }
  },

  resumeRecording: async () => {
    try {
      await invoke("resume_recording_cmd");
      set({ isPaused: false });
      get().addToast("Resumed", "info");
    } catch (e) {
      get().addToast(`Resume failed: ${e}`, "error");
    }
  },

  loadAudioDevices: async () => {
    try {
      const devices = await invoke<string[]>("get_audio_devices");
      set({ audioDevices: devices });
    } catch (e) {
      console.error("Failed to load audio devices:", e);
    }
  },

  openPath: async (path: string) => {
    try {
      await invoke("open_path", { path });
    } catch (e) {
      get().addToast(`Failed to open: ${e}`, "error");
    }
  },
}));
