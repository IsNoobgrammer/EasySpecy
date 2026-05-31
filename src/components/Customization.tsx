import { useState, useEffect, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";
import {
  TrailRenderer, ClickEffectRenderer, drawPreviewBackground,
  type TrailStyle, type ClickEffect,
} from "../lib/effects";

interface CursorPackInfo {
  id: string;
  name: string;
  description: string;
  author: string;
  is_builtin: boolean;
}

const PACK_COLORS: Record<string, string> = {
  default: "#8b949e", macos: "#f5f5f7", posy: "#e8e8e8",
  neon_green: "#00ff88", neon_pink: "#ff44cc", minimal_dot: "#ffffff",
  crosshair: "#ff4444", retro_pixel: "#ffff00", glass_arrow: "#aabbff",
  easyspecy: "#00e88a",
};

const TRAIL_STYLES: { id: TrailStyle; label: string; desc: string; icon: string }[] = [
  { id: "glow", label: "Glow", desc: "Soft luminous bloom", icon: "✨" },
  { id: "particles", label: "Spark", desc: "Scattered firefly sparks", icon: "⚡" },
  { id: "ribbon", label: "Ribbon", desc: "Smooth flowing band", icon: "🌊" },
  { id: "dots", label: "Dots", desc: "Connected dot trail", icon: "🔵" },
  { id: "aurora", label: "Aurora", desc: "Rainbow flowing wave", icon: "🌈" },
  { id: "none", label: "Off", desc: "No trail effect", icon: "○" },
];

const CLICK_EFFECTS: { id: ClickEffect; label: string; desc: string; icon: string }[] = [
  { id: "ripple", label: "Ripple", desc: "Expanding water rings", icon: "◎" },
  { id: "spotlight", label: "Spotlight", desc: "Radial light flare", icon: "✦" },
  { id: "ring", label: "Ring", desc: "Contract + expand ring", icon: "◯" },
  { id: "pulse", label: "Pulse", desc: "Pulsing energy waves", icon: "◉" },
  { id: "confetti", label: "Confetti", desc: "Burst of particles", icon: "✧" },
  { id: "none", label: "None", desc: "No click effect", icon: "○" },
];

// ─── Simulated cursor path for preview ──────────────────────────

function generateDemoPath(w: number, h: number, t: number): { x: number; y: number } {
  const cx = w / 2, cy = h / 2;
  const rx = w * 0.32, ry = h * 0.28;
  // Figure-8 (lemniscate) path — smooth, organic
  const speed = 0.018;
  const angle = t * speed;
  const x = cx + rx * Math.sin(angle);
  const y = cy + ry * Math.sin(angle * 2) * 0.7;
  return { x, y };
}

export function Customization({ onBack: _onBack }: { onBack: () => void }) {
  const [packs, setPacks] = useState<CursorPackInfo[]>([]);
  const [selectedPack, setSelectedPack] = useState("default");
  const [trailStyle, setTrailStyle] = useState<TrailStyle>("glow");
  const [trailColor, setTrailColor] = useState("#00e88a");
  const [clickEffect, setClickEffect] = useState<ClickEffect>("ripple");
  const [clickColor, setClickColor] = useState("#00e88a");
  const [previewActive, setPreviewActive] = useState(false);

  const trailCanvasRef = useRef<HTMLCanvasElement>(null);
  const clickCanvasRef = useRef<HTMLCanvasElement>(null);
  const trailRenderer = useRef(new TrailRenderer());
  const clickRenderer = useRef(new ClickEffectRenderer());
  const tickRef = useRef(0);

  // Load saved config
  useEffect(() => {
    invoke<CursorPackInfo[]>("get_cursor_packs").then(setPacks).catch(() => {});
    invoke<any>("get_config").then((cfg: any) => {
      if (cfg.cursor_pack) setSelectedPack(cfg.cursor_pack);
      if (cfg.trail_style) setTrailStyle(cfg.trail_style);
      if (cfg.click_effect) setClickEffect(cfg.click_effect);
      if (cfg.cursor_trail_color) {
        setTrailColor(cfg.cursor_trail_color);
        setClickColor(cfg.cursor_trail_color);
      }
    }).catch(() => {});
  }, []);

  // Sync renderer settings
  useEffect(() => {
    trailRenderer.current.setStyle(trailStyle);
    trailRenderer.current.setColor(trailColor);
  }, [trailStyle, trailColor]);

  useEffect(() => {
    clickRenderer.current.setStyle(clickEffect);
    clickRenderer.current.setColor(clickColor);
    // Start/stop auto-simulate for click preview
    const canvas = clickCanvasRef.current;
    if (canvas && clickEffect !== "none") {
      clickRenderer.current.startAutoSimulate(canvas);
    } else {
      clickRenderer.current.stopAutoSimulate();
    }
    return () => { clickRenderer.current.stopAutoSimulate(); };
  }, [clickEffect, clickColor]);

  // Animation loop — runs both trail + click canvases
  useEffect(() => {
    const trailCanvas = trailCanvasRef.current;
    const clickCanvas = clickCanvasRef.current;
    if (!trailCanvas || !clickCanvas) return;

    const trailCtx = trailCanvas.getContext("2d")!;
    const clickCtx = clickCanvas.getContext("2d")!;
    let raf: number;

    const animate = () => {
      tickRef.current++;
      const tw = trailCanvas.width;
      const th = trailCanvas.height;
      const cw = clickCanvas.width;
      const ch = clickCanvas.height;

      // ── Trail: simulate cursor movement along demo path ──
      const { x, y } = generateDemoPath(tw, th, tickRef.current);
      trailRenderer.current.addPoint(x, y);
      trailRenderer.current.update();

      trailCtx.clearRect(0, 0, tw, th);
      drawPreviewBackground(trailCtx, tw, th);
      trailRenderer.current.draw(trailCtx, tw, th);

      // Draw small cursor indicator at head
      const headAlpha = trailStyle === "none" ? 0.6 : 0.9;
      trailCtx.beginPath();
      trailCtx.arc(x, y, 3, 0, Math.PI * 2);
      trailCtx.fillStyle = `rgba(255,255,255,${headAlpha})`;
      trailCtx.fill();

      // ── Click effects ──
      clickRenderer.current.update();
      clickCtx.clearRect(0, 0, cw, ch);
      drawPreviewBackground(clickCtx, cw, ch);
      clickRenderer.current.draw(clickCtx, cw, ch);

      raf = requestAnimationFrame(animate);
    };

    animate();
    return () => cancelAnimationFrame(raf);
  }, [trailStyle]); // Re-mount animation when style changes

  // Manual trail hover (overrides demo path when user moves mouse)
  const handleTrailHover = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (trailStyle === "none") return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    trailRenderer.current.addPoint(x, y);
  }, [trailStyle]);

  // Manual click on click preview canvas
  const handleClickPreview = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (clickEffect === "none") return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    clickRenderer.current.addClick(x, y);
  }, [clickEffect]);

  const handleSelectPack = async (packId: string) => {
    setSelectedPack(packId);
    try { await invoke("update_config_field", { key: "cursor_pack", value: packId }); } catch {}
  };

  const handleTrailStyleChange = async (id: TrailStyle) => {
    setTrailStyle(id);
    trailRenderer.current.clear();
    try { await invoke("update_config_field", { key: "trail_style", value: id }); } catch {}
  };

  const handleClickEffectChange = async (id: ClickEffect) => {
    setClickEffect(id);
    clickRenderer.current.clear();
    try { await invoke("update_config_field", { key: "click_effect", value: id }); } catch {}
  };

  const handleTrailColorChange = async (color: string) => {
    setTrailColor(color);
    setClickColor(color);
    try { await invoke("update_config_field", { key: "cursor_trail_color", value: color }); } catch {}
  };

  const handlePreview = async () => {
    if (previewActive) {
      await invoke("restore_cursors").catch(() => {});
      setPreviewActive(false);
    } else {
      await invoke("apply_cursor_pack", { packId: selectedPack }).catch(() => {});
      setPreviewActive(true);
      setTimeout(async () => { await invoke("restore_cursors").catch(() => {}); setPreviewActive(false); }, 5000);
    }
  };

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="max-w-[680px] w-full mx-auto py-8 px-5 space-y-6">

        {/* ═══ HEADER ═══ */}
        <motion.div initial={{ opacity: 0, y: -8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}>
          <h2 className="text-[22px] font-semibold tracking-tight" style={{ color: "var(--text-primary)" }}>Customization</h2>
          <p className="text-[13px] mt-1" style={{ color: "var(--text-secondary)" }}>Shape how your recordings look and feel.</p>
        </motion.div>

        {/* ═══ CURSOR PACKS ═══ */}
        <motion.section initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.05, duration: 0.4, ease: [0.16, 1, 0.3, 1] }} className="overflow-hidden" style={{ border: "1px solid var(--border-default)", borderRadius: "12px", background: "var(--bg-surface)" }}>
          <div className="flex justify-between items-center px-5 py-3" style={{ borderBottom: "1px solid var(--border-default)" }}>
            <div className="flex items-center gap-2.5">
              <div className="w-7 h-7 flex items-center justify-center rounded-md" style={{ background: PACK_COLORS[selectedPack] + "18" }}>
                <Icon name="capture" size={15} style={{ color: PACK_COLORS[selectedPack] || "var(--accent-primary)" }} />
              </div>
              <div>
                <span className="font-mono text-[11px] font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.04em" }}>Cursor Pack</span>
                <span className="font-mono text-[10px] ml-2" style={{ color: "var(--text-muted)" }}>· {packs.find(p => p.id === selectedPack)?.name || selectedPack}</span>
              </div>
            </div>
            <motion.button onClick={handlePreview} className="font-mono text-[10px] px-3 py-1.5 cursor-pointer font-bold uppercase rounded-md" style={{ border: `1px solid ${previewActive ? "#f04040" : PACK_COLORS[selectedPack] || "#00e88a"}`, color: previewActive ? "#f04040" : PACK_COLORS[selectedPack] || "#00e88a", background: previewActive ? "rgba(240,64,64,0.06)" : "transparent" }} whileHover={{ scale: 1.03 }} whileTap={{ scale: 0.97 }}>
              {previewActive ? "■ STOP" : "▶ LIVE 5s"}
            </motion.button>
          </div>
          <div className="p-4">
            <div className="grid grid-cols-2 gap-2.5">
              {packs.map((pack, i) => {
                const active = selectedPack === pack.id;
                const color = PACK_COLORS[pack.id] || "#888";
                return (
                  <motion.button key={pack.id} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.06 + i * 0.025, ease: [0.16, 1, 0.3, 1] }} onClick={() => handleSelectPack(pack.id)} className="text-left p-3.5 cursor-pointer relative group" style={{ border: active ? `1.5px solid ${color}` : "1px solid var(--border-default)", background: active ? `${color}08` : "var(--surface-container-low, var(--surface-container-low))", borderRadius: "10px", boxShadow: active ? `0 0 20px ${color}12, inset 0 0 20px ${color}06` : "none", transition: "all 0.2s cubic-bezier(0.16, 1, 0.3, 1)" }} whileHover={{ y: -2, boxShadow: `0 4px 16px ${color}15` }} whileTap={{ scale: 0.98 }}>
                    <div className="flex items-center justify-between mb-2">
                      <span className="w-3 h-3 rounded-full" style={{ background: color, boxShadow: active ? `0 0 10px ${color}` : "none", transition: "box-shadow 0.3s" }} />
                      <AnimatePresence>
                        {active && <motion.span initial={{ scale: 0, rotate: -90 }} animate={{ scale: 1, rotate: 0 }} exit={{ scale: 0 }} transition={{ type: "spring", stiffness: 500, damping: 25 }}><Icon name="check_circle" size={14} style={{ color }} /></motion.span>}
                      </AnimatePresence>
                    </div>
                    <div className="font-mono text-[12px] font-semibold" style={{ color: active ? "var(--text-primary)" : "var(--text-secondary)" }}>{pack.name}</div>
                    <div className="font-mono text-[10px] mt-0.5 leading-tight" style={{ color: "var(--text-muted)", opacity: 0.8 }}>{pack.description}</div>
                  </motion.button>
                );
              })}
            </div>
          </div>
          <div className="px-5 py-2" style={{ borderTop: "1px solid var(--border-default)", background: "var(--bg-elevated)" }}>
            <p className="font-mono text-[9px] text-center" style={{ color: "var(--text-muted)" }}>
              Custom packs → <span style={{ color: "var(--accent-primary)" }}>~/.config/easyspecy/cursors/your-pack/</span>
            </p>
          </div>
        </motion.section>

        {/* ═══ TRAIL EFFECTS ═══ */}
        <motion.section initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.12, duration: 0.4, ease: [0.16, 1, 0.3, 1] }} className="overflow-hidden" style={{ border: "1px solid var(--border-default)", borderRadius: "12px", background: "var(--bg-surface)" }}>
          <div className="flex justify-between items-center px-5 py-3" style={{ borderBottom: "1px solid var(--border-default)" }}>
            <div className="flex items-center gap-2.5">
              <div className="w-7 h-7 flex items-center justify-center rounded-md" style={{ background: trailColor + "18" }}>
                <Icon name="auto_fix_high" size={15} style={{ color: trailColor }} />
              </div>
              <div>
                <span className="font-mono text-[11px] font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.04em" }}>Trail Effect</span>
                <span className="font-mono text-[10px] ml-2" style={{ color: "var(--text-muted)" }}>· {TRAIL_STYLES.find(s => s.id === trailStyle)?.label}</span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <input type="color" value={trailColor} onChange={(e) => handleTrailColorChange(e.target.value)} className="w-5 h-5 cursor-pointer rounded" style={{ border: "1px solid var(--border-default)", background: "transparent" }} />
              <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>{trailColor.toUpperCase()}</span>
            </div>
          </div>

          {/* Style pills */}
          <div className="px-5 py-3 flex flex-wrap gap-1.5" style={{ borderBottom: "1px solid var(--border-default)" }}>
            {TRAIL_STYLES.map((s) => (
              <motion.button key={s.id} onClick={() => handleTrailStyleChange(s.id)} className="px-3 py-1.5 font-mono text-[10px] font-bold uppercase cursor-pointer rounded-md flex items-center gap-1" style={{ background: trailStyle === s.id ? trailColor : "transparent", color: trailStyle === s.id ? "#0d0f1a" : "var(--text-secondary)", border: trailStyle === s.id ? "none" : "1px solid var(--border-default)", transition: "all 0.15s" }} whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
                <span style={{ fontSize: "11px" }}>{s.icon}</span>
                {s.label}
              </motion.button>
            ))}
          </div>

          {/* Live trail canvas */}
          <div className="p-4">
            <div className="relative overflow-hidden" style={{ borderRadius: "8px", background: "var(--bg-elevated)", border: "1px dashed var(--border-default)" }}>
              <canvas ref={trailCanvasRef} width={620} height={140} className="w-full cursor-crosshair" style={{ height: "140px", display: "block" }} onMouseMove={handleTrailHover} />
              {trailStyle === "none" && (
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>Trail disabled</span>
                </div>
              )}
              {trailStyle !== "none" && (
                <div className="absolute bottom-2 right-3 pointer-events-none">
                  <span className="font-mono text-[9px] px-2 py-0.5 rounded" style={{ color: "var(--text-muted)", background: "rgba(0,0,0,0.3)" }}>Move cursor to interact</span>
                </div>
              )}
            </div>
            {/* Style description */}
            <div className="mt-2 flex items-center gap-2">
              <span className="text-[11px]">{TRAIL_STYLES.find(s => s.id === trailStyle)?.icon}</span>
              <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>{TRAIL_STYLES.find(s => s.id === trailStyle)?.desc}</span>
            </div>
          </div>
        </motion.section>

        {/* ═══ CLICK EFFECTS ═══ */}
        <motion.section initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.18, duration: 0.4, ease: [0.16, 1, 0.3, 1] }} className="overflow-hidden" style={{ border: "1px solid var(--border-default)", borderRadius: "12px", background: "var(--bg-surface)" }}>
          <div className="flex justify-between items-center px-5 py-3" style={{ borderBottom: "1px solid var(--border-default)" }}>
            <div className="flex items-center gap-2.5">
              <div className="w-7 h-7 flex items-center justify-center rounded-md" style={{ background: clickColor + "18" }}>
                <Icon name="radio_button_checked" size={15} style={{ color: clickColor }} />
              </div>
              <div>
                <span className="font-mono text-[11px] font-bold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.04em" }}>Click Effect</span>
                <span className="font-mono text-[10px] ml-2" style={{ color: "var(--text-muted)" }}>· {CLICK_EFFECTS.find(s => s.id === clickEffect)?.label}</span>
              </div>
            </div>
            <span className="font-mono text-[9px] px-2 py-0.5 rounded-full" style={{ color: "var(--accent-primary)", background: "rgba(0,232,138,0.08)", border: "1px solid rgba(0,232,138,0.15)" }}>PHASE 2</span>
          </div>

          {/* Effect grid */}
          <div className="p-4 pb-2">
            <div className="grid grid-cols-3 gap-2">
              {CLICK_EFFECTS.map((effect) => {
                const active = clickEffect === effect.id;
                return (
                  <motion.button key={effect.id} onClick={() => handleClickEffectChange(effect.id)} className="flex flex-col items-center justify-center gap-1 py-3.5 cursor-pointer" style={{ border: active ? `1.5px solid ${clickColor}` : "1px solid var(--border-default)", background: active ? `${clickColor}08` : "var(--surface-container-low)", borderRadius: "8px", boxShadow: active ? `0 0 12px ${clickColor}12` : "none", opacity: effect.id === "none" || active ? 1 : 0.7, transition: "all 0.2s" }} whileHover={{ scale: 1.03, opacity: 1 }} whileTap={{ scale: 0.96 }}>
                    <span className="text-[16px]" style={{ filter: active ? "none" : "grayscale(0.6)" }}>{effect.icon}</span>
                    <span className="font-mono text-[9px] font-bold uppercase" style={{ color: active ? clickColor : "var(--text-muted)", letterSpacing: "0.05em" }}>{effect.label}</span>
                    <span className="font-mono text-[8px]" style={{ color: "var(--text-muted)", opacity: 0.6 }}>{effect.desc}</span>
                  </motion.button>
                );
              })}
            </div>
          </div>

          {/* Click preview canvas */}
          <div className="px-4 pb-4">
            <div className="relative overflow-hidden" style={{ borderRadius: "8px", background: "var(--bg-elevated)", border: "1px dashed var(--border-default)" }}>
              <canvas ref={clickCanvasRef} width={620} height={140} className="w-full cursor-pointer" style={{ height: "140px", display: "block" }} onClick={handleClickPreview} />
              {clickEffect === "none" && (
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>Click effect disabled</span>
                </div>
              )}
              {clickEffect !== "none" && (
                <div className="absolute bottom-2 right-3 pointer-events-none">
                  <span className="font-mono text-[9px] px-2 py-0.5 rounded" style={{ color: "var(--text-muted)", background: "rgba(0,0,0,0.3)" }}>Click or wait for preview</span>
                </div>
              )}
            </div>
            <div className="mt-2 flex items-center gap-2">
              <span className="text-[11px]">{CLICK_EFFECTS.find(s => s.id === clickEffect)?.icon}</span>
              <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>{CLICK_EFFECTS.find(s => s.id === clickEffect)?.desc}</span>
            </div>
          </div>
        </motion.section>

        <div className="h-6" />
      </div>
    </div>
  );
}
