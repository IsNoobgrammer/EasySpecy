import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useShallow } from "zustand/react/shallow";
import { useStore, type QualitySample } from "../stores/recording";

const LABEL: Record<string, string> = {
  Low: "Low", Medium: "Medium", High: "High", Ultra: "Ultra", Smaller: "Smaller file", Custom: "Custom",
};
const NOTE: Record<string, string> = {
  Low: "Text may smear while scrolling",
  Medium: "Good for most screen recordings — the default",
  High: "Crisper text while things move",
  Ultra: "Near-lossless, big files",
  Smaller: "Smaller video, takes longer to save",
  Custom: "Your own bitrate",
};

/** Shows what each quality setting looks like on the user's own screen, so they can pick by eye
 *  instead of guessing what "5 Mbps" means. */
export function QualityPreview({ onClose }: { onClose: () => void }) {
  const { config, saveConfig } = useStore(useShallow((s) => ({ config: s.config, saveConfig: s.saveConfig })));
  const [samples, setSamples] = useState<QualitySample[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [customMbps, setCustomMbps] = useState(8);
  const [busy, setBusy] = useState(false);
  const [zoom, setZoom] = useState<QualitySample | null>(null);
  // Backend milestones: jump to `done`, then glide linearly to `towards` over `secs`
  const [bar, setBar] = useState({ width: 0, secs: 0, label: "Starting…" });
  useEffect(() => {
    const un = listen<[number, number, string, number]>("quality-preview-progress", ({ payload: [done, towards, label, secs] }) => {
      setBar({ width: done, secs: 0, label });
      requestAnimationFrame(() => requestAnimationFrame(() => setBar({ width: towards, secs, label })));
    });
    return () => { un.then((f) => f()); };
  }, []);

  const render = async (custom?: number) => {
    setBusy(true); setError(null); setSamples(null); setBar({ width: 0, secs: 0, label: "Starting…" });
    try { setSamples(await invoke<QualitySample[]>("quality_preview", { customKbps: custom ? Math.round(custom * 1000) : null })); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  useEffect(() => { render(); }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") (zoom ? setZoom(null) : onClose()); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, zoom]);

  const choose = async (s: QualitySample) => {
    if (!config) return;
    if (s.id === "Smaller") await saveConfig({ ...config, compact_output: true });
    else if (s.id === "Custom") await saveConfig({ ...config, video_quality: "Custom", video_bitrate_kbps: s.kbps, compact_output: false });
    else await saveConfig({ ...config, video_quality: s.id as typeof config.video_quality, compact_output: false });
    onClose();
  };
  const current = config?.compact_output ? "Smaller" : config?.video_quality;

  return (
    <div className="fixed inset-0 z-[9998] flex items-center justify-center p-6" style={{ background: "oklch(0.08 0.01 260 / 0.7)" }} onClick={onClose}>
      <motion.div
        initial={{ opacity: 0, scale: 0.97 }} animate={{ opacity: 1, scale: 1 }} transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
        className="w-full max-w-4xl max-h-[90vh] flex flex-col"
        style={{ background: "var(--bg-surface)", border: "var(--border-width) solid var(--border-default)", borderRadius: "var(--radius-md)", boxShadow: "var(--shadow-lg)" }}
        onClick={(e) => e.stopPropagation()} role="dialog" aria-label="Compare video quality"
      >
        <div className="px-5 py-3" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <div className="font-mono text-xs font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>How each quality looks on your screen</div>
          <div className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
            Each tile is a real 1:1 crop of your screen, encoded exactly like a recording and shown right after the screen changes, when lower quality blurs most. Click a tile to zoom.
          </div>
        </div>

        <div className="overflow-y-auto p-4">
          {error && <p className="font-mono text-xs p-3" style={{ color: "var(--accent-danger)" }}>Couldn't render previews: {error}</p>}
          {!samples && !error && (
            <div className="p-8 flex flex-col items-center gap-3" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(bar.width * 100)} aria-label="Rendering quality previews">
              <div className="w-full max-w-md h-1.5 rounded-full overflow-hidden" style={{ background: "var(--bg-elevated)" }}>
                <div className="h-full w-full origin-left" style={{ transform: `scaleX(${bar.width})`, transition: `transform ${bar.secs}s linear`, background: "var(--accent-primary)" }} />
              </div>
              <span className="font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>{bar.label}</span>
            </div>
          )}
          {samples && (
            <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
              {samples.map((s) => (
                <div key={s.id} className="flex flex-col" style={{ border: `var(--border-width) solid ${current === s.id ? "var(--accent-primary)" : "var(--border-default)"}`, borderRadius: "var(--radius-sm)", overflow: "hidden" }}>
                  <button type="button" onClick={() => setZoom(s)} className="cursor-zoom-in bg-black" aria-label={`Zoom ${LABEL[s.id]} preview`}>
                    <img src={s.image} alt={`${LABEL[s.id]} quality sample`} className="w-full block" />
                  </button>
                  <div className="p-2.5 flex flex-col gap-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="font-mono text-[11px] font-bold uppercase" style={{ color: "var(--text-primary)" }}>{LABEL[s.id]}{current === s.id ? " · current" : ""}</span>
                      <span className="font-mono text-[10px]" style={{ color: "var(--text-secondary)" }}>≈ {s.mb_per_min.toFixed(0)} MB/min · {(s.kbps / 1000).toFixed(1)} Mbps</span>
                    </div>
                    <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>{NOTE[s.id]}</span>
                    <button type="button" onClick={() => choose(s)} className="self-start mt-1 px-2.5 py-1 font-mono text-[10px] font-bold uppercase cursor-pointer"
                      style={{ border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", color: "var(--text-primary)" }}>
                      Use this
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="px-5 py-3 flex flex-wrap items-center gap-3" style={{ borderTop: "var(--border-thin) solid var(--border-default)" }}>
          <label className="font-mono text-[10px] uppercase" style={{ color: "var(--text-secondary)" }} htmlFor="custom-mbps">Try a custom bitrate</label>
          <input id="custom-mbps" type="range" min={1} max={30} step={0.5} value={customMbps} onChange={(e) => setCustomMbps(Number(e.target.value))} className="flex-1 min-w-[160px]" />
          <span className="font-mono text-[11px] w-28" style={{ color: "var(--text-primary)" }}>{customMbps} Mbps ≈ {(customMbps * 1000 * 60 / 8 / 1024).toFixed(0)} MB/min</span>
          <button type="button" disabled={busy} onClick={() => render(customMbps)} className="px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer disabled:opacity-50"
            style={{ border: "var(--border-thin) solid var(--border-default)", borderRadius: "var(--radius-sm)", color: "var(--text-primary)" }}>
            {busy ? "Rendering…" : "Render custom"}
          </button>
          <button type="button" onClick={onClose} className="px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer"
            style={{ background: "var(--accent-primary)", color: "#003d22", borderRadius: "var(--radius-sm)" }}>
            Close
          </button>
        </div>
      </motion.div>

      {zoom && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center p-6" style={{ background: "oklch(0.05 0 0 / 0.85)" }} onClick={(e) => { e.stopPropagation(); setZoom(null); }}>
          <img src={zoom.image} alt={`${LABEL[zoom.id]} quality, zoomed`} style={{ width: "min(100%, 1280px)", imageRendering: "pixelated" }} />
        </div>
      )}
    </div>
  );
}
