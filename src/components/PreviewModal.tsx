import { useEffect } from "react";
import { motion } from "motion/react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useShallow } from "zustand/react/shallow";
import { useStore, PREVIEW_SECONDS } from "../stores/recording";

/** Plays the temporary preview.mp4 — exactly what a real recording would look like. */
export function PreviewModal() {
  const { previewPath, closePreview, startPreview } = useStore(useShallow((s) => ({
    previewPath: s.previewPath, closePreview: s.closePreview, startPreview: s.startPreview,
  })));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") closePreview(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closePreview]);

  if (!previewPath) return null;
  // Each preview overwrites the same file — the timestamp stops the player serving a stale copy
  const src = `${convertFileSrc(previewPath)}?t=${Date.now()}`;

  return (
    <div className="fixed inset-0 z-[9998] flex items-center justify-center p-6" style={{ background: "oklch(0.08 0.01 260 / 0.7)" }} onClick={closePreview}>
      <motion.div
        initial={{ opacity: 0, scale: 0.97 }} animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
        className="w-full max-w-3xl flex flex-col"
        style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)", borderRadius: "var(--radius-md)", boxShadow: "var(--shadow-lg)" }}
        onClick={(e) => e.stopPropagation()}
        role="dialog" aria-label="Recording preview"
      >
        <div className="px-5 py-3 flex items-center justify-between" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <span className="font-mono text-xs font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>
            Preview — this is how your recording will look
          </span>
          <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>Temporary · deleted when you close the app</span>
        </div>
        <video key={src} src={src} controls autoPlay className="w-full bg-black" style={{ maxHeight: "60vh" }} />
        <div className="px-5 py-3 flex items-center justify-end gap-2">
          <button
            className="px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer"
            style={{ border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", color: "var(--text-secondary)" }}
            onClick={() => { closePreview(); startPreview(); }}
          >
            ↻ Preview again ({PREVIEW_SECONDS}s)
          </button>
          <button
            className="px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer"
            style={{ background: "var(--accent-primary)", color: "#003d22", borderRadius: "var(--radius-sm)" }}
            onClick={closePreview}
          >
            Close
          </button>
        </div>
      </motion.div>
    </div>
  );
}
