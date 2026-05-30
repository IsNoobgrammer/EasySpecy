import { useState, useEffect } from "react";
import { Dashboard } from "./components/Dashboard";
import { Settings } from "./components/Settings";
import { ToastContainer } from "./components/Toast";
import { useStore } from "./stores/recording";

export default function App() {
  const [page, setPage] = useState<"dashboard" | "settings">("dashboard");
  const loadConfig = useStore((s) => s.loadConfig);

  useEffect(() => {
    loadConfig();
  }, []);

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
