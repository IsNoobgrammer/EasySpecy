import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";

export interface AppConfig {
  output_dir: string;
  resolution_width: number;
  resolution_height: number;
  fps: number;
  video_encoder: "H264" | "H265" | "AV1" | "AV1_NVENC" | "H264_NVENC" | "H265_NVENC" | "VP9" | "MobileShareable";
  video_bitrate_kbps: number;
  video_quality: "Low" | "Medium" | "High" | "Ultra" | "Insane" | "Custom";
  audio_enabled: boolean;
  audio_source: "Mic" | "System" | "Both";
  audio_sample_rate: number;
  audio_device: string;
  mic_gain: number;
  system_volume: number;
  noise_gate_threshold: number;
  noise_reduction: number;
  noise_reduction_mode: "Off" | "Gate" | "Spectral" | "RNN" | "Full";
  webcam_enabled: boolean;
  webcam_device: string;
  webcam_position: "TopLeft" | "TopRight" | "BottomLeft" | "BottomRight";
  webcam_size: number;
  webcam_shape: "Circle" | "Rounded" | "Squircle";
  webcam_border_color: string;
  webcam_border_width: number;
  webcam_opacity: number;
  webcam_x: number;
  webcam_y: number;
  webcam_sharpen: number;
  webcam_brightness: number;
  webcam_contrast: number;
  auto_zoom_enabled: boolean;
  zoom_level: number;
  zoom_dwell_ms: number;
  zoom_speed: number;
  zoom_sensitivity: number;
  cursor_trail_enabled: boolean;
  cursor_trail_color: string;
  cursor_trail_size: number;
  cursor_smoothing: boolean;
  trail_length: number;
  cursor_size_multiplier: number;
  cursor_pack: string;
  trail_style: string;
  click_effect: string;
  cursor_secondary_color: string;
  hotkey_start: string;
  hotkey_stop: string;
  hotkey_pause: string;
  minimize_to_tray: boolean;
  copy_path_on_save: boolean;
  recording_mode: "FullScreen" | "Region" | "Window";
  auto_check_updates: boolean;

  // Keyboard overlay settings
  keyboard_overlay_enabled: boolean;
  keyboard_game_capture: boolean;
  keyboard_overlay_font_family: string;
  keyboard_overlay_font_size: number;
  keyboard_overlay_opacity: number;
  keyboard_overlay_x: number;
  keyboard_overlay_y: number;
  keyboard_overlay_corner_radius: number;
  keyboard_overlay_border_width: number;
  keyboard_overlay_border_color: string;
  keyboard_overlay_background_color: string;
  keyboard_overlay_text_color: string;
  keyboard_overlay_theme: string;
  keyboard_overlay_key_mappings: string;
  keyboard_overlay_max_bubbles: number;
  keyboard_overlay_bubble_timeout_ms: number;
  keyboard_overlay_width: number;

  // GPU Settings
  gpu_encoders_enabled: boolean;
}

export interface KeyEvent {
  key: string;
  timestamp_ms: number;
  duration_ms: number;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  win?: boolean;
}

export interface RecordingResult {
  output_path: string;
  duration_secs: number;
  frame_count: number;
  file_size_bytes: number;
  has_audio: boolean;
}

export interface RecordingEntry {
  id: string;
  output_path: string;
  duration_secs: number;
  file_size_bytes: number;
  has_audio: boolean;
  resolution: string;
  fps: number;
  created_at: string;
}

export interface Toast {
  id: number;
  message: string;
  type: "info" | "success" | "error";
  action?: { label: string; onClick: () => void };
}

export interface CaptureRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface AudioLevels {
  micRms: number;
  micPeak: number;
  micDb: number;
  sysRms: number;
  sysPeak: number;
  sysDb: number;
}

type RecordingPhase = "idle" | "recording" | "encoding";
type SelectorMode = "none" | "region" | "window";

export interface WindowInfo { title: string; x: number; y: number; width: number; height: number; hwnd: number; }

interface AppState {
  config: AppConfig | null;
  configLoaded: boolean;
  recordingPhase: RecordingPhase;
  isPaused: boolean;
  recordingStartTime: number | null;
  pausedMs: number;           // accumulated ms spent paused
  pauseStartTime: number | null; // wall-clock when current pause began
  lastRecording: RecordingResult | null;
  history: RecordingEntry[];
  audioDevices: string[];
  toasts: Toast[];
  toastId: number;
  hotkeysRegistered: boolean;
  selectorMode: SelectorMode;
  regionShot: string | null;   // desktop screenshot the region selector draws on
  windows: WindowInfo[];       // candidates for Window mode
  encodingProgress: number;
  encodingStage: string;
  estimatedMbPerMin: number;
  audioLevels: AudioLevels;

  loadConfig: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<void>;
  updateField: (key: string, value: string | number | boolean) => Promise<void>;
  startRecording: () => Promise<void>;
  recordWindow: (w: WindowInfo) => Promise<void>;
  stopRecording: () => Promise<void>;
  pauseRecording: () => Promise<void>;
  resumeRecording: () => Promise<void>;
  loadAudioDevices: () => Promise<void>;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  registerHotkeys: () => Promise<void>;
  unregisterHotkeys: () => Promise<void>;
  setSelectorMode: (mode: SelectorMode) => void;
  setCaptureRegion: (region: CaptureRegion) => Promise<void>;
  addToast: (message: string, type: Toast["type"], action?: Toast["action"]) => void;
  removeToast: (id: number) => void;
  openPath: (path: string) => Promise<void>;
  revealInExplorer: (path: string) => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
  pollEncodingProgress: () => Promise<void>;
  loadEstimatedSize: () => Promise<void>;
  startAudioMonitor: () => Promise<void>;
  stopAudioMonitor: () => Promise<void>;
  pollAudioLevels: () => Promise<void>;
  keyboardEvents: KeyEvent[];
  pollKeyboardEvents: () => Promise<void>;
}

const hotkeyId = (k: string) => k.toLowerCase().replace(/\s/g, "");

/** Start capture and wait until the first frame is armed, then start the UI timer. */
async function beginCapture(successMsg: string) {
  const { addToast } = useStore.getState();
  try {
    addToast("Initializing capture...", "info");
    await invoke("start_recording", { outputPath: null }); // blocks until frame 0 + audio armed
    useStore.setState({ keyboardEvents: [], recordingPhase: "recording", isPaused: false,
      recordingStartTime: Date.now(), pausedMs: 0, pauseStartTime: null });
    addToast(successMsg, "success");
  } catch (e) { addToast(`Start failed: ${e}`, "error"); }
}

export const useStore = create<AppState>((set, get) => ({
  config: null,
  configLoaded: false,
  recordingPhase: "idle",
  isPaused: false,
  recordingStartTime: null,
  pausedMs: 0,
  pauseStartTime: null,
  lastRecording: null,
  history: [],
  audioDevices: [],
  toasts: [],
  toastId: 0,
  hotkeysRegistered: false,
  selectorMode: "none",
  regionShot: null,
  windows: [],
  encodingProgress: 0,
  encodingStage: "",
  estimatedMbPerMin: 0,
  audioLevels: { micRms: 0, micPeak: 0, micDb: -60, sysRms: 0, sysPeak: 0, sysDb: -60 },
  keyboardEvents: [],

  pollKeyboardEvents: async () => {
    try {
      const keyboardEvents = await invoke<KeyEvent[]>("get_keyboard_events");
      set({ keyboardEvents });
    } catch {}
  },

  addToast: (message, type, action) => {
    const id = get().toastId + 1;
    set((s) => ({ toasts: [...s.toasts, { id, message, type, action }], toastId: id }));
    setTimeout(() => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), 4000);
  },

  removeToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  loadConfig: async () => {
    try {
      const config = await invoke<AppConfig>("get_config");
      set({ config, configLoaded: true });
      if (!get().hotkeysRegistered) await get().registerHotkeys();
    } catch (e) { get().addToast(`Config load failed: ${e}`, "error"); }
  },

  saveConfig: async (config: AppConfig) => {
    try {
      await invoke("save_config", { config });
      await get().unregisterHotkeys(); // must run before set(): it unregisters get().config's keys
      set({ config });
      await get().registerHotkeys();
    } catch (e) { get().addToast(`Save failed: ${e}`, "error"); }
  },

  updateField: async (key: string, value: string | number | boolean) => {
    try {
      await invoke("update_config_field", { key, value });
      const config = await invoke<AppConfig>("get_config");
      set({ config });
      get().addToast(`${key} updated`, "success");
    } catch (e) { get().addToast(`Update failed: ${e}`, "error"); }
  },

  registerHotkeys: async () => {
    const { config } = get();
    if (!config) return;
    try {
      const startKey = config.hotkey_start.toLowerCase().replace(/\s/g, "");
      const stopKey = config.hotkey_stop.toLowerCase().replace(/\s/g, "");
      await register(startKey, (event) => {
        if (event.state === "Pressed" && get().recordingPhase === "idle") get().startRecording();
      });
      await register(stopKey, (event) => {
        if (event.state === "Pressed" && get().recordingPhase === "recording") get().stopRecording();
      });
      if (config.hotkey_pause) {
        await register(hotkeyId(config.hotkey_pause), (event) => {
          if (event.state !== "Pressed" || get().recordingPhase !== "recording") return;
          if (get().isPaused) get().resumeRecording(); else get().pauseRecording();
        });
      }
      set({ hotkeysRegistered: true });
    } catch (e) { console.error("Hotkey registration failed:", e); }
  },

  unregisterHotkeys: async () => {
    try {
      const { config } = get();
      if (config) {
        await unregister(config.hotkey_start.toLowerCase().replace(/\s/g, "")).catch(() => {});
        await unregister(config.hotkey_stop.toLowerCase().replace(/\s/g, "")).catch(() => {});
        if (config.hotkey_pause) await unregister(hotkeyId(config.hotkey_pause)).catch(() => {});
      }
      set({ hotkeysRegistered: false });
    } catch {}
  },

  setSelectorMode: (mode) => set({ selectorMode: mode }),

  setCaptureRegion: async (region) => {
    // Selector reports CSS px; the crop runs on physical frame px (125%/150% scaling).
    const dpr = window.devicePixelRatio || 1;
    await invoke("exit_region_mode").catch(() => {});
    set({ selectorMode: "none", regionShot: null });
    try {
      await invoke("set_capture_region", {
        x: Math.round(region.x * dpr), y: Math.round(region.y * dpr),
        width: Math.round(region.width * dpr), height: Math.round(region.height * dpr),
      });
    } catch (e) { get().addToast(`Region failed: ${e}`, "error"); return; }
    await beginCapture(`Recording region ${Math.round(region.width * dpr)}×${Math.round(region.height * dpr)}`);
  },

  recordWindow: async (w) => {
    set({ selectorMode: "none" });
    try {
      // Bounds are already physical px. Crop of the screen (not window capture) so the
      // live overlays — webcam, trail, keys — stay in the recording.
      await invoke("set_capture_region", { x: w.x, y: w.y, width: w.width, height: w.height });
      await invoke("focus_window", { hwnd: w.hwnd });
    } catch (e) { get().addToast(`Window select failed: ${e}`, "error"); return; }
    await beginCapture(`Recording "${w.title.slice(0, 40)}"`);
  },

  startRecording: async () => {
    const { config } = get();
    if (config?.recording_mode === "Region") {
      try {
        const shot = await invoke<string>("enter_region_mode");
        set({ selectorMode: "region", regionShot: shot });
      } catch (e) {
        await invoke("exit_region_mode").catch(() => {});
        get().addToast(`Region select failed: ${e}`, "error");
      }
      return;
    }
    if (config?.recording_mode === "Window") {
      try {
        set({ selectorMode: "window", windows: await invoke<WindowInfo[]>("get_windows") });
      } catch (e) { get().addToast(`Window list failed: ${e}`, "error"); }
      return;
    }
    // FullScreen — drop any region left over from an earlier region recording
    await invoke("clear_capture_region").catch(() => {});
    await beginCapture("Recording started");
  },

  stopRecording: async () => {
    try {
      set({ recordingPhase: "encoding", encodingProgress: 0, encodingStage: "Stopping capture..." });
      // Start polling encoding progress
      const progressInterval = setInterval(async () => {
        try {
          const [progress, stage] = await invoke<[number, string]>("get_encoding_progress");
          set({ encodingProgress: progress, encodingStage: stage });
        } catch {}
      }, 200);

      const result = await invoke<RecordingResult>("stop_recording");
      clearInterval(progressInterval);
      set({ recordingPhase: "idle", isPaused: false, recordingStartTime: null,
            pausedMs: 0, pauseStartTime: null, lastRecording: result,
            encodingProgress: 100, encodingStage: "Done" });
      const sizeMB = (result.file_size_bytes / 1_048_576).toFixed(1);
      const dur = result.duration_secs.toFixed(1);
      get().addToast(`Saved! ${dur}s, ${sizeMB}MB`, "success", { label: "Open", onClick: () => get().openPath(result.output_path) });
      if (get().config?.copy_path_on_save) await get().copyToClipboard(result.output_path);
      try {
        if ("Notification" in window && Notification.permission === "granted") {
          new Notification("EasySpecy — Recording Saved", { body: `${dur}s, ${sizeMB}MB` });
        }
      } catch {}
      await get().loadHistory();
    } catch (e) {
      set({ recordingPhase: "idle", isPaused: false, recordingStartTime: null, pausedMs: 0, pauseStartTime: null });
      get().addToast(`Stop failed: ${e}`, "error");
    }
  },

  pauseRecording: async () => {
    try {
      await invoke("pause_recording_cmd");
      set({ isPaused: true, pauseStartTime: Date.now() });
      get().addToast("Paused", "info");
    }
    catch (e) { get().addToast(`Pause failed: ${e}`, "error"); }
  },

  resumeRecording: async () => {
    try {
      await invoke("resume_recording_cmd");
      const { pauseStartTime, pausedMs } = get();
      const added = pauseStartTime ? Date.now() - pauseStartTime : 0;
      set({ isPaused: false, pauseStartTime: null, pausedMs: pausedMs + added });
      get().addToast("Resumed", "info");
    }
    catch (e) { get().addToast(`Resume failed: ${e}`, "error"); }
  },

  loadAudioDevices: async () => {
    try { const devices = await invoke<string[]>("get_audio_devices"); set({ audioDevices: devices }); } catch {}
  },

  loadHistory: async () => {
    try { const history = await invoke<RecordingEntry[]>("get_recording_history"); set({ history }); } catch {}
  },

  clearHistory: async () => {
    try { await invoke("clear_recording_history"); set({ history: [] }); get().addToast("History cleared", "info"); } catch {}
  },

  openPath: async (path: string) => {
    try { await invoke("open_path", { path }); } catch (e) { get().addToast(`Open failed: ${e}`, "error"); }
  },

  revealInExplorer: async (path: string) => {
    try {
      const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
      await revealItemInDir(path);
    } catch (e) { 
      get().addToast(`Reveal failed: ${e}`, "error"); 
    }
  },

  copyToClipboard: async (text: string) => {
    try { await navigator.clipboard.writeText(text); get().addToast("Path copied!", "info"); } catch {}
  },

  pollEncodingProgress: async () => {
    try {
      const [progress, stage] = await invoke<[number, string]>("get_encoding_progress");
      set({ encodingProgress: progress, encodingStage: stage });
    } catch {}
  },

  loadEstimatedSize: async () => {
    try {
      const [mbPerMin] = await invoke<[number, number, string]>("get_estimated_size");
      set({ estimatedMbPerMin: mbPerMin });
    } catch {}
  },

  startAudioMonitor: async () => {
    try {
      await invoke("start_audio_monitor_cmd");
    } catch (e) {
      console.warn("Audio monitor start failed:", e);
    }
  },

  stopAudioMonitor: async () => {
    try {
      await invoke("stop_audio_monitor_cmd");
      set({ audioLevels: { micRms: 0, micPeak: 0, micDb: -60, sysRms: 0, sysPeak: 0, sysDb: -60 } });
    } catch {}
  },

  pollAudioLevels: async () => {
    try {
      const levels = await invoke<{ mic_rms: number; mic_peak: number; mic_db: number; sys_rms: number; sys_peak: number; sys_db: number }>("get_audio_levels");
      set({ audioLevels: { micRms: levels.mic_rms, micPeak: levels.mic_peak, micDb: levels.mic_db, sysRms: levels.sys_rms, sysPeak: levels.sys_peak, sysDb: levels.sys_db } });
    } catch {}
  },
}));
