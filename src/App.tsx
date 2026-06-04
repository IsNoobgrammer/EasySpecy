import { useState, useEffect } from "react";
import { motion } from "motion/react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";

import { ToastContainer } from "./components/Toast";
import { Icon } from "./components/Icon";
import { SidebarStats } from "./components/StatusBar";
import { useStore } from "./stores/recording";
import { useThemeStore } from "./lib/theme";
import { ContextMenuProvider, useAppContextMenu } from "./components/ContextMenu";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Page = "dashboard" | "settings";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const loadConfig = useStore((s) => s.loadConfig);
  const loadHistory = useStore((s) => s.loadHistory);
  const recordingPhase = useStore((s) => s.recordingPhase);
  const config = useStore((s) => s.config);
  const audioLevels = useStore((s) => s.audioLevels);
  const startAudioMonitor = useStore((s) => s.startAudioMonitor);
  const stopAudioMonitor = useStore((s) => s.stopAudioMonitor);
  const pollAudioLevels = useStore((s) => s.pollAudioLevels);
  const { theme, toggleTheme } = useThemeStore();
  const [version, setVersion] = useState("0.1.0");
  const [portableMode, setPortableMode] = useState(false);

  const configLoaded = useStore((s) => s.configLoaded);
  const [updateAvailable, setUpdateAvailable] = useState<any>(null);
  const [updateProgress, setUpdateProgress] = useState<{ status: 'idle' | 'downloading' | 'installing' | 'complete' | 'error'; percentage: number; downloaded: number; total?: number }>({
    status: 'idle',
    percentage: 0,
    downloaded: 0
  });

  const checkForUpdates = async (isManual = false) => {
    try {
      const update = await check();
      if (update) {
        setUpdateAvailable(update);
      } else if (isManual) {
        useStore.getState().addToast("EasySpecy is up to date!", "success");
      }
    } catch (err) {
      console.error("Failed to check for updates:", err);
      if (isManual) {
        useStore.getState().addToast(`Check failed: ${err}`, "error");
      }
    }
  };

  useEffect(() => {
    if (configLoaded && config?.auto_check_updates) {
      checkForUpdates(false);
    }
  }, [configLoaded, config?.auto_check_updates]);

  const handleDownloadAndInstall = async () => {
    if (!updateAvailable) return;
    setUpdateProgress({ status: 'downloading', percentage: 0, downloaded: 0 });

    if (portableMode) {
      // ── Portable mode: download ZIP, replace exe, relaunch ──
      try {
        // Derive portable ZIP URL from the version number
        const ver = updateAvailable.version;
        const portableUrl = `https://github.com/IsNoobgrammer/EasySpecy/releases/download/v${ver}/EasySpecy-Portable.zip`;

        setUpdateProgress({ status: 'downloading', percentage: 10, downloaded: 0 });
        await invoke('install_portable_update', { url: portableUrl });
        // install_portable_update calls process::exit(0) after relaunching,
        // so we only reach here on error
      } catch (err) {
        console.error("Portable update failed:", err);
        setUpdateProgress({ status: 'error', percentage: 0, downloaded: 0 });
        useStore.getState().addToast(`Portable update failed: ${err}`, "error");
      }
      return;
    }

    // ── NSIS mode: use Tauri updater (remembers install path automatically) ──
    try {
      let totalSize: number | undefined;
      await updateAvailable.downloadAndInstall((event: any) => {
        switch (event.event) {
          case "Started":
            totalSize = event.data.contentLength;
            setUpdateProgress({
              status: 'downloading',
              percentage: 0,
              downloaded: 0,
              total: totalSize
            });
            break;
          case "Progress":
            const downloaded = event.data.chunkLength;
            setUpdateProgress((prev) => {
              const newDownloaded = (prev.downloaded || 0) + downloaded;
              const percentage = totalSize ? Math.min(Math.round((newDownloaded / totalSize) * 100), 100) : 0;
              return {
                ...prev,
                percentage,
                downloaded: newDownloaded,
                total: totalSize
              };
            });
            break;
          case "Finished":
            setUpdateProgress({ status: 'installing', percentage: 100, downloaded: totalSize || 0, total: totalSize });
            break;
        }
      });
      setUpdateProgress({ status: 'complete', percentage: 100, downloaded: totalSize || 0 });
      useStore.getState().addToast("Update installed! Restarting...", "success");
      setTimeout(async () => {
        await relaunch();
      }, 1500);
    } catch (err) {
      console.error("Failed to download and install update:", err);
      setUpdateProgress({ status: 'error', percentage: 0, downloaded: 0 });
      useStore.getState().addToast(`Update failed: ${err}`, "error");
    }
  };

  useEffect(() => {
    loadConfig();
    loadHistory();
    invoke<string>("get_version").then(setVersion).catch(() => {});
    invoke<boolean>("is_portable_mode").then(setPortableMode).catch(() => {});
    if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission();
    }
    const unlisten = listen("region-recording-started", () => {
      useStore.setState({ recordingPhase: "recording", isPaused: false, recordingStartTime: Date.now() });
      useStore.getState().addToast("Recording started (region)", "success");
    });
    // Release browser webcam when backend needs the device for nokhwa capture
    const unlistenWebcam = listen("release-webcam", () => {
      // Stop any existing video element streams (from WebcamPreview etc.)
      document.querySelectorAll("video").forEach((v) => {
        if (v.srcObject instanceof MediaStream) {
          (v.srcObject as MediaStream).getTracks().forEach((t) => t.stop());
          v.srcObject = null;
        }
      });
    });
    // Show webcam errors to user
    const unlistenWebcamError = listen<string>("webcam-error", (event) => {
      useStore.getState().addToast(`Webcam error: ${event.payload}`, "error");
    });
    return () => {
      unlisten.then((fn) => fn());
      unlistenWebcam.then((fn) => fn());
      unlistenWebcamError.then((fn) => fn());
    };
  }, []);

  // Audio monitor lifecycle — runs on ALL pages (not just Dashboard)
  useEffect(() => {
    startAudioMonitor();
    return () => { stopAudioMonitor(); };
  }, []);

  // Poll audio levels at ~20Hz — global so sidebar meter works everywhere
  useEffect(() => {
    const interval = setInterval(pollAudioLevels, 50);
    return () => clearInterval(interval);
  }, [pollAudioLevels]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const isRecording = recordingPhase === "recording";
  const appContextMenu = useAppContextMenu(setPage);

  // Disable browser default context menu (right-click)
  useEffect(() => {
    const handler = (e: Event) => e.preventDefault();
    document.addEventListener("contextmenu", handler);
    return () => document.removeEventListener("contextmenu", handler);
  }, []);

  return (
    <ContextMenuProvider>
    <div className="h-screen w-screen overflow-hidden flex" style={{ color: "var(--text-primary)" }} onContextMenu={appContextMenu.onContextMenu}>
      <ToastContainer />

      {/* ═══ SIDEBAR — 240px, exact Stitch layout ═══ */}
      <aside className="flex flex-col py-6 w-[240px] h-full shrink-0 z-10" style={{ background: "var(--surface-container-low, #191b26)", borderRight: "1px solid var(--border-default)" }}>
        {/* Logo */}
        <div className="px-4 mb-8">
          <div className="flex items-center gap-2">
            <img src="/logo.png" alt="EasySpecy" className="w-8 h-8 rounded" />
            <div>
              <h1 className="text-base font-bold" style={{ color: "var(--accent-primary)", fontFamily: "Inter" }}>EasySpecy</h1>
              <p className="font-mono uppercase" style={{ color: "var(--text-secondary)", fontSize: "9px", letterSpacing: "0.08em", fontWeight: 700 }}>PRO RECORDER</p>
            </div>
          </div>
        </div>

        {/* New Recording CTA */}
        <div className="px-4 mb-2">
          <motion.button
            onClick={() => { setPage("dashboard"); useStore.getState().startRecording(); }}
            disabled={isRecording}
            className="w-full px-4 py-2 font-mono text-[13px] font-medium flex items-center justify-center gap-2 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
            style={{ background: "var(--accent-primary-container, #00e88a)", color: "var(--on-primary-container, #006338)", borderRadius: "4px" }}
            whileHover={{ filter: "brightness(1.1)" }}
            whileTap={{ scale: 0.95 }}
          >
            <Icon name="radio_button_checked" size={16} />
            NEW RECORDING
          </motion.button>
        </div>

        {/* Nav Items */}
        <nav className="flex-1 mt-2 space-y-1">
          <NavItem icon="folder_open" label="Library" active={page === "dashboard"} onClick={() => setPage("dashboard")} />

          <NavItem icon="settings" label="Settings" active={page === "settings"} onClick={() => setPage("settings")} />
        </nav>

        {/* Bottom — Stats + Loudness + Theme */}
        <div className="mt-auto px-4 pt-4" style={{ borderTop: "1px solid var(--border-default)" }}>
          {/* ═══ SIDEBAR STATS (System Info + Loudness) ═══ */}
          <div className="mb-3">
            <SidebarStats
              audioSource={config?.audio_source || "Mic"}
              audioEnabled={config?.audio_enabled || false}
              levels={audioLevels}
            />
          </div>
          <motion.div
            onClick={toggleTheme}
            className="flex items-center gap-4 px-4 py-2 cursor-pointer rounded"
            style={{ color: "var(--text-secondary)" }}
            whileHover={{ background: "var(--surface-container-high, #272935)", color: "var(--text-primary)" }}
          >
            <Icon name={theme === "dark" ? "light_mode" : "dark_mode"} size={18} />
            <span className="font-mono text-[13px] font-medium">{theme === "dark" ? "Light Mode" : "Dark Mode"}</span>
          </motion.div>
        </div>
      </aside>

      {/* ═══ MAIN AREA ═══ */}
      <div className="flex-1 flex flex-col relative overflow-hidden">
        {/* Top Header Bar */}
        <header className="flex justify-between items-center h-8 px-6 w-full z-10" style={{ background: "var(--bg-surface)", borderBottom: "1px solid var(--border-default)" }}>
          <div className="flex items-center gap-4">
            <span className="font-mono text-[13px] font-medium" style={{ color: "var(--text-secondary)" }}>v{version}</span>
            <div className="h-3 w-px" style={{ background: "var(--border-default)" }} />
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full" style={{ background: isRecording ? "var(--accent-record)" : "var(--accent-primary)", boxShadow: isRecording ? "0 0 8px rgba(240,64,64,0.6)" : "0 0 8px rgba(133,255,180,0.6)" }} />
              <span className="font-mono text-[13px] font-medium uppercase" style={{ color: isRecording ? "var(--accent-record)" : "var(--accent-primary)" }}>
                {isRecording ? "Recording" : "System Ready"}
              </span>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <button className="cursor-pointer" style={{ color: "var(--text-secondary)" }} onClick={() => setPage("settings")}>
              <Icon name="settings" size={18} />
            </button>
          </div>
        </header>

        {/* Page Content */}
        <div className="flex-1 relative overflow-hidden">
          {page === "settings" ? (
            <Settings onBack={() => setPage("dashboard")} onCheckUpdate={checkForUpdates} />
          ) : (
            <Dashboard onOpenSettings={() => setPage("settings")} />
          )}
        </div>
      </div>

      {/* ═══ BACKGROUND AURORA — exact Stitch ═══ */}
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="absolute rounded-full" style={{ top: "-10%", right: "-10%", width: "40%", height: "40%", background: "rgba(133,255,180,0.05)", filter: "blur(120px)" }} />
        <div className="absolute rounded-full" style={{ bottom: "-10%", left: "-10%", width: "30%", height: "30%", background: "rgba(192,193,255,0.05)", filter: "blur(100px)" }} />
      </div>

      {/* ═══ UPDATE MODAL ═══ */}
      {updateAvailable && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            className="w-[480px] max-h-[85vh] flex flex-col rounded-xl overflow-hidden border shadow-2xl backdrop-blur-lg"
            style={{
              background: "var(--bg-surface, #161722)",
              borderColor: "var(--border-default, rgba(0, 232, 138, 0.2))",
            }}
          >
            {/* Header */}
            <div className="px-6 py-4 border-b flex justify-between items-center" style={{ borderColor: "var(--border-default)" }}>
              <div className="flex items-center gap-2">
                <Icon name="system_update_alt" size={18} style={{ color: "var(--accent-primary)" }} />
                <span className="font-mono text-xs font-bold uppercase tracking-wider">
                  Update Available
                </span>
              </div>
              <span className="font-mono text-[10px] px-2 py-0.5 rounded border uppercase" style={{ color: "var(--accent-primary)", borderColor: "var(--border-default)", background: "rgba(0, 232, 138, 0.08)" }}>
                v{updateAvailable.version}
              </span>
              {portableMode && (
                <span className="font-mono text-[10px] px-2 py-0.5 rounded border uppercase" style={{ color: "#ffd000", borderColor: "rgba(255, 208, 0, 0.3)", background: "rgba(255, 208, 0, 0.08)" }}>
                  Portable
                </span>
              )}
            </div>

            {/* Content / Release Notes */}
            <div className="p-6 flex-1 overflow-y-auto space-y-4">
              <div className="space-y-1">
                <span className="font-mono text-[10px]" style={{ color: "var(--text-secondary)" }}>RELEASE NOTES</span>
                <div 
                  className="font-mono text-xs p-4 rounded border overflow-y-auto max-h-[220px] scrollbar-thin whitespace-pre-wrap leading-relaxed"
                  style={{
                    background: "var(--surface-container-low, #191b26)",
                    borderColor: "var(--border-default)",
                    color: "var(--text-primary)"
                  }}
                >
                  {updateAvailable.body || "No release notes provided."}
                </div>
              </div>

              {/* Progress UI */}
              {updateProgress.status !== 'idle' && (
                <div className="space-y-2 pt-2">
                  <div className="flex justify-between font-mono text-[10px]" style={{ color: "var(--text-secondary)" }}>
                    <span className="uppercase">
                      {updateProgress.status === 'downloading' && "Downloading files..."}
                      {updateProgress.status === 'installing' && "Installing files..."}
                      {updateProgress.status === 'complete' && "Update Complete!"}
                      {updateProgress.status === 'error' && "Download Failed"}
                    </span>
                    <span>{updateProgress.percentage}%</span>
                  </div>
                  <div className="h-2 w-full rounded-full overflow-hidden relative border" style={{ background: "var(--surface-container-low)", borderColor: "var(--border-default)" }}>
                    <div
                      className="h-full rounded-full transition-all duration-300"
                      style={{
                        width: `${updateProgress.percentage}%`,
                        background: updateProgress.status === 'error' ? '#ff4444' : "linear-gradient(to right, var(--accent-primary) 65%, #ffd000 100%)",
                      }}
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Footer Buttons */}
            <div className="px-6 py-4 border-t flex justify-end gap-3" style={{ background: "rgba(0,0,0,0.15)", borderColor: "var(--border-default)" }}>
              {updateProgress.status === 'idle' ? (
                <>
                  <button
                    onClick={() => setUpdateAvailable(null)}
                    className="font-mono text-xs px-4 py-2 rounded cursor-pointer border font-bold uppercase transition-colors"
                    style={{ borderColor: "var(--border-default)", color: "var(--text-secondary)" }}
                  >
                    Later
                  </button>
                  <button
                    onClick={handleDownloadAndInstall}
                    className="font-mono text-xs px-4 py-2 rounded cursor-pointer font-extrabold uppercase border-none text-white shadow-lg"
                    style={{
                      background: "linear-gradient(135deg, var(--accent-primary-container), var(--accent-primary))",
                      color: "var(--on-primary)"
                    }}
                  >
                    Update Now
                  </button>
                </>
              ) : (
                <div className="font-mono text-xs py-2 uppercase font-extrabold" style={{ color: updateProgress.status === 'error' ? '#ff4444' : "var(--accent-primary)" }}>
                  {updateProgress.status === 'downloading' && "Downloading..."}
                  {updateProgress.status === 'installing' && "Running Installer..."}
                  {updateProgress.status === 'complete' && "Relaunching EasySpecy..."}
                  {updateProgress.status === 'error' && (
                    <button 
                      onClick={() => setUpdateProgress({ status: 'idle', percentage: 0, downloaded: 0 })}
                      className="px-4 py-2 rounded cursor-pointer border border-red-500/20 text-red-400 font-bold"
                    >
                      Retry
                    </button>
                  )}
                </div>
              )}
            </div>
          </motion.div>
        </div>
      )}
    </div>
    </ContextMenuProvider>
  );
}

function NavItem({ icon, label, active, onClick }: { icon: string; label: string; active: boolean; onClick: () => void }) {
  return (
    <motion.div
      onClick={onClick}
      className="flex items-center gap-4 px-4 py-2 cursor-pointer transition-all duration-150"
      style={{
        background: active ? "var(--surface-variant, #323440)" : "transparent",
        color: active ? "var(--accent-primary)" : "var(--text-secondary)",
        borderLeft: active ? "4px solid var(--accent-primary)" : "4px solid transparent",
      }}
      whileHover={!active ? { background: "var(--surface-container-high, #272935)", color: "var(--text-primary)" } : {}}
    >
      <Icon name={icon} size={18} />
      <span className="font-mono text-[13px] font-medium">{label}</span>
    </motion.div>
  );
}
