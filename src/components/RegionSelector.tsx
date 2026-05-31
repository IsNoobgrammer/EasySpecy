import { useState, useEffect, useCallback } from "react";
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

  const handleMouseUp = useCallback(() => {
    if (!start || !end) return;
    setSelecting(false);
    const rect = getRect();
    if (rect.width > 30 && rect.height > 30) {
      onComplete(rect);
    }
  }, [start, end, onComplete]);

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
      {!selecting && !rect && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="absolute top-8 left-1/2 -translate-x-1/2 px-6 py-3"
          style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)" }}
        >
          <span className="font-mono text-sm" style={{ color: "var(--text-primary)" }}>
            Drag to select area · <kbd style={{ background: "var(--bg-elevated)", border: "var(--border-thin) solid var(--border-default)", padding: "2px 6px", margin: "0 4px", fontSize: "0.75rem" }}>Esc</kbd> to cancel
          </span>
        </motion.div>
      )}

      {rect && rect.width > 0 && rect.height > 0 && (
        <>
          <div className="absolute" style={{
            left: rect.x, top: rect.y, width: rect.width, height: rect.height,
            background: "transparent", boxShadow: "0 0 0 9999px oklch(0 0 0 / 0.45)",
          }} />
          <div className="absolute pointer-events-none" style={{
            left: rect.x, top: rect.y, width: rect.width, height: rect.height,
            border: "2px solid var(--accent-primary)",
          }} />
          <div className="absolute font-mono text-xs px-2 py-1 pointer-events-none" style={{
            left: rect.x, top: Math.max(0, rect.y - 28),
            background: "var(--accent-primary)", color: "var(--text-primary)",
          }}>
            {rect.width} × {rect.height}
          </div>
          {[
            { x: rect.x - 4, y: rect.y - 4 },
            { x: rect.x + rect.width - 4, y: rect.y - 4 },
            { x: rect.x - 4, y: rect.y + rect.height - 4 },
            { x: rect.x + rect.width - 4, y: rect.y + rect.height - 4 },
          ].map((pos, i) => (
            <div key={i} className="absolute pointer-events-none" style={{
              left: pos.x, top: pos.y, width: 8, height: 8, background: "var(--accent-primary)",
            }} />
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

// ═══ WINDOW PICKER — compact grid, keyboard + mouse flow ═══
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

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "Enter" && windows.length > 0) {
        e.preventDefault();
        onSelect(windows[selected]);
      }
      if (e.key === "ArrowRight" || e.key === "Tab") {
        e.preventDefault();
        setSelected((s) => (s + 1) % windows.length);
      }
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        setSelected((s) => (s - 1 + windows.length) % windows.length);
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((s) => Math.min(s + 3, windows.length - 1));
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((s) => Math.max(s - 3, 0));
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [windows, selected, onSelect, onClose]);

  const handleDoubleClick = (w: WindowInfo) => {
    onSelect(w);
  };

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
        className="w-full max-w-2xl max-h-[70vh] overflow-hidden"
        style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)" }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-5 py-3 flex items-center justify-between" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <div>
            <div className="font-mono text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
              Select Window
            </div>
            <div className="font-mono mt-0.5" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
              Arrow keys + Enter · Double-click to select · Esc to cancel
            </div>
          </div>
          <span className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>
            {windows.length} windows
          </span>
        </div>

        {/* Window Grid — compact cards */}
        <div className="p-4 overflow-y-auto max-h-[55vh]">
          {loading ? (
            <div className="text-center py-8 font-mono text-xs" style={{ color: "var(--text-muted)" }}>
              Scanning windows...
            </div>
          ) : windows.length === 0 ? (
            <div className="text-center py-8 font-mono text-xs" style={{ color: "var(--text-muted)" }}>
              No capturable windows found
            </div>
          ) : (
            <div className="grid grid-cols-4 gap-2">
              {windows.map((w, i) => (
                <motion.button
                  key={i}
                  className="text-left p-2 cursor-pointer group"
                  style={{
                    border: `var(--border-thin) solid ${i === selected ? "var(--accent-primary)" : "var(--border-default)"}`,
                    background: i === selected ? "var(--bg-elevated)" : "var(--bg-base)",
                  }}
                  whileHover={{ borderColor: "var(--accent-primary)", background: "var(--bg-elevated)" }}
                  whileTap={{ scale: 0.97 }}
                  onClick={() => onSelect(w)}
                  onDoubleClick={() => handleDoubleClick(w)}
                  onMouseEnter={() => setSelected(i)}
                >
                  {/* Window preview placeholder */}
                  <div className="w-full h-16 mb-1.5 flex items-center justify-center" style={{
                    background: "var(--bg-surface)",
                    border: "var(--border-thin) solid var(--border-default)",
                  }}>
                    <span className="text-lg opacity-50">🪟</span>
                  </div>
                  <div className="font-mono truncate" style={{
                    color: i === selected ? "var(--accent-primary)" : "var(--text-primary)",
                    fontSize: "0.6rem",
                    fontWeight: i === selected ? 600 : 400,
                  }}>
                    {w.title.length > 25 ? w.title.substring(0, 25) + "…" : w.title}
                  </div>
                  <div className="font-mono" style={{ color: "var(--text-muted)", fontSize: "0.5rem" }}>
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
