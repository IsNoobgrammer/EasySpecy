import { useEffect, useState, useRef, useCallback } from "react";
import { motion } from "motion/react";

// ═══ SYSTEM INFO BAR (replaces audio waveform) ═══

interface SystemInfo {
  screenWidth: number;
  screenHeight: number;
  refreshRate: number;
  cpuCores: number;
  cpuName: string;
  ramGb: number;
}

export function StatusBar({ isRecording }: { isRecording: boolean }) {
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [time, setTime] = useState(new Date());

  useEffect(() => {
    // Gather system info once
    const gather = async () => {
      try {
        const screenW = window.screen.width;
        const screenH = window.screen.height;
        const dpr = window.devicePixelRatio || 1;
        const physW = Math.round(screenW * dpr);
        const physH = Math.round(screenH * dpr);

        // Refresh rate — estimate from requestAnimationFrame timing
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
          // Clamp to common values
          if (refreshRate > 140) refreshRate = 144;
          else if (refreshRate > 110) refreshRate = 120;
          else if (refreshRate > 70) refreshRate = 75;
          else refreshRate = 60;
        } catch {}

        const cpuCores = navigator.hardwareConcurrency || 4;

        setInfo({
          screenWidth: physW,
          screenHeight: physH,
          refreshRate,
          cpuCores,
          cpuName: "",
          ramGb: 0,
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

  const formatTime = (d: Date) => {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
      className="flex flex-col w-full relative"
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

      {/* System info chips */}
      <div className="flex items-center justify-between px-4 py-2">
        <div className="flex items-center gap-3">
          {/* Resolution */}
          <Chip
            label={info ? `${info.screenWidth}×${info.screenHeight}` : "—"}
            sub="RES"
            accent="var(--accent-primary)"
          />

          {/* Refresh rate */}
          <Chip
            label={info ? `${info.refreshRate}Hz` : "—"}
            sub="REFRESH"
            accent="rgba(0, 200, 180, 0.9)"
          />

          {/* CPU cores */}
          <Chip
            label={info ? `${info.cpuCores}C` : "—"}
            sub="CPU"
            accent="rgba(180, 140, 255, 0.9)"
          />

          {/* Device pixel ratio */}
          <Chip
            label={`${window.devicePixelRatio || 1}x`}
            sub="DPR"
            accent="rgba(255, 200, 0, 0.9)"
          />
        </div>

        {/* Clock */}
        <span
          className="font-mono text-[10px] tabular-nums"
          style={{ color: "var(--text-muted)" }}
        >
          {formatTime(time)}
        </span>
      </div>

      {/* Footer credit */}
      <div
        className="flex items-center justify-center px-4 py-1.5"
        style={{ borderTop: "1px solid var(--border-default)" }}
      >
        <span
          className="font-mono"
          style={{ color: "var(--text-muted)", fontSize: "8px", letterSpacing: "0.04em" }}
        >
          Made with <span style={{ color: "var(--accent-danger)" }}>♥</span> by{" "}
          <span style={{ color: "var(--text-secondary)" }}>Shaurya</span> &{" "}
          <span style={{ color: "var(--text-secondary)" }}>Bonna</span> · © 2026{" "}
          <span style={{ color: "var(--accent-primary)" }}>EasySpecy</span>
        </span>
      </div>
    </motion.div>
  );
}

// ═══ Chip component ═══

function Chip({ label, sub, accent }: { label: string; sub: string; accent: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span
        className="font-mono text-[10px] font-bold tabular-nums"
        style={{ color: accent }}
      >
        {label}
      </span>
      <span
        className="font-mono uppercase"
        style={{ color: "var(--text-muted)", fontSize: "7px", fontWeight: 700, letterSpacing: "0.08em" }}
      >
        {sub}
      </span>
    </div>
  );
}

// ═══ SIDEBAR LOUDNESS METER ═══
// Vertical VU meter for the sidebar — shows dB level intuitively

interface AudioLevels {
  micRms: number;
  micPeak: number;
  micDb: number;
  sysRms: number;
  sysPeak: number;
  sysDb: number;
}

function lerp(current: number, target: number, speed: number): number {
  return current + (target - current) * speed;
}

interface LoudnessMeterProps {
  audioSource: "Mic" | "System" | "Both";
  audioEnabled: boolean;
  levels: AudioLevels;
}

export function LoudnessMeter({ audioSource, audioEnabled, levels }: LoudnessMeterProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<{
    level: number;
    peak: number;
    peakHoldTimer: number;
  }>({
    level: 0,
    peak: 0,
    peakHoldTimer: 0,
  });

  const getActiveLevel = useCallback((): { rms: number; peak: number; db: number } => {
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
  }, [audioSource, audioEnabled, levels]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let running = true;
    const state = animRef.current;

    const draw = () => {
      if (!running) return;

      const dpr = window.devicePixelRatio || 1;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;

      if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
        canvas.width = w * dpr;
        canvas.height = h * dpr;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }

      ctx.clearRect(0, 0, w, h);

      const activeLevel = getActiveLevel();

      // Convert dB to normalized 0-1 range
      // -60dB = 0, 0dB = 1
      const dbNorm = Math.max(0, Math.min(1, (activeLevel.db + 60) / 60));

      // Smooth animation — fast attack, slow decay
      const targetLevel = dbNorm;
      const speed = targetLevel > state.level ? 0.4 : 0.08;
      state.level = lerp(state.level, targetLevel, speed);

      // Peak hold
      if (state.level > state.peak) {
        state.peak = state.level;
        state.peakHoldTimer = 0;
      } else {
        state.peakHoldTimer++;
        if (state.peakHoldTimer > 30) {
          state.peak = Math.max(0, state.peak - 0.01);
        }
      }

      const barX = 4;
      const barWidth = w - 8;
      const barTop = 4;
      const barBottom = h - 4;
      const barHeight = barBottom - barTop;

      // Background track
      ctx.fillStyle = "rgba(59, 74, 63, 0.25)";
      ctx.beginPath();
      ctx.roundRect(barX, barTop, barWidth, barHeight, 3);
      ctx.fill();

      // Segmented meter
      const numSegments = 20;
      const segmentGap = 1.5;
      const segmentHeight = (barHeight - (numSegments - 1) * segmentGap) / numSegments;
      const filledSegments = Math.round(state.level * numSegments);

      for (let i = 0; i < numSegments; i++) {
        const segY = barBottom - (i + 1) * (segmentHeight + segmentGap) + segmentGap;
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

          if (segNorm > 0.7) {
            ctx.shadowColor = color;
            ctx.shadowBlur = 3;
          } else {
            ctx.shadowColor = "transparent";
            ctx.shadowBlur = 0;
          }
        } else {
          ctx.fillStyle = "rgba(59, 74, 63, 0.15)";
          ctx.shadowColor = "transparent";
          ctx.shadowBlur = 0;
        }

        ctx.beginPath();
        ctx.roundRect(barX, segY, barWidth, segmentHeight, 1.5);
        ctx.fill();
      }

      // Peak indicator line
      ctx.shadowColor = "transparent";
      ctx.shadowBlur = 0;
      const peakY = barBottom - state.peak * barHeight;
      ctx.fillStyle = "rgba(255, 255, 255, 0.7)";
      ctx.fillRect(barX - 1, peakY - 1, barWidth + 2, 2);

      requestAnimationFrame(draw);
    };

    requestAnimationFrame(draw);
    return () => { running = false; };
  }, [getActiveLevel]);

  const activeLevel = getActiveLevel();
  const dbDisplay = activeLevel.db > -59 ? `${activeLevel.db.toFixed(0)}` : "--";

  const dbColor = activeLevel.db > -12
    ? "var(--accent-danger)"
    : activeLevel.db > -24
    ? "rgba(255, 200, 0, 0.9)"
    : "var(--accent-primary)";

  return (
    <div className="flex flex-col items-center gap-1.5 px-2 py-2">
      <span
        className="font-mono uppercase"
        style={{
          color: "var(--text-muted)",
          fontSize: "8px",
          letterSpacing: "0.1em",
          fontWeight: 700,
        }}
      >
        {audioSource === "System" ? "SYS" : audioSource === "Both" ? "MIX" : "MIC"}
      </span>

      <canvas
        ref={canvasRef}
        style={{ width: 20, height: 100, display: "block" }}
      />

      <div className="flex flex-col items-center">
        <span
          className="font-mono text-[11px] font-bold tabular-nums"
          style={{ color: dbColor }}
        >
          {dbDisplay}
        </span>
        <span
          className="font-mono"
          style={{ color: "var(--text-muted)", fontSize: "7px", fontWeight: 600 }}
        >
          dB
        </span>
      </div>
    </div>
  );
}
