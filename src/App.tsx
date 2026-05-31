import { useState, useEffect } from "react";
import { motion } from "motion/react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { Customization } from "./components/Customization";
import { ToastContainer } from "./components/Toast";
import { Icon } from "./components/Icon";
import { useStore } from "./stores/recording";
import { useThemeStore } from "./lib/theme";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

type Page = "dashboard" | "settings" | "customization";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const loadConfig = useStore((s) => s.loadConfig);
  const loadHistory = useStore((s) => s.loadHistory);
  const recordingPhase = useStore((s) => s.recordingPhase);
  const { theme, toggleTheme } = useThemeStore();
  const [version, setVersion] = useState("0.1.0");

  useEffect(() => {
    loadConfig();
    loadHistory();
    invoke<string>("get_version").then(setVersion).catch(() => {});
    if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission();
    }
    const unlisten = listen("region-recording-started", () => {
      useStore.setState({ recordingPhase: "recording", isPaused: false, recordingStartTime: Date.now() });
      useStore.getState().addToast("Recording started (region)", "success");
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const isRecording = recordingPhase === "recording";

  return (
    <div className="h-screen w-screen overflow-hidden flex" style={{ color: "var(--text-primary)" }}>
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
          <NavItem icon="palette" label="Customization" active={page === "customization"} onClick={() => setPage("customization")} />
          <NavItem icon="settings" label="Settings" active={page === "settings"} onClick={() => setPage("settings")} />
        </nav>

        {/* Bottom */}
        <div className="mt-auto px-4 pt-4" style={{ borderTop: "1px solid var(--border-default)" }}>
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
            <Settings onBack={() => setPage("dashboard")} />
          ) : page === "customization" ? (
            <Customization onBack={() => setPage("dashboard")} />
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
    </div>
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
