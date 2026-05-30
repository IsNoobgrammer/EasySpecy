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

interface RecordingState {
  // Config
  config: AppConfig | null;
  configLoaded: boolean;

  // Recording state
  isRecording: boolean;
  isPaused: boolean;
  recordingStartTime: number | null;

  // Devices
  audioDevices: string[];

  // Actions
  loadConfig: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  startRecording: () => void;
  stopRecording: () => void;
  pauseRecording: () => void;
  loadAudioDevices: () => Promise<void>;
}

export const useRecordingStore = create<RecordingState>((set, get) => ({
  config: null,
  configLoaded: false,
  isRecording: false,
  isPaused: false,
  recordingStartTime: null,
  audioDevices: [],

  loadConfig: async () => {
    try {
      const config = await invoke<AppConfig>("get_config");
      set({ config, configLoaded: true });
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  },

  saveConfig: async (config: AppConfig) => {
    try {
      await invoke("save_config", { config });
      set({ config });
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  },

  startRecording: () => {
    set({ isRecording: true, isPaused: false, recordingStartTime: Date.now() });
    // TODO Phase 1: invoke Rust start_recording
  },

  stopRecording: () => {
    set({ isRecording: false, isPaused: false, recordingStartTime: null });
    // TODO Phase 1: invoke Rust stop_recording
  },

  pauseRecording: () => {
    const { isPaused } = get();
    set({ isPaused: !isPaused });
    // TODO Phase 1: invoke Rust pause/resume
  },

  loadAudioDevices: async () => {
    try {
      const devices = await invoke<string[]>("get_audio_devices");
      set({ audioDevices: devices });
    } catch (e) {
      console.error("Failed to load audio devices:", e);
    }
  },
}));
