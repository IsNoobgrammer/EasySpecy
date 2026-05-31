import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";

export interface AppConfig {
  output_dir: string;
  resolution_width: number;
  resolution_height: number;
  fps: number;
  audio_enabled: boolean;
  audio_source: "mic" | "system" | "both";
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
  has_audio: boolean;
}

export interface Toast {
  id: number;
  message: string;
  type: "info" | "success" | "error";
  action?: { label: string; onClick: () => void };
}

type RecordingPhase = "idle" | "recording" | "encoding";

interface AppState {
  config: AppConfig | null;
  configLoaded: boolean;
  recordingPhase: RecordingPhase;
  isPaused: boolean;
  recordingStartTime: number | null;
  lastRecording: RecordingResult | null;
  audioDevices: string[];
  toasts: Toast[];
  toastId: number;
  hotkeysRegistered: boolean;

  loadConfig: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  pauseRecording: () => Promise<void>;
  resumeRecording: () => Promise<void>;
  loadAudioDevices: () => Promise<void>;
  registerHotkeys: () => Promise<void>;
  unregisterHotkeys: () => Promise<void>;
  addToast: (message: string, type: Toast["type"], action?: Toast["action"]) => void;
  removeToast: (id: number) => void;
  openPath: (path: string) => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
}

export const useStore = create<AppState>((set, get) => ({
  config: null,
  configLoaded: false,
  recordingPhase: "idle",
  isPaused: false,
  recordingStartTime: null,
  lastRecording: null,
  audioDevices: [],
  toasts: [],
  toastId: 0,
  hotkeysRegistered: false,

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
      if (!get().hotkeysRegistered) {
        await get().registerHotkeys();
      }
    } catch (e) {
      get().addToast(`Config load failed: ${e}`, "error");
    }
  },

  saveConfig: async (config: AppConfig) => {
    try {
      get().addToast("Saving...", "info");
      await invoke("save_config", { config });
      set({ config });
      await get().unregisterHotkeys();
      await get().registerHotkeys();
      get().addToast("Settings saved!", "success");
    } catch (e) {
      get().addToast(`Save failed: ${e}`, "error");
    }
  },

  registerHotkeys: async () => {
    const { config } = get();
    if (!config) return;
    try {
      const startKey = config.hotkey_start.toLowerCase().replace(/\s/g, "");
      const stopKey = config.hotkey_stop.toLowerCase().replace(/\s/g, "");

      await register(startKey, (event) => {
        if (event.state === "Pressed" && get().recordingPhase === "idle") {
          get().startRecording();
        }
      });
      await register(stopKey, (event) => {
        if (event.state === "Pressed" && get().recordingPhase === "recording") {
          get().stopRecording();
        }
      });
      set({ hotkeysRegistered: true });
    } catch (e) {
      console.error("Hotkey registration failed:", e);
    }
  },

  unregisterHotkeys: async () => {
    try {
      const { config } = get();
      if (config) {
        await unregister(config.hotkey_start.toLowerCase().replace(/\s/g, "")).catch(() => {});
        await unregister(config.hotkey_stop.toLowerCase().replace(/\s/g, "")).catch(() => {});
      }
      set({ hotkeysRegistered: false });
    } catch {}
  },

  startRecording: async () => {
    const { addToast } = get();
    try {
      await invoke("start_recording", { outputPath: null });
      set({ recordingPhase: "recording", isPaused: false, recordingStartTime: Date.now() });
      addToast("Recording started", "success");
    } catch (e) {
      addToast(`Start failed: ${e}`, "error");
    }
  },

  stopRecording: async () => {
    const { addToast, copyToClipboard } = get();
    try {
      // Immediately show encoding state
      set({ recordingPhase: "encoding" });
      addToast("Encoding video...", "info");

      const result = await invoke<RecordingResult>("stop_recording");

      set({
        recordingPhase: "idle",
        isPaused: false,
        recordingStartTime: null,
        lastRecording: result,
      });

      const sizeMB = (result.file_size_bytes / 1_048_576).toFixed(1);
      const dur = result.duration_secs.toFixed(1);
      const audioLabel = result.has_audio ? "+audio" : "video-only";

      addToast(
        `Saved! ${dur}s, ${sizeMB}MB (${audioLabel})`,
        "success",
        { label: "Open file", onClick: () => get().openPath(result.output_path) }
      );

      if (get().config?.copy_path_on_save) {
        await copyToClipboard(result.output_path);
      }
    } catch (e) {
      set({ recordingPhase: "idle", isPaused: false, recordingStartTime: null });
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
      console.error("Audio devices failed:", e);
    }
  },

  openPath: async (path: string) => {
    try {
      await invoke("open_path", { path });
    } catch (e) {
      get().addToast(`Open failed: ${e}`, "error");
    }
  },

  copyToClipboard: async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      get().addToast("Path copied!", "info");
    } catch {}
  },
}));
