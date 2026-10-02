import { useState, useEffect, useCallback } from "react";
import { motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

// ═══ REGION SELECTOR — Fullscreen overlay for selecting any area on screen ═══
export function RegionSelector({ shot, onComplete, onCancel }: {
  shot: string | null; // desktop screenshot taken just before the selector opened
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
      className="fixed inset-0 z-[9999] cursor-crosshair select-none"
      style={{
        backgroundColor: "#000",
        backgroundImage: shot ? `url(${shot})` : undefined,
        backgroundSize: "100% 100%",
      }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {!rect && <div className="absolute inset-0" style={{ background: "oklch(0.08 0.01 260 / 0.45)" }} />}

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
            {Math.round(rect.width * devicePixelRatio)} × {Math.round(rect.height * devicePixelRatio)}
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

// ═══ WINDOW PICKER — choose an open window for Window mode ═══
export function WindowPicker({ windows, onPick, onCancel }: {
  windows: { title: string; width: number; height: number; hwnd: number }[];
  onPick: (index: number) => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onCancel(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center p-6"
      style={{ background: "oklch(0.08 0.01 260 / 0.6)" }} onClick={onCancel}>
      <motion.div
        initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 12 }}
        transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
        className="w-full max-w-lg flex flex-col"
        style={{ maxHeight: "80vh", background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)", borderRadius: "var(--radius-md)", boxShadow: "var(--shadow-lg)" }}
        onClick={(e) => e.stopPropagation()}
        role="dialog" aria-label="Choose a window to record"
      >
        <div className="px-5 py-4 flex items-center justify-between" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <span className="font-mono text-xs font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>Choose a window</span>
          <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>ESC to cancel</span>
        </div>
        <div className="overflow-y-auto p-2">
          {windows.length === 0 && (
            <p className="p-4 font-mono text-xs" style={{ color: "var(--text-muted)" }}>No windows found. Open the app you want to record, then try again.</p>
          )}
          {windows.map((w, i) => (
            <button key={w.hwnd} onClick={() => onPick(i)}
              className="w-full text-left px-3 py-2.5 flex items-center justify-between gap-3 cursor-pointer hover:bg-[var(--bg-elevated)] focus:bg-[var(--bg-elevated)] outline-none"
              style={{ borderRadius: "var(--radius-sm)" }}>
              <span className="text-sm truncate" style={{ color: "var(--text-primary)" }}>{w.title}</span>
              <span className="font-mono text-[10px] shrink-0" style={{ color: "var(--text-muted)" }}>{w.width}×{w.height}</span>
            </button>
          ))}
        </div>
      </motion.div>
    </div>
  );
}
