import { useEffect, useRef, useCallback } from "react";
import { motion } from "motion/react";

// ═══ PRO AUDIO VISUALIZATION STATUS BAR ═══
// Canvas-based waveform + VU meter + system stats
// Replaces the old fake AudioMeter component

interface AudioLevels {
  micRms: number;
  micPeak: number;
  micDb: number;
  sysRms: number;
  sysPeak: number;
  sysDb: number;
}

interface StatusBarProps {
  audioSource: "Mic" | "System" | "Both";
  audioEnabled: boolean;
  levels: AudioLevels;
  isRecording: boolean;
  fps: number;
  resolution: string;
  sampleRate: number;
}

// Smooth value interpolation for animation
function lerp(current: number, target: number, speed: number): number {
  return current + (target - current) * speed;
}

export function StatusBar({
  audioSource,
  audioEnabled,
  levels,
  isRecording,
  fps,
  resolution,
  sampleRate,
}: StatusBarProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<{
    bars: number[];
    peakHold: number[];
    peakDecay: number[];
    waveformPhase: number;
    vuLevel: number;
    vuPeak: number;
    vuPeakHold: number;
  }>({
    bars: new Array(32).fill(0),
    peakHold: new Array(32).fill(0),
    peakDecay: new Array(32).fill(0),
    waveformPhase: 0,
    vuLevel: 0,
    vuPeak: 0,
    vuPeakHold: 0,
  });

  // Get the relevant level based on audio source
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
        ctx.scale(dpr, dpr);
      }

      ctx.clearRect(0, 0, w, h);

      const activeLevel = getActiveLevel();
      const numBars = 32;
      const barWidth = 4;
      const barGap = 2;
      const totalBarWidth = numBars * (barWidth + barGap) - barGap;
      const startX = (w - totalBarWidth) / 2;
      const maxBarHeight = h - 20;
      const centerY = h - 10;

      // Update frequency bars
      state.waveformPhase += 0.02;
      for (let i = 0; i < numBars; i++) {
        // Create frequency-like distribution from the audio level
        const freqFactor = 1.0 - Math.abs(i - numBars / 2) / (numBars / 2);
        const noise = Math.sin(state.waveformPhase * 3 + i * 0.7) * 0.15 +
          Math.sin(state.waveformPhase * 5 + i * 1.3) * 0.1;
        const target = activeLevel.rms * (0.5 + freqFactor * 0.5 + noise);
        state.bars[i] = lerp(state.bars[i], target, 0.3);

        // Peak hold
        if (state.bars[i] > state.peakHold[i]) {
          state.peakHold[i] = state.bars[i];
          state.peakDecay[i] = 0;
        } else {
          state.peakDecay[i] += 0.005;
          state.peakHold[i] = Math.max(0, state.peakHold[i] - state.peakDecay[i]);
        }
      }

      // Draw frequency bars with gradient
      for (let i = 0; i < numBars; i++) {
        const barHeight = Math.max(2, state.bars[i] * maxBarHeight);
        const x = startX + i * (barWidth + barGap);
        const y = centerY - barHeight;

        // Gradient: emerald at bottom → cyan at top
        const gradient = ctx.createLinearGradient(x, centerY, x, y);
        gradient.addColorStop(0, "rgba(0, 232, 138, 0.9)");
        gradient.addColorStop(0.4, "rgba(0, 200, 180, 0.85)");
        gradient.addColorStop(0.7, "rgba(0, 180, 220, 0.8)");
        gradient.addColorStop(1, "rgba(100, 200, 255, 0.7)");

        // Glow on taller bars
        if (barHeight > maxBarHeight * 0.5) {
          ctx.shadowColor = "rgba(0, 232, 138, 0.4)";
          ctx.shadowBlur = 6;
        } else {
          ctx.shadowColor = "transparent";
          ctx.shadowBlur = 0;
        }

        ctx.fillStyle = gradient;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, [2, 2, 0, 0]);
        ctx.fill();

        // Peak indicator
        const peakY = centerY - Math.max(2, state.peakHold[i] * maxBarHeight);
        ctx.shadowColor = "transparent";
        ctx.shadowBlur = 0;
        ctx.fillStyle = "rgba(133, 255, 180, 0.8)";
        ctx.fillRect(x, peakY - 2, barWidth, 2);
      }

      // Draw smooth waveform line overlay
      ctx.beginPath();
      ctx.strokeStyle = "rgba(133, 255, 180, 0.25)";
      ctx.lineWidth = 1.5;
      for (let i = 0; i < numBars; i++) {
        const barHeight = Math.max(2, state.bars[i] * maxBarHeight);
        const x = startX + i * (barWidth + barGap) + barWidth / 2;
        const y = centerY - barHeight;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();

      // ═══ VU METER (right side) ═══
      const vuX = w - 60;
      const vuWidth = 8;
      const vuHeight = h - 24;
      const vuY = 8;

      // Background track
      ctx.fillStyle = "rgba(59, 74, 63, 0.3)";
      ctx.beginPath();
      ctx.roundRect(vuX, vuY, vuWidth, vuHeight, 4);
      ctx.fill();

      // VU level
      state.vuLevel = lerp(state.vuLevel, activeLevel.rms, 0.25);
      state.vuPeak = lerp(state.vuPeak, activeLevel.peak, 0.35);
      if (state.vuPeak > state.vuPeakHold) {
        state.vuPeakHold = state.vuPeak;
      } else {
        state.vuPeakHold = Math.max(0, state.vuPeakHold - 0.008);
      }

      const fillHeight = Math.min(vuHeight, state.vuLevel * vuHeight * 1.5);

      // Gradient fill: green → yellow → red
      const vuGradient = ctx.createLinearGradient(vuX, vuY + vuHeight, vuX, vuY);
      vuGradient.addColorStop(0, "rgba(0, 232, 138, 0.8)");
      vuGradient.addColorStop(0.6, "rgba(200, 232, 0, 0.8)");
      vuGradient.addColorStop(0.8, "rgba(255, 180, 0, 0.8)");
      vuGradient.addColorStop(1, "rgba(240, 64, 64, 0.9)");

      ctx.fillStyle = vuGradient;
      ctx.beginPath();
      ctx.roundRect(vuX, vuY + vuHeight - fillHeight, vuWidth, fillHeight, 4);
      ctx.fill();

      // Peak indicator on VU
      const peakVuY = vuY + vuHeight - Math.min(vuHeight, state.vuPeakHold * vuHeight * 1.5);
      ctx.fillStyle = "rgba(255, 255, 255, 0.6)";
      ctx.fillRect(vuX - 1, peakVuY, vuWidth + 2, 2);

      // dB scale markers
      ctx.font = "9px 'JetBrains Mono', monospace";
      ctx.fillStyle = "rgba(132, 149, 135, 0.5)";
      ctx.textAlign = "right";
      const dbMarks = [0, -12, -24, -36, -48, -60];
      for (const db of dbMarks) {
        const norm = (db + 60) / 60;
        const markY = vuY + vuHeight - norm * vuHeight;
        ctx.fillText(`${db}`, vuX - 4, markY + 3);
      }

      requestAnimationFrame(draw);
    };

    const frameId = requestAnimationFrame(draw);
    return () => {
      running = false;
      cancelAnimationFrame(frameId);
    };
  }, [getActiveLevel]);

  const activeLevel = getActiveLevel();
  const dbDisplay = activeLevel.db > -59 ? `${activeLevel.db.toFixed(0)}dB` : "--dB";

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
      className="flex items-center gap-4 px-4 w-full"
      style={{
        height: 72,
        background: "var(--bg-surface)",
        borderTop: "1px solid rgba(0, 232, 138, 0.15)",
        borderBottom: "1px solid var(--border-default)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Subtle noise texture overlay */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage: "var(--noise)",
          backgroundRepeat: "repeat",
          backgroundSize: "256px 256px",
          opacity: 0.3,
          pointerEvents: "none",
        }}
      />

      {/* ═══ CHANNEL SELECTOR ═══ */}
      <div className="flex flex-col gap-1 shrink-0 z-10">
        <span
          className="font-mono uppercase"
          style={{
            color: "var(--text-muted)",
            fontSize: "9px",
            letterSpacing: "0.08em",
            fontWeight: 700,
          }}
        >
          CHANNEL
        </span>
        <div className="flex gap-1">
          {(["Mic", "System", "Both"] as const).map((ch) => (
            <div
              key={ch}
              className="px-2 py-1 font-mono text-[10px] font-bold"
              style={{
                borderRadius: "var(--radius-xs)",
                background:
                  audioSource === ch
                    ? "rgba(0, 232, 138, 0.15)"
                    : "var(--bg-elevated)",
                color:
                  audioSource === ch
                    ? "var(--accent-primary)"
                    : "var(--text-muted)",
                border: `1px solid ${
                  audioSource === ch
                    ? "rgba(0, 232, 138, 0.3)"
                    : "var(--border-default)"
                }`,
              }}
            >
              {ch === "System" ? "SYS" : ch.toUpperCase()}
            </div>
          ))}
        </div>
      </div>

      {/* ═══ WAVEFORM VISUALIZATION (center) ═══ */}
      <div className="flex-1 relative z-10 h-full py-2">
        <canvas
          ref={canvasRef}
          style={{ width: "100%", height: "100%", display: "block" }}
        />
      </div>

      {/* ═══ TECHNICAL READOUTS (right) ═══ */}
      <div className="flex items-center gap-5 shrink-0 z-10">
        {/* Audio Level */}
        <div className="flex flex-col items-end gap-0.5">
          <span
            className="font-mono uppercase"
            style={{
              color: "var(--text-muted)",
              fontSize: "9px",
              letterSpacing: "0.08em",
              fontWeight: 700,
            }}
          >
            LEVEL
          </span>
          <span
            className="font-mono text-sm font-bold"
            style={{
              color:
                activeLevel.db > -12
                  ? "var(--accent-danger)"
                  : activeLevel.db > -24
                  ? "#ffe0dd"
                  : "var(--accent-primary)",
            }}
          >
            {dbDisplay}
          </span>
        </div>

        {/* Divider */}
        <div
          className="h-8 w-px"
          style={{ background: "var(--border-default)" }}
        />

        {/* FPS */}
        <div className="flex flex-col items-end gap-0.5">
          <span
            className="font-mono uppercase"
            style={{
              color: "var(--text-muted)",
              fontSize: "9px",
              letterSpacing: "0.08em",
              fontWeight: 700,
            }}
          >
            FPS
          </span>
          <span
            className="font-mono text-sm font-bold"
            style={{ color: "var(--text-primary)" }}
          >
            {fps}
          </span>
        </div>

        {/* Resolution */}
        <div className="flex flex-col items-end gap-0.5">
          <span
            className="font-mono uppercase"
            style={{
              color: "var(--text-muted)",
              fontSize: "9px",
              letterSpacing: "0.08em",
              fontWeight: 700,
            }}
          >
            OUTPUT
          </span>
          <span
            className="font-mono text-[11px] font-bold"
            style={{ color: "var(--text-secondary)" }}
          >
            {resolution}
          </span>
        </div>

        {/* Sample Rate */}
        <div className="flex flex-col items-end gap-0.5">
          <span
            className="font-mono uppercase"
            style={{
              color: "var(--text-muted)",
              fontSize: "9px",
              letterSpacing: "0.08em",
              fontWeight: 700,
            }}
          >
            SAMPLE
          </span>
          <span
            className="font-mono text-[11px] font-bold"
            style={{ color: "var(--text-secondary)" }}
          >
            {sampleRate / 1000}kHz
          </span>
        </div>
      </div>

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
    </motion.div>
  );
}
