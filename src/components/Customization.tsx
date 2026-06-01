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

const TRAIL_STYLES: { id: TrailStyle; label: string; desc: string }[] = [
  { id: "glow", label: "Glow", desc: "Soft luminous bloom that reacts to speed" },
  { id: "particles", label: "Spark", desc: "Embers and fireflies that orbit the cursor" },
  { id: "ribbon", label: "Ribbon", desc: "Multi-layered flowing band with shimmer" },
  { id: "dots", label: "Dots", desc: "Connected halos with white-hot cores" },
  { id: "aurora", label: "Aurora", desc: "Rainbow wave with flowing light layers" },
  { id: "none", label: "Off", desc: "No trail effect" },
];

const CLICK_EFFECTS: { id: ClickEffect; label: string; desc: string }[] = [
  { id: "ripple", label: "Ripple", desc: "Expanding water rings with flash" },
  { id: "spotlight", label: "Spotlight", desc: "Radial flare with cross rays" },
  { id: "ring", label: "Ring", desc: "Double ring — expand and contract" },
  { id: "pulse", label: "Pulse", desc: "Breathing energy waves" },
  { id: "confetti", label: "Confetti", desc: "Burst of mixed-shape particles" },
  { id: "none", label: "None", desc: "No click effect" },
];

// ─── Mini preview canvas for effect cards ────────────────────────

function MiniPreview({
  trailStyle, clickEffect, color,
  onHover, onClick,
}: {
  trailStyle?: TrailStyle;
  clickEffect?: ClickEffect;
  color: string;
  onHover?: (e: React.MouseEvent<HTMLCanvasElement>) => void;
  onClick?: (e: React.MouseEvent<HTMLCanvasElement>) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const trailRef = useRef(new TrailRenderer());
  const clickRef = useRef(new ClickEffectRenderer());
  const timeRef = useRef(0);
  const hasInteracted = useRef(false);

  useEffect(() => {
    if (trailStyle) { trailRef.current.setStyle(trailStyle); trailRef.current.setColor(color); }
    if (clickEffect) { clickRef.current.setStyle(clickEffect); clickRef.current.setColor(color); }
  }, [trailStyle, clickEffect, color]);

  // Auto-simulate clicks for click preview
  useEffect(() => {
    if (!clickEffect || clickEffect === "none") return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    clickRef.current.startAutoSimulate(canvas);
    return () => { clickRef.current.stopAutoSimulate(); };
  }, [clickEffect]);

  // Animation loop — NO auto-cursor movement for trail
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    let raf: number;

    const animate = () => {
      timeRef.current++;
      const w = canvas.width;
      const h = canvas.height;

      ctx.clearRect(0, 0, w, h);
      drawPreviewBackground(ctx, w, h);

      // Trail: only draw what user has created, no auto-animation
      trailRef.current.update();
      trailRef.current.draw(ctx, w, h);

      // Show subtle prompt if trail is active but no interaction yet
      if (trailStyle && trailStyle !== "none" && !hasInteracted.current) {
        const pulse = 0.4 + Math.sin(timeRef.current * 0.04) * 0.15;
        ctx.font = "11px 'JetBrains Mono', monospace";
        ctx.fillStyle = `rgba(255,255,255,${pulse})`;
        ctx.textAlign = "center";
        ctx.fillText("move your cursor here", w / 2, h / 2 + 4);
        ctx.textAlign = "left";
      }

      // Click effects
      clickRef.current.update();
      clickRef.current.draw(ctx, w, h);

      raf = requestAnimationFrame(animate);
    };
    animate();
    return () => cancelAnimationFrame(raf);
  }, [trailStyle, clickEffect]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!trailStyle || trailStyle === "none") return;
    hasInteracted.current = true;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    trailRef.current.addPoint(x, y);
    onHover?.(e);
  }, [trailStyle, onHover]);

  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!clickEffect || clickEffect === "none") return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) * (e.currentTarget.width / rect.width);
    const y = (e.clientY - rect.top) * (e.currentTarget.height / rect.height);
    clickRef.current.addClick(x, y);
    onClick?.(e);
  }, [clickEffect, onClick]);

  return (
    <canvas
      ref={canvasRef}
      width={620}
      height={140}
      className="w-full cursor-crosshair"
      style={{
        height: "140px",
        display: "block",
        borderRadius: "8px",
      }}
      onMouseMove={handleMouseMove}
      onClick={handleClick}
    />
  );
}

// ─── Section wrapper ─────────────────────────────────────────────

function Section({
  icon, title, badge, children, delay = 0, accent,
}: {
  icon: string;
  title: string;
  badge?: string;
  children: React.ReactNode;
  delay?: number;
  accent: string;
}) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay, duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
      className="overflow-hidden relative"
      style={{
        border: "1px solid var(--border-default)",
        borderRadius: "12px",
        background: "var(--bg-surface)",
      }}
    >
      {/* Noise texture overlay */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.03'/%3E%3C/svg%3E")`,
          backgroundRepeat: "repeat",
          backgroundSize: "256px 256px",
          borderRadius: "12px",
          mixBlendMode: "overlay",
        }}
      />

      {/* Header */}
      <div className="flex justify-between items-center px-5 py-3 relative" style={{ borderBottom: "1px solid var(--border-default)" }}>
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 flex items-center justify-center rounded-md" style={{ background: accent + "14" }}>
            <Icon name={icon} size={15} style={{ color: accent }} />
          </div>
          <span className="font-mono text-[11px] font-semibold uppercase" style={{ color: "var(--text-primary)", letterSpacing: "0.06em" }}>{title}</span>
        </div>
        {badge && (
          <span className="font-mono text-[9px] px-2 py-0.5 rounded-full font-bold" style={{ color: accent, background: accent + "10", border: `1px solid ${accent}20` }}>{badge}</span>
        )}
      </div>

      <div className="relative">{children}</div>
    </motion.section>
  );
}

// ─── Main Customization Page ─────────────────────────────────────

export function Customization({ onBack: _onBack }: { onBack: () => void }) {
  const [packs, setPacks] = useState<CursorPackInfo[]>([]);
  const [selectedPack, setSelectedPack] = useState("default");
  const [trailStyle, setTrailStyle] = useState<TrailStyle>("glow");
  const [trailColor, setTrailColor] = useState("#00e88a");
  const [clickEffect, setClickEffect] = useState<ClickEffect>("ripple");
  const [clickColor, setClickColor] = useState("#00e88a");
  const [secondaryColor, setSecondaryColor] = useState("#ff4488");
  const [previewActive, setPreviewActive] = useState(false);

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
      if (cfg.cursor_secondary_color) setSecondaryColor(cfg.cursor_secondary_color);
    }).catch(() => {});
  }, []);

  const handleSelectPack = async (packId: string) => {
    setSelectedPack(packId);
    try { await invoke("update_config_field", { key: "cursor_pack", value: packId }); } catch {}
  };

  const handleTrailStyleChange = async (id: TrailStyle) => {
    setTrailStyle(id);
    try { await invoke("update_config_field", { key: "trail_style", value: id }); } catch {}
  };

  const handleClickEffectChange = async (id: ClickEffect) => {
    setClickEffect(id);
    try { await invoke("update_config_field", { key: "click_effect", value: id }); } catch {}
  };

  const handleTrailColorChange = async (color: string) => {
    setTrailColor(color);
    setClickColor(color);
    try { await invoke("update_config_field", { key: "cursor_trail_color", value: color }); } catch {}
  };

  const handleSecondaryColorChange = async (color: string) => {
    setSecondaryColor(color);
    try { await invoke("update_config_field", { key: "cursor_secondary_color", value: color }); } catch {}
  };

  const handlePreview = async () => {
    if (previewActive) {
      await invoke("restore_cursors").catch(() => {});
      setPreviewActive(false);
    } else {
      await invoke("apply_cursor_pack", { packId: selectedPack }).catch(() => {});
      setPreviewActive(true);
      setTimeout(async () => { await invoke("restore_cursors").catch(() => {}); setPreviewActive(false); }, 30000);
    }
  };

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="max-w-[680px] w-full mx-auto py-8 px-5 space-y-5">

        {/* ═══ HEADER ═══ */}
        <motion.div initial={{ opacity: 0, y: -8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}>
          <h2 className="text-[22px] font-semibold tracking-tight" style={{ color: "var(--text-primary)" }}>Customization</h2>
          <p className="text-[13px] mt-1" style={{ color: "var(--text-secondary)" }}>Shape how your recordings look and feel.</p>
        </motion.div>

        {/* ═══ CURSOR PACKS ═══ */}
        <Section icon="capture" title="Cursor pack" accent={PACK_COLORS[selectedPack] || "#8b949e"} delay={0.05}>
          <div className="flex justify-between items-center px-5 py-2.5" style={{ borderBottom: "1px solid var(--border-default)" }}>
            <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>
              {packs.find(p => p.id === selectedPack)?.name || selectedPack}
            </span>
            <motion.button
              onClick={handlePreview}
              className="font-mono text-[10px] px-3 py-1.5 cursor-pointer font-bold uppercase rounded-md"
              style={{
                border: `1px solid ${previewActive ? "#f04040" : PACK_COLORS[selectedPack] || "#00e88a"}`,
                color: previewActive ? "#f04040" : PACK_COLORS[selectedPack] || "#00e88a",
                background: previewActive ? "rgba(240,64,64,0.06)" : "transparent",
              }}
              whileHover={{ scale: 1.03 }}
              whileTap={{ scale: 0.97 }}
            >
              {previewActive ? "■ Stop" : "▶ Live 30s"}
            </motion.button>
          </div>

          <div className="p-4">
            <div className="grid grid-cols-2 gap-2.5">
              {packs.map((pack, i) => {
                const active = selectedPack === pack.id;
                const color = PACK_COLORS[pack.id] || "#888";
                return (
                  <motion.button
                    key={pack.id}
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.06 + i * 0.025, ease: [0.16, 1, 0.3, 1] }}
                    onClick={() => handleSelectPack(pack.id)}
                    className="text-left p-3.5 cursor-pointer relative group"
                    style={{
                      border: active ? `1.5px solid ${color}` : "1px solid var(--border-default)",
                      background: active ? `${color}08` : "var(--surface-container-low)",
                      borderRadius: "10px",
                      boxShadow: active ? `0 0 20px ${color}12, inset 0 0 20px ${color}06` : "none",
                      transition: "all 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
                    }}
                    whileHover={{ y: -2, boxShadow: `0 4px 16px ${color}15` }}
                    whileTap={{ scale: 0.98 }}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="w-3 h-3 rounded-full" style={{ background: color, boxShadow: active ? `0 0 10px ${color}` : "none" }} />
                      <AnimatePresence>
                        {active && <motion.span initial={{ scale: 0, rotate: -90 }} animate={{ scale: 1, rotate: 0 }} exit={{ scale: 0 }} transition={{ type: "spring", stiffness: 500, damping: 25 }}><Icon name="check_circle" size={14} style={{ color }} /></motion.span>}
                      </AnimatePresence>
                    </div>
                    <div className="font-mono text-[12px] font-semibold" style={{ color: active ? "var(--text-primary)" : "var(--text-secondary)" }}>{pack.name}</div>
                    <div className="font-mono text-[10px] mt-0.5 leading-tight" style={{ color: "var(--text-muted)" }}>{pack.description}</div>
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
        </Section>

        {/* ═══ TRAIL EFFECT ═══ */}
        <Section icon="auto_fix_high" title="Trail effect" accent={trailColor} delay={0.1}>
          {/* Color picker row */}
          <div className="flex items-center justify-between px-5 py-2.5" style={{ borderBottom: "1px solid var(--border-default)" }}>
            <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>
              {TRAIL_STYLES.find(s => s.id === trailStyle)?.desc}
            </span>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={trailColor}
                onChange={(e) => handleTrailColorChange(e.target.value)}
                className="w-5 h-5 cursor-pointer rounded"
                style={{ border: "1px solid var(--border-default)", background: "transparent" }}
              />
              <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>{trailColor.toUpperCase()}</span>
            </div>
          </div>

          {/* Style selector — segmented cards */}
          <div className="px-4 py-3">
            <div className="grid grid-cols-3 gap-1.5">
              {TRAIL_STYLES.map((s) => {
                const active = trailStyle === s.id;
                return (
                  <motion.button
                    key={s.id}
                    onClick={() => handleTrailStyleChange(s.id)}
                    className="flex flex-col items-center gap-0.5 py-2.5 cursor-pointer rounded-lg"
                    style={{
                      background: active ? trailColor + "12" : "transparent",
                      border: active ? `1.5px solid ${trailColor}40` : "1px solid var(--border-default)",
                      transition: "all 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
                    }}
                    whileHover={{ background: active ? trailColor + "18" : "var(--surface-container-low)" }}
                    whileTap={{ scale: 0.97 }}
                  >
                    <span className="font-mono text-[10px] font-bold" style={{ color: active ? trailColor : "var(--text-secondary)" }}>{s.label}</span>
                    {active && (
                      <motion.div
                        layoutId="trail-indicator"
                        className="w-1 h-1 rounded-full"
                        style={{ background: trailColor }}
                        transition={{ type: "spring", stiffness: 500, damping: 30 }}
                      />
                    )}
                  </motion.button>
                );
              })}
            </div>
          </div>

          {/* Live preview canvas */}
          <div className="px-4 pb-4">
            <div className="relative overflow-hidden" style={{ borderRadius: "8px", background: "var(--bg-elevated)", border: "1px dashed var(--border-default)" }}>
              <MiniPreview trailStyle={trailStyle} color={trailColor} />
              {trailStyle === "none" && (
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>Trail disabled</span>
                </div>
              )}
            </div>
          </div>
        </Section>

        {/* ═══ CLICK EFFECT ═══ */}
        <Section icon="radio_button_checked" title="Click effect" accent={clickColor} delay={0.15} badge="Phase 2">
          {/* Style selector */}
          <div className="px-4 py-3">
            <div className="grid grid-cols-3 gap-1.5">
              {CLICK_EFFECTS.map((s) => {
                const active = clickEffect === s.id;
                return (
                  <motion.button
                    key={s.id}
                    onClick={() => handleClickEffectChange(s.id)}
                    className="flex flex-col items-center gap-0.5 py-2.5 cursor-pointer rounded-lg"
                    style={{
                      background: active ? clickColor + "12" : "transparent",
                      border: active ? `1.5px solid ${clickColor}40` : "1px solid var(--border-default)",
                      transition: "all 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
                    }}
                    whileHover={{ background: active ? clickColor + "18" : "var(--surface-container-low)" }}
                    whileTap={{ scale: 0.97 }}
                  >
                    <span className="font-mono text-[10px] font-bold" style={{ color: active ? clickColor : "var(--text-secondary)" }}>{s.label}</span>
                    {active && (
                      <motion.div
                        layoutId="click-indicator"
                        className="w-1 h-1 rounded-full"
                        style={{ background: clickColor }}
                        transition={{ type: "spring", stiffness: 500, damping: 30 }}
                      />
                    )}
                  </motion.button>
                );
              })}
            </div>
          </div>

          {/* Description */}
          <div className="px-5 pb-1">
            <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>
              {CLICK_EFFECTS.find(s => s.id === clickEffect)?.desc}
            </span>
          </div>

          {/* Live preview canvas */}
          <div className="px-4 pb-4">
            <div className="relative overflow-hidden" style={{ borderRadius: "8px", background: "var(--bg-elevated)", border: "1px dashed var(--border-default)" }}>
              <MiniPreview clickEffect={clickEffect} color={clickColor} />
              {clickEffect === "none" && (
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>Click effect disabled</span>
                </div>
              )}
              {clickEffect !== "none" && (
                <div className="absolute bottom-2 right-3 pointer-events-none">
                  <span className="font-mono text-[9px] px-2 py-0.5 rounded" style={{ color: "var(--text-muted)", background: "rgba(0,0,0,0.25)" }}>Click to add</span>
                </div>
              )}
            </div>
          </div>
        </Section>

        {/* ═══ SECONDARY COLOR ═══ */}
        <Section icon="tune" title="Right-click color" accent={secondaryColor} delay={0.2}>
          <div className="flex items-center justify-between px-5 py-3">
            <div>
              <span className="font-mono text-[10px] font-semibold" style={{ color: "var(--text-primary)" }}>Secondary color</span>
              <span className="font-mono text-[9px] ml-2" style={{ color: "var(--text-muted)" }}>used for right-click effects + glow gradient</span>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={secondaryColor}
                onChange={(e) => handleSecondaryColorChange(e.target.value)}
                className="w-5 h-5 cursor-pointer rounded"
                style={{ border: "1px solid var(--border-default)", background: "transparent" }}
              />
              <span className="font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>{secondaryColor.toUpperCase()}</span>
            </div>
          </div>
        </Section>

        <div className="h-6" />
      </div>
    </div>
  );
}
