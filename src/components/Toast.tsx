import { useStore, Toast } from "../stores/recording";

export function ToastContainer() {
  const toasts = useStore((s) => s.toasts);
  const removeToast = useStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onClose={() => removeToast(t.id)} />
      ))}
    </div>
  );
}

function ToastItem({ toast, onClose }: { toast: Toast; onClose: () => void }) {
  const bgColors = {
    info: "bg-[#1c2128] border-[#30363d]",
    success: "bg-[#0d2818] border-[#238636]",
    error: "bg-[#2d1215] border-[#f85149]",
  };

  const dotColors = {
    info: "bg-[#58a6ff]",
    success: "bg-[#3fb950]",
    error: "bg-[#f85149]",
  };

  return (
    <div
      className={`
        ${bgColors[toast.type]}
        border rounded-lg px-4 py-3 shadow-lg
        animate-[slideIn_0.2s_ease-out]
        flex items-center gap-3
      `}
      style={{
        animation: "slideIn 0.2s ease-out",
      }}
    >
      <div className={`w-2 h-2 rounded-full ${dotColors[toast.type]} shrink-0`} />
      <span className="text-sm text-[#e6edf3] flex-1">{toast.message}</span>
      {toast.action && (
        <button
          onClick={() => {
            toast.action!.onClick();
            onClose();
          }}
          className="text-xs text-[#58a6ff] hover:text-[#79c0ff] font-medium whitespace-nowrap"
        >
          {toast.action.label}
        </button>
      )}
      <button
        onClick={onClose}
        className="text-[#8b949e] hover:text-[#e6edf3] text-xs ml-1"
      >
        ✕
      </button>
    </div>
  );
}
