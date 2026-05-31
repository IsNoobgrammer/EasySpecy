import { useState, useEffect } from "react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { ToastContainer } from "./components/Toast";
import { useStore } from "./stores/recording";
import { useThemeStore } from "./lib/theme";

export default function App() {
  const [page, setPage] = useState<"dashboard" | "settings">("dashboard");
  const loadConfig = useStore((s) => s.loadConfig);
  const { theme } = useThemeStore();

  useEffect(() => {
    loadConfig();
    // Request notification permission
    if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission();
    }
  }, []);

  // Apply theme to document
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
    </div>
  );
}
