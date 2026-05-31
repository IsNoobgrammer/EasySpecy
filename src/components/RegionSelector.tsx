import { useState, useEffect, useCallback } from "react";
import { motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

// ═══ REGION SELECTOR — Fullscreen overlay for selecting any area on screen ═══
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
    const rect = {
      x: Math.min(start.x, end.x),
      y: Math.min(start.y, end.y),
      width: Math.abs(end.x - start.x),
      height: Math.abs(end.y - start.y),
    };
    if (rect.width > 30 && rect.height > 30) {
      onComplete(rect);
    }
  }, [start, end, onComplete]);

  const rect = start && end ? {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  } : null;

  return (
    <div
      className="fixed inset-0 z-[9999] cursor-crosshair"
      style={{ background: "oklch(0.08 0.01 260 / 0.55)" }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {/* Instruction Banner */}
      {!selecting && !rect && (
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
          className="absolute top-8 left-1/2 -translate-x-1/2 px-6 py-3 flex items-center gap-3"
          style={{
            background: "var(--bg-surface)",
            border: "var(--border-width) solid var(--border-default)",
            borderRadius: "var(--radius-md)",
            boxShadow: "var(--shadow-lg)",
          }}
        >
          <Icon name="capture" size={18} style={{ color: "var(--accent-primary)" }} />
          <span className="font-mono text-xs" style={{ color: "var(--text-primary)" }}>
            Drag to select recording area
          </span>
          <span className="font-mono text-xs px-2 py-0.5" style={{
            color: "var(--text-muted)",
            background: "var(--bg-elevated)",
            border: "var(--border-thin) solid var(--border-default)",
            borderRadius: "var(--radius-xs)",
          }}>
            ESC
          </span>
        </motion.div>
      )}

      {/* Selection Rectangle */}
      {rect && rect.width > 0 && rect.height > 0 && (
        <>
          <div className="absolute" style={{
            left: rect.x, top: rect.y, width: rect.width, height: rect.height,
            background: "transparent",
            boxShadow: "0 0 0 9999px oklch(0.08 0.01 260 / 0.55)",
          }} />
          <div className="absolute pointer-events-none" style={{
            left: rect.x, top: rect.y, width: rect.width, height: rect.height,
            border: "2px solid var(--accent-primary)",
            borderRadius: "2px",
            boxShadow: "0 0 12px oklch(0.78 0.18 160 / 0.3), inset 0 0 12px oklch(0.78 0.18 160 / 0.05)",
          }} />
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="absolute font-mono text-xs px-2.5 py-1 pointer-events-none"
            style={{
              left: rect.x, top: Math.max(0, rect.y - 32),
              background: "var(--accent-primary)",
              color: "#003d22",
              fontWeight: 700,
              borderRadius: "var(--radius-xs)",
              letterSpacing: "0.03em",
            }}
          >
            {rect.width} × {rect.height}
          </motion.div>
          {[
            { x: rect.x - 4, y: rect.y - 4 },
            { x: rect.x + rect.width - 4, y: rect.y - 4 },
            { x: rect.x - 4, y: rect.y + rect.height - 4 },
            { x: rect.x + rect.width - 4, y: rect.y + rect.height - 4 },
          ].map((pos, i) => (
            <div key={i} className="absolute pointer-events-none" style={{
              left: pos.x, top: pos.y, width: 8, height: 8,
              background: "var(--accent-primary)",
              borderRadius: "1px",
              boxShadow: "0 0 6px oklch(0.78 0.18 160 / 0.5)",
            }} />
          ))}
        </>
      )}
    </div>
  );
}
