import { useState, useEffect } from "react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { ToastContainer } from "./components/Toast";
import { WindowPicker } from "./components/RegionSelector";
import { useStore } from "./stores/recording";
import { useThemeStore } from "./lib/theme";
import { listen } from "@tauri-apps/api/event";

export default function App() {
  const [page, setPage] = useState<"dashboard" | "settings">("dashboard");
  const loadConfig = useStore((s) => s.loadConfig);
  const loadHistory = useStore((s) => s.loadHistory);
  const { theme } = useThemeStore();
  const selectorMode = useStore((s) => s.selectorMode);
  const setSelectorMode = useStore((s) => s.setSelectorMode);
  const setCaptureWindow = useStore((s) => s.setCaptureWindow);

  useEffect(() => {
    loadConfig();
    loadHistory();
    if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission();
    }

    // Listen for region recording started event from Rust
    const unlisten = listen("region-recording-started", () => {
      useStore.setState({
        recordingPhase: "recording",
        isPaused: false,
        recordingStartTime: Date.now(),
      });
      useStore.getState().addToast("Recording started (region)", "success");
    });

    return () => { unlisten.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return (
    <div className="h-screen w-screen overflow-hidden">
      <ToastContainer />
      {page === "settings" ? (
        <Settings onBack={() => setPage("dashboard")} />
      ) : (
        <Dashboard onOpenSettings={() => setPage("settings")} />
      )}
      {selectorMode === "window" && (
        <WindowPicker
          onSelect={(w) => setCaptureWindow(w)}
          onClose={() => setSelectorMode("none")}
        />
      )}
    </div>
  );
}
