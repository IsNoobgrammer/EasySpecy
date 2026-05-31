import { useState, useRef, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { invoke } from "@tauri-apps/api/core";

interface Point {
  x: number;
  y: number;
}

interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function RegionSelector({ onComplete, onCancel }: {
  onComplete: (region: Region) => void;
  onCancel: () => void;
}) {
  const [start, setStart] = useState<Point | null>(null);
  const [end, setEnd] = useState<Point | null>(null);
  const [selecting, setSelecting] = useState(false);
  const overlayRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    setStart({ x: e.clientX, y: e.clientY });
    setEnd({ x: e.clientX, y: e.clientY });
    setSelecting(true);
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (selecting) {
      setEnd({ x: e.clientX, y: e.clientY });
    }
  }, [selecting]);

  const handleMouseUp = useCallback(() => {
    if (start && end) {
      const x = Math.min(start.x, end.x);
      const y = Math.min(start.y, end.y);
      const width = Math.abs(end.x - start.x);
      const height = Math.abs(end.y - start.y);

      if (width > 50 && height > 50) {
        onComplete({ x, y, width, height });
      } else {
        onCancel();
      }
    }
    setSelecting(false);
  }, [start, end, onComplete, onCancel]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onCancel]);

  const region = start && end ? {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  } : null;

  return (
    <div
      ref={overlayRef}
      className="fixed inset-0 z-[9999] cursor-crosshair"
      style={{ background: "oklch(0 0 0 / 0.5)" }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {/* Instructions */}
      <AnimatePresence>
        {!selecting && !region && (
          <motion.div
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="absolute top-8 left-1/2 -translate-x-1/2 px-6 py-3"
            style={{
              background: "var(--bg-surface)",
              border: "var(--border-width) solid var(--border-default)",
            }}
          >
            <span className="font-mono text-sm" style={{ color: "var(--text-primary)" }}>
              Drag to select recording area · Press Esc to cancel
            </span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Selection rectangle */}
      {region && region.width > 0 && region.height > 0 && (
        <>
          {/* Clear area (selected region) */}
          <div
            className="absolute"
            style={{
              left: region.x,
              top: region.y,
              width: region.width,
              height: region.height,
              background: "transparent",
              boxShadow: "0 0 0 9999px oklch(0 0 0 / 0.5)",
            }}
          />
          {/* Border */}
          <div
            className="absolute pointer-events-none"
            style={{
              left: region.x,
              top: region.y,
              width: region.width,
              height: region.height,
              border: "2px solid var(--accent-primary)",
            }}
          />
          {/* Size indicator */}
          <div
            className="absolute font-mono text-xs px-2 py-1 pointer-events-none"
            style={{
              left: region.x,
              top: region.y - 28,
              background: "var(--accent-primary)",
              color: "var(--text-primary)",
            }}
          >
            {region.width} × {region.height}
          </div>
          {/* Corner handles */}
          {[
            { x: region.x - 4, y: region.y - 4 },
            { x: region.x + region.width - 4, y: region.y - 4 },
            { x: region.x - 4, y: region.y + region.height - 4 },
            { x: region.x + region.width - 4, y: region.y + region.height - 4 },
          ].map((pos, i) => (
            <div
              key={i}
              className="absolute pointer-events-none"
              style={{
                left: pos.x,
                top: pos.y,
                width: 8,
                height: 8,
                background: "var(--accent-primary)",
              }}
            />
          ))}
        </>
      )}
    </div>
  );
}

// ═══ WINDOW PICKER ═══
export interface WindowInfo {
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  hwnd: number;
}

export function WindowPicker({ onSelect, onClose }: {
  onSelect: (window: WindowInfo) => void;
  onClose: () => void;
}) {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<WindowInfo[]>("get_windows").then((w) => {
      setWindows(w);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[9999] flex items-center justify-center"
      style={{ background: "oklch(0 0 0 / 0.7)" }}
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95, y: 20 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.95, y: 20 }}
        className="w-full max-w-lg max-h-[70vh] overflow-hidden"
        style={{
          background: "var(--bg-surface)",
          border: "var(--border-width) solid var(--border-default)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <span className="font-mono text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
            Select Window
          </span>
          <motion.button
            onClick={onClose}
            className="font-mono text-xs cursor-pointer"
            style={{ color: "var(--text-muted)" }}
            whileHover={{ color: "var(--text-primary)" }}
          >
            Esc to close
          </motion.button>
        </div>

        {/* Window List */}
        <div className="overflow-y-auto max-h-[50vh] p-2 space-y-1">
          {loading ? (
            <div className="p-8 text-center font-mono text-xs" style={{ color: "var(--text-muted)" }}>
              Enumerating windows...
            </div>
          ) : windows.length === 0 ? (
            <div className="p-8 text-center font-mono text-xs" style={{ color: "var(--text-muted)" }}>
              No windows found
            </div>
          ) : (
            windows.map((w, i) => (
              <motion.button
                key={i}
                className="w-full text-left px-3 py-2.5 cursor-pointer flex items-center justify-between"
                style={{
                  border: "var(--border-thin) solid var(--border-default)",
                  background: "var(--bg-base)",
                }}
                whileHover={{ borderColor: "var(--accent-primary)", background: "var(--bg-elevated)" }}
                whileTap={{ scale: 0.98 }}
                onClick={() => onSelect(w)}
              >
                <div className="flex-1 min-w-0">
                  <div className="font-mono text-xs truncate" style={{ color: "var(--text-primary)" }}>
                    {w.title}
                  </div>
                  <div className="font-mono mt-0.5" style={{ color: "var(--text-muted)", fontSize: "0.55rem" }}>
                    {w.width}×{w.height}
                  </div>
                </div>
                <span className="font-mono text-xs ml-2" style={{ color: "var(--accent-primary)" }}>→</span>
              </motion.button>
            ))
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}
