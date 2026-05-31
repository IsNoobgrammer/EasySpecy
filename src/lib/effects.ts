/**
 * EasySpecy — Canvas Effects Engine
 * Shared between Customization preview and RecordingOverlay.
 * Zero dependencies. Pure canvas 2D.
 */

// ─── Types ───────────────────────────────────────────────────────

export interface Point {
  x: number;
  y: number;
  age: number;
  vx?: number;
  vy?: number;
  size?: number;
  alpha?: number;
  hue?: number;
}

export interface ClickEvent {
  x: number;
  y: number;
  age: number;
  color: string;
}

export type TrailStyle = "glow" | "particles" | "ribbon" | "dots" | "aurora" | "none";
export type ClickEffect = "ripple" | "spotlight" | "ring" | "pulse" | "confetti" | "none";

// ─── Color Helpers ───────────────────────────────────────────────

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.substring(0, 2), 16),
    parseInt(h.substring(2, 4), 16),
    parseInt(h.substring(4, 6), 16),
  ];
}

function rgba(hex: string, alpha: number): string {
  const [r, g, b] = hexToRgb(hex);
  return `rgba(${r},${g},${b},${alpha})`;
}

function hslToStr(h: number, s: number, l: number, a: number): string {
  return `hsla(${h},${s}%,${l}%,${a})`;
}

// ─── Trail Renderer ─────────────────────────────────────────────

export class TrailRenderer {
  private points: Point[] = [];
  private maxPoints = 60;
  private style: TrailStyle = "glow";
  private color = "#00e88a";
  private time = 0;

  setStyle(s: TrailStyle) { this.style = s; }
  setColor(c: string) { this.color = c; }
  clear() { this.points = []; }

  addPoint(x: number, y: number) {
    const last = this.points[this.points.length - 1];
    // Skip if too close
    if (last && Math.hypot(x - last.x, y - last.y) < 2) return;

    this.points.push({
      x, y, age: 0,
      vx: (Math.random() - 0.5) * 2,
      vy: (Math.random() - 0.5) * 2,
      size: 1,
      alpha: 1,
      hue: Math.random() * 60 - 30,
    });
    if (this.points.length > this.maxPoints) this.points.shift();
  }

  update() {
    this.time++;
    this.points = this.points.filter(p => {
      p.age++;
      return p.age < 45;
    });
  }

  draw(ctx: CanvasRenderingContext2D, w: number, h: number) {
    ctx.clearRect(0, 0, w, h);
    if (this.style === "none" || this.points.length === 0) return;

    switch (this.style) {
      case "glow": this.drawGlow(ctx); break;
      case "particles": this.drawParticles(ctx); break;
      case "ribbon": this.drawRibbon(ctx); break;
      case "dots": this.drawDots(ctx); break;
      case "aurora": this.drawAurora(ctx); break;
    }
  }

  private drawGlow(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    if (pts.length < 2) return;

    // Outer glow layer (wide, very transparent)
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    for (let pass = 0; pass < 3; pass++) {
      const widths = [20, 10, 4];
      const alphas = [0.06, 0.12, 0.8];
      ctx.beginPath();
      ctx.moveTo(pts[0].x, pts[0].y);
      for (let i = 1; i < pts.length; i++) {
        const p0 = pts[i - 1];
        const p1 = pts[i];
        const mx = (p0.x + p1.x) / 2;
        const my = (p0.y + p1.y) / 2;
        ctx.quadraticCurveTo(p0.x, p0.y, mx, my);
      }
      const last = pts[pts.length - 1];
      ctx.lineTo(last.x, last.y);
      ctx.strokeStyle = rgba(this.color, alphas[pass]);
      ctx.lineWidth = widths[pass];
      ctx.stroke();
    }

    // Head glow
    const head = pts[pts.length - 1];
    const headAlpha = Math.max(0, 1 - head.age / 20);
    const grad = ctx.createRadialGradient(head.x, head.y, 0, head.x, head.y, 18);
    grad.addColorStop(0, rgba(this.color, 0.6 * headAlpha));
    grad.addColorStop(0.4, rgba(this.color, 0.2 * headAlpha));
    grad.addColorStop(1, rgba(this.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(head.x, head.y, 18, 0, Math.PI * 2);
    ctx.fill();

    ctx.restore();
  }

  private drawParticles(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    ctx.save();

    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const lifeRatio = 1 - p.age / 45;
      if (lifeRatio <= 0) continue;

      // Main dot
      const size = 2 + lifeRatio * 3;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size, 0, Math.PI * 2);
      ctx.fillStyle = rgba(this.color, lifeRatio * 0.9);
      ctx.fill();

      // Spark particles
      const sparkCount = Math.floor(lifeRatio * 4);
      for (let s = 0; s < sparkCount; s++) {
        const angle = (this.time * 0.1 + i * 0.7 + s * 1.8) % (Math.PI * 2);
        const dist = 4 + Math.random() * 8 * lifeRatio;
        const sx = p.x + Math.cos(angle) * dist;
        const sy = p.y + Math.sin(angle) * dist;
        const sparkSize = 0.5 + Math.random() * 1.5;
        ctx.beginPath();
        ctx.arc(sx, sy, sparkSize, 0, Math.PI * 2);
        ctx.fillStyle = rgba(this.color, lifeRatio * 0.4 * Math.random());
        ctx.fill();
      }
    }

    // Ambient floating particles near head
    if (pts.length > 0) {
      const head = pts[pts.length - 1];
      for (let i = 0; i < 8; i++) {
        const angle = (this.time * 0.03 + i * 0.785) % (Math.PI * 2);
        const dist = 10 + Math.sin(this.time * 0.05 + i) * 8;
        const fx = head.x + Math.cos(angle) * dist;
        const fy = head.y + Math.sin(angle) * dist;
        const fSize = 1 + Math.sin(this.time * 0.08 + i * 2) * 0.5;
        ctx.beginPath();
        ctx.arc(fx, fy, fSize, 0, Math.PI * 2);
        ctx.fillStyle = rgba(this.color, 0.3 + Math.sin(this.time * 0.1 + i) * 0.2);
        ctx.fill();
      }
    }

    ctx.restore();
  }

  private drawRibbon(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    if (pts.length < 3) return;
    ctx.save();

    // Smooth ribbon using Catmull-Rom → Bezier
    for (let pass = 0; pass < 2; pass++) {
      const baseWidth = pass === 0 ? 12 : 5;
      const alpha = pass === 0 ? 0.08 : 0.3;

      ctx.beginPath();
      ctx.moveTo(pts[0].x, pts[0].y);

      for (let i = 1; i < pts.length - 1; i++) {
        const p0 = pts[Math.max(0, i - 1)];
        const p1 = pts[i];
        const p2 = pts[Math.min(pts.length - 1, i + 1)];
        const p3 = pts[Math.min(pts.length - 1, i + 2)];

        const cp1x = p1.x + (p2.x - p0.x) / 6;
        const cp1y = p1.y + (p2.y - p0.y) / 6;
        const cp2x = p2.x - (p3.x - p1.x) / 6;
        const cp2y = p2.y - (p3.y - p1.y) / 6;

        ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
      }

      ctx.strokeStyle = rgba(this.color, alpha);
      ctx.lineWidth = baseWidth;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.stroke();
    }

    // Shimmer highlight along center
    ctx.beginPath();
    ctx.moveTo(pts[0].x, pts[0].y);
    for (let i = 1; i < pts.length - 1; i++) {
      const p0 = pts[Math.max(0, i - 1)];
      const p1 = pts[i];
      const p2 = pts[Math.min(pts.length - 1, i + 1)];
      const p3 = pts[Math.min(pts.length - 1, i + 2)];
      const cp1x = p1.x + (p2.x - p0.x) / 6;
      const cp1y = p1.y + (p2.y - p0.y) / 6;
      const cp2x = p2.x - (p3.x - p1.x) / 6;
      const cp2y = p2.y - (p3.y - p1.y) / 6;
      ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
    }
    ctx.strokeStyle = rgba(this.color, 0.5);
    ctx.lineWidth = 1.5;
    ctx.stroke();

    ctx.restore();
  }

  private drawDots(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    ctx.save();

    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const lifeRatio = 1 - p.age / 45;
      if (lifeRatio <= 0) continue;

      const size = 2 + lifeRatio * 4;
      const alpha = lifeRatio * 0.8;

      // Outer halo
      ctx.beginPath();
      ctx.arc(p.x, p.y, size + 2, 0, Math.PI * 2);
      ctx.fillStyle = rgba(this.color, alpha * 0.15);
      ctx.fill();

      // Core dot
      ctx.beginPath();
      ctx.arc(p.x, p.y, size, 0, Math.PI * 2);
      ctx.fillStyle = rgba(this.color, alpha);
      ctx.fill();

      // Connecting line to next
      if (i < pts.length - 1) {
        const next = pts[i + 1];
        const nextLife = 1 - next.age / 45;
        ctx.beginPath();
        ctx.moveTo(p.x, p.y);
        ctx.lineTo(next.x, next.y);
        ctx.strokeStyle = rgba(this.color, Math.min(alpha, nextLife) * 0.2);
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    }

    ctx.restore();
  }

  private drawAurora(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    if (pts.length < 3) return;
    ctx.save();

    // Rainbow hue shift over time
    const baseHue = (this.time * 2) % 360;

    for (let layer = 0; layer < 3; layer++) {
      const offset = layer * 40;
      const width = 18 - layer * 5;
      const layerAlpha = 0.08 + layer * 0.05;

      ctx.beginPath();

      // Top edge
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const lifeRatio = 1 - p.age / 45;
        const wave = Math.sin(this.time * 0.03 + i * 0.2 + layer) * 6 * lifeRatio;
        const nx = p.x + wave;
        const ny = p.y - width / 2 + Math.sin(this.time * 0.02 + i * 0.15) * 3;
        if (i === 0) ctx.moveTo(nx, ny);
        else ctx.lineTo(nx, ny);
      }

      // Bottom edge (reverse)
      for (let i = pts.length - 1; i >= 0; i--) {
        const p = pts[i];
        const lifeRatio = 1 - p.age / 45;
        const wave = Math.sin(this.time * 0.03 + i * 0.2 + layer + 1) * 6 * lifeRatio;
        const nx = p.x + wave;
        const ny = p.y + width / 2 + Math.sin(this.time * 0.02 + i * 0.15 + 2) * 3;
        ctx.lineTo(nx, ny);
      }

      ctx.closePath();

      // Gradient fill
      const hueShift = (baseHue + offset) % 360;
      const grd = ctx.createLinearGradient(pts[0].x, pts[0].y, pts[pts.length - 1].x, pts[pts.length - 1].y);
      grd.addColorStop(0, hslToStr(hueShift, 80, 60, 0));
      grd.addColorStop(0.3, hslToStr(hueShift + 30, 80, 60, layerAlpha));
      grd.addColorStop(0.7, hslToStr(hueShift + 60, 80, 60, layerAlpha));
      grd.addColorStop(1, hslToStr(hueShift + 90, 80, 60, 0));
      ctx.fillStyle = grd;
      ctx.fill();
    }

    // Core bright line
    ctx.beginPath();
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      if (i === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    }
    ctx.strokeStyle = rgba(this.color, 0.4);
    ctx.lineWidth = 2;
    ctx.lineCap = "round";
    ctx.stroke();

    ctx.restore();
  }
}

// ─── Click Effect Renderer ──────────────────────────────────────

export class ClickEffectRenderer {
  private clicks: ClickEvent[] = [];
  private style: ClickEffect = "ripple";
  private color = "#00e88a";
  private confettiParticles: ConfettiParticle[] = [];

  setStyle(s: ClickEffect) { this.style = s; }
  setColor(c: string) { this.color = c; }
  clear() { this.clicks = []; this.confettiParticles = []; }

  addClick(x: number, y: number) {
    this.clicks.push({ x, y, age: 0, color: this.color });
    if (this.style === "confetti") {
      this.spawnConfetti(x, y);
    }
  }

  // Auto-simulate clicks at interval for preview
  private simulateInterval: ReturnType<typeof setInterval> | null = null;
  startAutoSimulate(canvas: HTMLCanvasElement) {
    this.stopAutoSimulate();
    const rect = canvas.getBoundingClientRect();
    this.simulateInterval = setInterval(() => {
      const x = rect.width * (0.2 + Math.random() * 0.6);
      const y = rect.height * (0.2 + Math.random() * 0.6);
      this.addClick(x, y);
    }, 1200);
  }
  stopAutoSimulate() {
    if (this.simulateInterval) {
      clearInterval(this.simulateInterval);
      this.simulateInterval = null;
    }
  }

  private spawnConfetti(x: number, y: number) {
    for (let i = 0; i < 24; i++) {
      const angle = (Math.PI * 2 * i) / 24 + (Math.random() - 0.5) * 0.3;
      const speed = 2 + Math.random() * 4;
      const colors = [this.color, "#ff4444", "#4488ff", "#ffcc00", "#ff44cc", "#44ffcc"];
      this.confettiParticles.push({
        x, y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed - 2,
        age: 0,
        maxAge: 40 + Math.random() * 20,
        color: colors[Math.floor(Math.random() * colors.length)],
        size: 2 + Math.random() * 3,
        rotation: Math.random() * Math.PI * 2,
        rotSpeed: (Math.random() - 0.5) * 0.2,
      });
    }
  }

  update() {
    this.clicks = this.clicks.filter(c => {
      c.age++;
      return c.age < 60;
    });
    this.confettiParticles = this.confettiParticles.filter(p => {
      p.age++;
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.12; // gravity
      p.vx *= 0.98;
      p.rotation += p.rotSpeed;
      return p.age < p.maxAge;
    });
  }

  draw(ctx: CanvasRenderingContext2D, _w: number, _h: number) {
    // Don't clear — overlay on top of trail
    if (this.style === "none") return;

    for (const click of this.clicks) {
      switch (this.style) {
        case "ripple": this.drawRipple(ctx, click); break;
        case "spotlight": this.drawSpotlight(ctx, click); break;
        case "ring": this.drawRing(ctx, click); break;
        case "pulse": this.drawPulse(ctx, click); break;
        case "confetti": break; // handled separately
      }
    }

    if (this.style === "confetti") {
      this.drawConfetti(ctx);
    }
  }

  private drawRipple(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 60;
    if (progress >= 1) return;

    for (let i = 0; i < 3; i++) {
      const ringProgress = Math.min(1, (progress - i * 0.1) / 0.7);
      if (ringProgress <= 0 || ringProgress >= 1) continue;

      const radius = ringProgress * 40;
      const alpha = (1 - ringProgress) * 0.6;

      ctx.beginPath();
      ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, alpha);
      ctx.lineWidth = 2.5 - ringProgress * 2;
      ctx.stroke();
    }

    // Center flash
    if (progress < 0.15) {
      const flashAlpha = (1 - progress / 0.15) * 0.8;
      const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, 8);
      grad.addColorStop(0, rgba(c.color, flashAlpha));
      grad.addColorStop(1, rgba(c.color, 0));
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(c.x, c.y, 8, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  private drawSpotlight(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 60;
    if (progress >= 1) return;

    const radius = 8 + progress * 35;
    const alpha = (1 - progress) * 0.5;

    // Inner bright spot
    const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, radius);
    grad.addColorStop(0, rgba(c.color, alpha * 0.8));
    grad.addColorStop(0.3, rgba(c.color, alpha * 0.4));
    grad.addColorStop(0.7, rgba(c.color, alpha * 0.1));
    grad.addColorStop(1, rgba(c.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
    ctx.fill();

    // Cross flare lines
    if (progress < 0.5) {
      const lineAlpha = (1 - progress * 2) * 0.4;
      const lineLen = 6 + progress * 20;
      ctx.strokeStyle = rgba(c.color, lineAlpha);
      ctx.lineWidth = 1.5;
      for (let a = 0; a < 4; a++) {
        const angle = (a * Math.PI) / 4 + progress * 0.5;
        ctx.beginPath();
        ctx.moveTo(c.x + Math.cos(angle) * 3, c.y + Math.sin(angle) * 3);
        ctx.lineTo(c.x + Math.cos(angle) * lineLen, c.y + Math.sin(angle) * lineLen);
        ctx.stroke();
      }
    }
  }

  private drawRing(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 60;
    if (progress >= 1) return;

    // Expanding ring
    const radius = 5 + progress * 30;
    const alpha = (1 - progress) * 0.7;

    ctx.beginPath();
    ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
    ctx.strokeStyle = rgba(c.color, alpha);
    ctx.lineWidth = 3 - progress * 2;
    ctx.stroke();

    // Inner contracting ring
    if (progress < 0.5) {
      const innerRadius = 15 - progress * 20;
      const innerAlpha = (1 - progress * 2) * 0.5;
      ctx.beginPath();
      ctx.arc(c.x, c.y, Math.max(0, innerRadius), 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, innerAlpha);
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Center dot
    const dotSize = 3 * (1 - progress);
    ctx.beginPath();
    ctx.arc(c.x, c.y, dotSize, 0, Math.PI * 2);
    ctx.fillStyle = rgba(c.color, (1 - progress) * 0.9);
    ctx.fill();
  }

  private drawPulse(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 60;
    if (progress >= 1) return;

    // Pulsing circles
    for (let i = 0; i < 4; i++) {
      const phase = (progress + i * 0.15) % 1;
      const radius = phase * 25;
      const alpha = (1 - phase) * 0.5;

      ctx.beginPath();
      ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, alpha);
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Center glow
    const pulse = Math.sin(progress * Math.PI * 6) * 0.3 + 0.7;
    const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, 6);
    grad.addColorStop(0, rgba(c.color, pulse * (1 - progress)));
    grad.addColorStop(1, rgba(c.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(c.x, c.y, 6, 0, Math.PI * 2);
    ctx.fill();
  }

  private drawConfetti(ctx: CanvasRenderingContext2D) {
    for (const p of this.confettiParticles) {
      const lifeRatio = 1 - p.age / p.maxAge;
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(p.rotation);
      ctx.fillStyle = rgba(p.color, lifeRatio * 0.9);
      ctx.fillRect(-p.size / 2, -p.size / 2, p.size, p.size * 0.4);
      ctx.restore();
    }
  }
}

interface ConfettiParticle {
  x: number; y: number;
  vx: number; vy: number;
  age: number; maxAge: number;
  color: string; size: number;
  rotation: number; rotSpeed: number;
}

// ─── Background Grid (for preview canvases) ─────────────────────

export function drawPreviewBackground(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // Subtle dot grid
  ctx.fillStyle = "rgba(255,255,255,0.03)";
  for (let x = 0; x < w; x += 20) {
    for (let y = 0; y < h; y += 20) {
      ctx.beginPath();
      ctx.arc(x, y, 0.5, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // Subtle vignette
  const grad = ctx.createRadialGradient(w / 2, h / 2, Math.min(w, h) * 0.2, w / 2, h / 2, Math.max(w, h) * 0.7);
  grad.addColorStop(0, "rgba(0,0,0,0)");
  grad.addColorStop(1, "rgba(0,0,0,0.15)");
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, w, h);
}
