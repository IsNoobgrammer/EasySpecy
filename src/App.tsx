import { useState, useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
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
    <div className="h-screen w-screen overflow-hidden" style={{ background: "var(--bg-base)" }}>
      <ToastContainer />
      <AnimatePresence mode="wait">
        {page === "settings" ? (
          <motion.div
            key="settings"
            initial={{ x: 40, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: 40, opacity: 0 }}
            transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
            className="h-full"
          >
            <Settings onBack={() => setPage("dashboard")} />
          </motion.div>
        ) : (
          <motion.div
            key="dashboard"
            initial={{ x: -40, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: -40, opacity: 0 }}
            transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
            className="h-full"
          >
            <Dashboard onOpenSettings={() => setPage("settings")} />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
