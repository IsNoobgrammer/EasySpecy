import { Icon } from "./Icon";
import { motion, AnimatePresence } from "motion/react";
import { useStore, Toast } from "../stores/recording";

export function ToastContainer() {
  const toasts = useStore((s) => s.toasts);
  const removeToast = useStore((s) => s.removeToast);

  return (
    <div className="fixed top-4 right-4 z-[10000] flex flex-col gap-2 max-w-sm">
      <AnimatePresence>
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} onClose={() => removeToast(t.id)} />
        ))}
      </AnimatePresence>
    </div>
  );
}

function ToastItem({ toast, onClose }: { toast: Toast; onClose: () => void }) {
  const borderColors = {
    info: "var(--accent-info)",
    success: "var(--accent-success)",
    error: "var(--accent-danger)",
  };

  const icons = {
    info: "info",
    success: "check_circle",
    error: "error",
  };

  return (
    <motion.div
      layout
      initial={{ x: 80, opacity: 0, scale: 0.95 }}
      animate={{ x: 0, opacity: 1, scale: 1 }}
      exit={{ x: 80, opacity: 0, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 400, damping: 30 }}
      className="flex items-center gap-3 px-4 py-3"
      style={{
        border: `var(--border-width) solid ${borderColors[toast.type]}`,
        background: "var(--bg-surface)",
        boxShadow: "var(--shadow-md)",
        borderRadius: "var(--radius-sm)",
        backdropFilter: "blur(8px)",
      }}
    >
      <Icon name={icons[toast.type]} size={16} style={{ color: borderColors[toast.type] }} />
      <span className="font-mono text-xs flex-1 font-bold" style={{ color: "var(--text-primary)", letterSpacing: "0.02em" }}>
        {toast.message}
      </span>
      {toast.action && (
        <motion.button
          onClick={() => {
            toast.action!.onClick();
            onClose();
          }}
          className="font-mono text-xs font-bold whitespace-nowrap cursor-pointer px-2 py-0.5"
          style={{ color: "var(--accent-info)", border: "var(--border-thin) solid var(--accent-info)", borderRadius: "var(--radius-xs)" }}
          whileHover={{ scale: 1.05, background: "oklch(from var(--accent-info) l c h / 0.1)" }}
          whileTap={{ scale: 0.95 }}
        >
          {toast.action.label.toUpperCase()}
        </motion.button>
      )}
      <motion.button
        onClick={onClose}
        className="cursor-pointer ml-1 flex items-center justify-center"
        style={{ color: "var(--text-muted)" }}
        whileHover={{ color: "var(--text-primary)", scale: 1.1 }}
        whileTap={{ scale: 0.9 }}
      >
        <Icon name="close" size={14} style={{  }} />
      </motion.button>
    </motion.div>
  );
}
