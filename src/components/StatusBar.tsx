import { useEffect, useState, useRef, useCallback } from "react";
import { motion } from "motion/react";
import { useStore } from "../stores/recording";

// ═══ FOOTER (credit line only) ═══

export function Footer({ isRecording }: { isRecording: boolean }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
      className="flex items-center justify-center w-full relative px-4 py-1.5"
      style={{
        background: "var(--bg-surface)",
        borderTop: "1px solid var(--border-default)",
      }}
    >
      {/* Recording indicator glow */}
      {isRecording && (
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            height: 1,
            background:
              "linear-gradient(90deg, transparent 0%, rgba(240, 64, 64, 0.6) 20%, rgba(240, 64, 64, 0.8) 50%, rgba(240, 64, 64, 0.6) 80%, transparent 100%)",
          }}
        />
      )}

      <span
        className="font-mono"
        style={{ color: "var(--text-muted)", fontSize: "8px", letterSpacing: "0.04em" }}
      >
        Made with <span style={{ color: "var(--accent-danger)" }}>♥</span> by{" "}
        <span style={{ color: "var(--text-secondary)" }}>Shaurya</span> &{" "}
        <span style={{ color: "var(--text-secondary)" }}>Bauna Intern</span> · © 2026{" "}
        <span style={{ color: "var(--accent-primary)" }}>EasySpecy</span>
      </span>
    </motion.div>
  );
}

// ═══ SIDEBAR STATS PANEL ═══
// Combines system info + loudness meter in the sidebar bottom section


function lerp(current: number, target: number, speed: number): number {
  return current + (target - current) * speed;
}

interface SidebarStatsProps {
  audioSource: "Mic" | "System" | "Both";
  audioEnabled: boolean;
}

export function SidebarStats({ audioSource, audioEnabled }: SidebarStatsProps) {
  const [systemInfo, setSystemInfo] = useState<{
    screenWidth: number;
    screenHeight: number;
    refreshRate: number;
    cpuCores: number;
    dpr: number;
  } | null>(null);
  const [time, setTime] = useState(new Date());

  // Gather system info once
  useEffect(() => {
    const gather = async () => {
      try {
        const dpr = window.devicePixelRatio || 1;
        const physW = Math.round(window.screen.width * dpr);
        const physH = Math.round(window.screen.height * dpr);

        // Refresh rate estimate
        let refreshRate = 60;
        try {
          const samples: number[] = [];
          for (let i = 0; i < 10; i++) {
            const start = performance.now();
            await new Promise<void>(r => requestAnimationFrame(() => r()));
            samples.push(performance.now() - start);
          }
          const avgMs = samples.reduce((a, b) => a + b, 0) / samples.length;
          refreshRate = Math.round(1000 / avgMs);
          if (refreshRate > 140) refreshRate = 144;
          else if (refreshRate > 110) refreshRate = 120;
          else if (refreshRate > 70) refreshRate = 75;
          else refreshRate = 60;
        } catch {}

        setSystemInfo({
          screenWidth: physW,
          screenHeight: physH,
          refreshRate,
          cpuCores: navigator.hardwareConcurrency || 4,
          dpr,
        });
      } catch {}
    };
    gather();
  }, []);

  // Clock
  useEffect(() => {
    const id = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const formatTime = (d: Date) => d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  return (
    <div className="flex flex-col gap-3 overflow-hidden">
      {/* ═══ SYSTEM STATS ═══ */}
      <div className="grid grid-cols-2 gap-x-3 gap-y-2 px-2">
        <StatItem
          label="RES"
          value={systemInfo ? `${systemInfo.screenWidth}×${systemInfo.screenHeight}` : "—"}
          color="var(--accent-primary)"
        />
        <StatItem
          label="REFRESH"
          value={systemInfo ? `${systemInfo.refreshRate}Hz` : "—"}
          color="rgba(0, 200, 180, 0.9)"
        />
        <StatItem
          label="CPU"
          value={systemInfo ? `${systemInfo.cpuCores}C` : "—"}
          color="rgba(180, 140, 255, 0.9)"
        />
        <StatItem
          label="DPR"
          value={`${window.devicePixelRatio || 1}x`}
          color="rgba(255, 200, 0, 0.9)"
        />
      </div>

      {/* ═══ LOUDNESS METER (horizontal layout) ═══ */}
      {audioEnabled && (
        <div className="px-2 overflow-hidden">
          <LoudnessMeterInline
            audioSource={audioSource}
            audioEnabled={audioEnabled}
          />
        </div>
      )}

      {/* ═══ CLOCK ═══ */}
      <div className="flex items-center justify-center">
        <span
          className="font-mono text-[10px] tabular-nums"
          style={{ color: "var(--text-muted)" }}
        >
          {formatTime(time)}
        </span>
      </div>
    </div>
  );
}

// ═══ Stat Item ═══
function StatItem({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="flex flex-col items-start gap-0.5">
      <span
        className="font-mono uppercase"
        style={{ color: "var(--text-muted)", fontSize: "7px", letterSpacing: "0.1em", fontWeight: 700 }}
      >
        {label}
      </span>
      <span
        className="font-mono text-[10px] font-bold tabular-nums"
        style={{ color }}
      >
        {value}
      </span>
    </div>
  );
}

// ═══ INLINE LOUDNESS METER (horizontal, fits sidebar width) ═══
function LoudnessMeterInline({ audioSource, audioEnabled }: SidebarStatsProps) {
  // Only this meter subscribes to levels (it used to re-render the whole app at 20 Hz).
  const levels = useStore((s) => s.audioLevels);
  const levelsRef = useRef(levels);
  levelsRef.current = levels;
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const kickRef = useRef<() => void>(() => {});
  const animRef = useRef<{
    level: number;
    peak: number;
    peakHoldTimer: number;
  }>({
    level: 0,
    peak: 0,
    peakHoldTimer: 0,
  });

  // Reads the ref, not props: the draw loop below stays mounted instead of being torn
  // down and rebuilt on every poll.
  const getActiveLevel = useCallback((): { rms: number; peak: number; db: number } => {
    const levels = levelsRef.current;
    if (!audioEnabled) return { rms: 0, peak: 0, db: -60 };
    switch (audioSource) {
      case "Mic":
        return { rms: levels.micRms, peak: levels.micPeak, db: levels.micDb };
      case "System":
        return { rms: levels.sysRms, peak: levels.sysPeak, db: levels.sysDb };
      case "Both":
        return {
          rms: Math.max(levels.micRms, levels.sysRms),
          peak: Math.max(levels.micPeak, levels.sysPeak),
          db: Math.max(levels.micDb, levels.sysDb),
        };
    }
  }, [audioSource, audioEnabled]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let running = true;
    let active = false; // is a rAF chain live?
    const state = animRef.current;
    let drawn = { level: -1, peak: -1, w: 0 };
    // Animate only while the bar is moving; a pending rAF alone keeps Chromium's renderer
    // + GPU process waking at 60 Hz, so a settled meter stops and levels changes restart it.
    const kick = () => { if (running && !active) { active = true; requestAnimationFrame(draw); } };
    kickRef.current = kick;

    const draw = () => {
      if (!running) { active = false; return; }

      const dpr = window.devicePixelRatio || 1;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;

      if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
        canvas.width = w * dpr;
        canvas.height = h * dpr;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }

      const activeLevel = getActiveLevel();
      const dbNorm = Math.max(0, Math.min(1, (activeLevel.db + 60) / 60));

      const targetLevel = dbNorm;
      const speed = targetLevel > state.level ? 0.4 : 0.08;
      state.level = lerp(state.level, targetLevel, speed);

      if (state.level > state.peak) {
        state.peak = state.level;
        state.peakHoldTimer = 0;
      } else {
        state.peakHoldTimer++;
        if (state.peakHoldTimer > 30) {
          state.peak = Math.max(0, state.peak - 0.01);
        }
      }

      // Nothing moved since the last paint → skip it; once fully settled, stop the loop
      if (Math.abs(state.level - drawn.level) < 0.002 && Math.abs(state.peak - drawn.peak) < 0.002 && drawn.w === w) {
        if (Math.abs(state.level - targetLevel) < 0.002 && state.peak - state.level < 0.002) { active = false; return; }
        requestAnimationFrame(draw);
        return;
      }
      drawn = { level: state.level, peak: state.peak, w };
      ctx.clearRect(0, 0, w, h);

      // Horizontal segmented bar
      const barY = 2;
      const barHeight = h - 4;
      const barLeft = 0;
      const barRight = w;
      const barWidth = barRight - barLeft;

      // Background track
      ctx.fillStyle = "rgba(59, 74, 63, 0.25)";
      ctx.beginPath();
      ctx.roundRect(barLeft, barY, barWidth, barHeight, 3);
      ctx.fill();

      // Segments
      const numSegments = 24;
      const segmentGap = 1.5;
      const segmentWidth = (barWidth - (numSegments - 1) * segmentGap) / numSegments;
      const filledSegments = Math.round(state.level * numSegments);

      for (let i = 0; i < numSegments; i++) {
        const segX = barLeft + i * (segmentWidth + segmentGap);
        const isFilled = i < filledSegments;

        if (isFilled) {
          const segNorm = i / numSegments;
          let color: string;
          if (segNorm < 0.6) {
            color = "rgba(0, 232, 138, 0.85)";
          } else if (segNorm < 0.8) {
            color = "rgba(255, 200, 0, 0.85)";
          } else {
            color = "rgba(240, 64, 64, 0.9)";
          }
          ctx.fillStyle = color;
        } else {
          ctx.fillStyle = "rgba(59, 74, 63, 0.15)";
        }

        ctx.beginPath();
        ctx.roundRect(segX, barY, segmentWidth, barHeight, 1.5);
        ctx.fill();
      }

      // Peak indicator
      const peakX = barLeft + state.peak * barWidth;
      ctx.fillStyle = "rgba(255, 255, 255, 0.6)";
      ctx.fillRect(peakX - 1, barY - 1, 2, barHeight + 2);

      requestAnimationFrame(draw);
    };

    kick();
    return () => { running = false; };
  }, [getActiveLevel]);

  useEffect(() => { kickRef.current(); }, [levels]);

  const activeLevel = getActiveLevel();
  const dbDisplay = activeLevel.db > -59 ? `${activeLevel.db.toFixed(0)}` : "--";
  const dbColor = activeLevel.db > -12
    ? "var(--accent-danger)"
    : activeLevel.db > -24
    ? "rgba(255, 200, 0, 0.9)"
    : "var(--accent-primary)";

  return (
    <div className="flex items-center gap-2 w-full overflow-hidden">
      {/* Source label */}
      <span
        className="font-mono uppercase shrink-0"
        style={{ color: "var(--text-muted)", fontSize: "7px", letterSpacing: "0.1em", fontWeight: 700 }}
      >
        {audioSource === "System" ? "SYS" : audioSource === "Both" ? "MIX" : "MIC"}
      </span>

      {/* Horizontal meter */}
      <canvas
        ref={canvasRef}
        className="flex-1 min-w-0"
        style={{ height: 10, display: "block", background: "transparent" }}
      />

      {/* dB readout */}
      <span
        className="font-mono text-[9px] font-bold tabular-nums shrink-0"
        style={{ color: dbColor, minWidth: 24, textAlign: "right" }}
      >
        {dbDisplay}<span style={{ fontSize: "7px", color: "var(--text-muted)" }}>dB</span>
      </span>
    </div>
  );
}

// Keep the old export name for backward compat (unused now but safe)
export const LoudnessMeter = SidebarStats;
