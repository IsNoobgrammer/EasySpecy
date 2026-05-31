import { useState, useEffect } from "react";
import { motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";

// ═══ REGION SELECTOR (fullscreen overlay) ═══
export function RegionSelector({ onComplete, onCancel }: {
  onComplete: (region: { x: number; y: number; width: number; height: number }) => void;
  onCancel: () => void;
}) {
  const [start, setStart] = useState<{ x: number; y: number } | null>(null);
  const [end, setEnd] = useState<{ x: number; y: number } | null>(null);
  const [selecting, setSelecting] = useState(false);

  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        await invoke("exit_region_mode").catch(() => {});
        onCancel();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onCancel]);

  const handleMouseDown = (e: React.MouseEvent) => {
    setStart({ x: e.clientX, y: e.clientY });
    setEnd({ x: e.clientX, y: e.clientY });
    setSelecting(true);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (selecting) setEnd({ x: e.clientX, y: e.clientY });
  };

  const handleMouseUp = () => {
    if (!start || !end) return;
    setSelecting(false);
    const rect = getRect();
    if (rect.width > 30 && rect.height > 30) {
      // Auto-confirm on mouse up
      onComplete(rect);
    }
  };

  const getRect = () => {
    if (!start || !end) return { x: 0, y: 0, width: 0, height: 0 };
    return {
      x: Math.min(start.x, end.x),
      y: Math.min(start.y, end.y),
      width: Math.abs(end.x - start.x),
      height: Math.abs(end.y - start.y),
    };
  };

  const rect = start && end ? getRect() : null;

  return (
    <div
      className="fixed inset-0 z-[9999] cursor-crosshair"
      style={{ background: "oklch(0 0 0 / 0.45)" }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {/* Instructions */}
      {!selecting && !rect && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="absolute top-8 left-1/2 -translate-x-1/2 px-6 py-3"
          style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)" }}
        >
          <span className="font-mono text-sm" style={{ color: "var(--text-primary)" }}>
            Drag to select area · Press <kbd style={{ background: "var(--bg-elevated)", border: "var(--border-thin) solid var(--border-default)", padding: "2px 6px", margin: "0 4px", fontSize: "0.75rem" }}>Esc</kbd> to cancel
          </span>
        </motion.div>
      )}

      {/* Selection */}
      {rect && rect.width > 0 && rect.height > 0 && (
        <>
          {/* Clear area */}
          <div
            className="absolute"
            style={{
              left: rect.x, top: rect.y, width: rect.width, height: rect.height,
              background: "transparent",
              boxShadow: "0 0 0 9999px oklch(0 0 0 / 0.45)",
            }}
          />
          {/* Border */}
          <div
            className="absolute pointer-events-none"
            style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height, border: "2px solid var(--accent-primary)" }}
          />
          {/* Size label */}
          <div
            className="absolute font-mono text-xs px-2 py-1 pointer-events-none"
            style={{ left: rect.x, top: Math.max(0, rect.y - 28), background: "var(--accent-primary)", color: "var(--text-primary)" }}
          >
            {rect.width} × {rect.height}
          </div>
          {/* Corner handles */}
          {[
            { x: rect.x - 4, y: rect.y - 4 },
            { x: rect.x + rect.width - 4, y: rect.y - 4 },
            { x: rect.x - 4, y: rect.y + rect.height - 4 },
            { x: rect.x + rect.width - 4, y: rect.y + rect.height - 4 },
          ].map((pos, i) => (
            <div key={i} className="absolute pointer-events-none" style={{ left: pos.x, top: pos.y, width: 8, height: 8, background: "var(--accent-primary)" }} />
          ))}
        </>
      )}
    </div>
  );
}

// ═══ WINDOW INFO TYPE ═══
export interface WindowInfo {
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  hwnd: number;
}

// ═══ WINDOW PICKER (Alt+Tab style grid) ═══
export function WindowPicker({ onSelect, onClose }: {
  onSelect: (window: WindowInfo) => void;
  onClose: () => void;
}) {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState(0);

  useEffect(() => {
    invoke<WindowInfo[]>("get_windows").then((w) => {
      setWindows(w);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "Enter" && windows.length > 0) onSelect(windows[selected]);
      if (e.key === "ArrowRight" || e.key === "Tab") {
        e.preventDefault();
        setSelected((s) => (s + 1) % windows.length);
      }
      if (e.key === "ArrowLeft") {
        setSelected((s) => (s - 1 + windows.length) % windows.length);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [windows, selected, onSelect, onClose]);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[9999] flex items-center justify-center"
      style={{ background: "oklch(0 0 0 / 0.75)" }}
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95, y: 20 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.95, y: 20 }}
        className="w-full max-w-3xl max-h-[80vh] overflow-hidden"
        style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)" }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <div className="font-mono text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
            Select Window to Capture
          </div>
          <div className="font-mono text-xs mt-1" style={{ color: "var(--text-muted)" }}>
            Arrow keys to navigate · Enter to select · Esc to cancel
          </div>
        </div>

        {/* Window Grid */}
        <div className="p-6 overflow-y-auto max-h-[60vh]">
          {loading ? (
            <div className="text-center py-12 font-mono text-sm" style={{ color: "var(--text-muted)" }}>
              Scanning windows...
            </div>
          ) : windows.length === 0 ? (
            <div className="text-center py-12 font-mono text-sm" style={{ color: "var(--text-muted)" }}>
              No capturable windows found
            </div>
          ) : (
            <div className="grid grid-cols-3 gap-4">
              {windows.map((w, i) => (
                <motion.button
                  key={i}
                  className="text-left p-4 cursor-pointer"
                  style={{
                    border: `var(--border-width) solid ${i === selected ? "var(--accent-primary)" : "var(--border-default)"}`,
                    background: i === selected ? "var(--bg-elevated)" : "var(--bg-base)",
                  }}
                  whileHover={{ borderColor: "var(--accent-primary)", background: "var(--bg-elevated)" }}
                  whileTap={{ scale: 0.97 }}
                  onClick={() => onSelect(w)}
                  onMouseEnter={() => setSelected(i)}
                >
                  {/* Window icon placeholder */}
                  <div className="w-full h-20 mb-3 flex items-center justify-center" style={{ background: "var(--bg-surface)", border: "var(--border-thin) solid var(--border-default)" }}>
                    <span className="text-2xl">🪟</span>
                  </div>
                  <div className="font-mono text-xs font-medium truncate" style={{ color: "var(--text-primary)" }}>
                    {w.title}
                  </div>
                  <div className="font-mono mt-1" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
                    {w.width}×{w.height}
                  </div>
                </motion.button>
              ))}
            </div>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}
