import { useState, useEffect } from "react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { useRecordingStore } from "./stores/recording";

export default function App() {
  const [page, setPage] = useState<"dashboard" | "settings">("dashboard");
  const loadConfig = useRecordingStore((s) => s.loadConfig);

  useEffect(() => {
    loadConfig();
  }, []);

  if (page === "settings") return <Settings onBack={() => setPage("dashboard")} />;
  return <Dashboard onOpenSettings={() => setPage("settings")} />;
}
