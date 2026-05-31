/**
 * EasySpecy — Canvas Effects Engine v2
 * Premium quality trail + click effects for Customization preview and RecordingOverlay.
 */

// ─── Types ───────────────────────────────────────────────────────

export interface Point {
  x: number;
  y: number;
  age: number;
  vx: number;
  vy: number;
  size: number;
  alpha: number;
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
  return `rgba(${r},${g},${b},${Math.max(0, Math.min(1, alpha))})`;
}

function hsl(h: number, s: number, l: number, a: number): string {
  return `hsla(${h % 360},${s}%,${l}%,${Math.max(0, Math.min(1, a))})`;
}

function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

function easeInOutQuad(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
}

// ─── Smooth Catmull-Rom spline through points ────────────────────

function drawSmoothPath(ctx: CanvasRenderingContext2D, pts: Point[], _closed = false) {
  if (pts.length < 2) return;
  ctx.moveTo(pts[0].x, pts[0].y);

  if (pts.length === 2) {
    ctx.lineTo(pts[1].x, pts[1].y);
    return;
  }

  for (let i = 0; i < pts.length - 1; i++) {
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
}

// ─── Trail Renderer ─────────────────────────────────────────────

export class TrailRenderer {
  private points: Point[] = [];
  private maxPoints = 80;
  private style: TrailStyle = "glow";
  private color = "#00e88a";
  private time = 0;
  private lastX = 0;
  private lastY = 0;
  private velocity = 0;

  setStyle(s: TrailStyle) { this.style = s; }
  setColor(c: string) { this.color = c; }
  clear() { this.points = []; }

  addPoint(x: number, y: number) {
    const dx = x - this.lastX;
    const dy = y - this.lastY;
    const dist = Math.hypot(dx, dy);
    if (dist < 1.5) return;

    this.velocity = Math.min(dist, 30);
    this.lastX = x;
    this.lastY = y;

    this.points.push({
      x, y, age: 0,
      vx: dx * 0.1,
      vy: dy * 0.1,
      size: 1,
      alpha: 1,
    });
    if (this.points.length > this.maxPoints) this.points.shift();
  }

  update() {
    this.time++;
    this.velocity *= 0.92;
    this.points = this.points.filter(p => {
      p.age++;
      p.x += p.vx * 0.3;
      p.y += p.vy * 0.3;
      p.vx *= 0.95;
      p.vy *= 0.95;
      return p.age < 50;
    });
  }

  draw(ctx: CanvasRenderingContext2D, _w: number, _h: number) {
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
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    // Multi-layer glow: outer diffuse → mid → core
    const velFactor = Math.min(1, this.velocity / 15);
    const layers = [
      { width: 28, alpha: 0.035, blur: 12 },
      { width: 16, alpha: 0.08, blur: 6 },
      { width: 8, alpha: 0.25, blur: 2 },
      { width: 3, alpha: 0.9, blur: 0 },
    ];

    for (const layer of layers) {
      ctx.save();
      if (layer.blur > 0) {
        ctx.filter = `blur(${layer.blur}px)`;
      }

      const w = layer.width * (0.6 + velFactor * 0.4);

      ctx.beginPath();
      drawSmoothPath(ctx, pts);
      ctx.strokeStyle = rgba(this.color, layer.alpha);
      ctx.lineWidth = w;
      ctx.stroke();
      ctx.restore();
    }

    // Head glow — pulsing radial
    const head = pts[pts.length - 1];
    const headLife = Math.max(0, 1 - head.age / 25);
    const pulse = 1 + Math.sin(this.time * 0.12) * 0.15;
    const headRadius = (14 + velFactor * 8) * pulse;

    const grad = ctx.createRadialGradient(head.x, head.y, 0, head.x, head.y, headRadius);
    grad.addColorStop(0, rgba(this.color, 0.7 * headLife));
    grad.addColorStop(0.3, rgba(this.color, 0.3 * headLife));
    grad.addColorStop(0.6, rgba(this.color, 0.08 * headLife));
    grad.addColorStop(1, rgba(this.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(head.x, head.y, headRadius, 0, Math.PI * 2);
    ctx.fill();

    // Tiny white core at head
    ctx.beginPath();
    ctx.arc(head.x, head.y, 2 * headLife, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(255,255,255,${0.9 * headLife})`;
    ctx.fill();

    ctx.restore();
  }

  private drawParticles(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    ctx.save();

    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const life = 1 - p.age / 50;
      if (life <= 0) continue;

      // Main ember
      const size = (2 + life * 4) * (0.5 + this.velocity / 20);
      const glow = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, size * 2);
      glow.addColorStop(0, rgba(this.color, life * 0.9));
      glow.addColorStop(0.5, rgba(this.color, life * 0.3));
      glow.addColorStop(1, rgba(this.color, 0));
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 2, 0, Math.PI * 2);
      ctx.fill();

      // Core dot
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 0.6, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${life * 0.7})`;
      ctx.fill();

      // Orbiting sparks
      const sparkCount = Math.floor(life * 5);
      for (let s = 0; s < sparkCount; s++) {
        const angle = this.time * 0.08 + i * 1.3 + s * 2.4;
        const dist = 3 + Math.sin(this.time * 0.06 + s) * 6 * life;
        const sx = p.x + Math.cos(angle) * dist;
        const sy = p.y + Math.sin(angle) * dist;
        const sparkSize = 0.8 + life * 1.2;
        ctx.beginPath();
        ctx.arc(sx, sy, sparkSize, 0, Math.PI * 2);
        ctx.fillStyle = rgba(this.color, life * 0.5 * (0.5 + Math.sin(angle * 3) * 0.5));
        ctx.fill();
      }
    }

    // Ambient fireflies near head
    if (pts.length > 0) {
      const head = pts[pts.length - 1];
      for (let i = 0; i < 12; i++) {
        const angle = this.time * 0.025 + i * 0.524;
        const dist = 8 + Math.sin(this.time * 0.04 + i * 1.7) * 12;
        const fx = head.x + Math.cos(angle) * dist;
        const fy = head.y + Math.sin(angle) * dist;
        const flicker = 0.3 + Math.sin(this.time * 0.12 + i * 2.3) * 0.25;
        const fSize = 1 + Math.sin(this.time * 0.07 + i * 1.5) * 0.6;
        ctx.beginPath();
        ctx.arc(fx, fy, fSize, 0, Math.PI * 2);
        ctx.fillStyle = rgba(this.color, flicker);
        ctx.fill();
      }
    }

    ctx.restore();
  }

  private drawRibbon(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    if (pts.length < 3) return;
    ctx.save();

    // Layered ribbon: wide translucent → narrow opaque → thin highlight
    const layers = [
      { width: 18, alpha: 0.04, blur: 4 },
      { width: 10, alpha: 0.12, blur: 1 },
      { width: 4, alpha: 0.35, blur: 0 },
      { width: 1.5, alpha: 0.7, blur: 0 },
    ];

    for (const layer of layers) {
      ctx.save();
      if (layer.blur > 0) ctx.filter = `blur(${layer.blur}px)`;
      ctx.beginPath();
      drawSmoothPath(ctx, pts);
      ctx.strokeStyle = rgba(this.color, layer.alpha);
      ctx.lineWidth = layer.width;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.stroke();
      ctx.restore();
    }

    // Flowing shimmer along the ribbon
    ctx.beginPath();
    drawSmoothPath(ctx, pts);
    const shimmerAlpha = 0.15 + Math.sin(this.time * 0.08) * 0.1;
    ctx.strokeStyle = `rgba(255,255,255,${shimmerAlpha})`;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 8]);
    ctx.lineDashOffset = -this.time * 2;
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.restore();
  }

  private drawDots(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    ctx.save();

    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const life = 1 - p.age / 50;
      if (life <= 0) continue;

      const size = (2 + life * 5) * easeOutCubic(life);
      const alpha = life * 0.85;

      // Outer halo
      ctx.beginPath();
      ctx.arc(p.x, p.y, size + 3, 0, Math.PI * 2);
      ctx.fillStyle = rgba(this.color, alpha * 0.1);
      ctx.fill();

      // Main dot
      ctx.beginPath();
      ctx.arc(p.x, p.y, size, 0, Math.PI * 2);
      ctx.fillStyle = rgba(this.color, alpha);
      ctx.fill();

      // White center
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 0.35, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${alpha * 0.6})`;
      ctx.fill();

      // Connection line to next
      if (i < pts.length - 1) {
        const next = pts[i + 1];
        const nextLife = 1 - next.age / 50;
        ctx.beginPath();
        ctx.moveTo(p.x, p.y);
        ctx.lineTo(next.x, next.y);
        ctx.strokeStyle = rgba(this.color, Math.min(alpha, nextLife) * 0.15);
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    }

    ctx.restore();
  }

  private drawAurora(ctx: CanvasRenderingContext2D) {
    const pts = this.points;
    if (pts.length < 4) return;
    ctx.save();

    const baseHue = (this.time * 1.5) % 360;

    // Multiple aurora layers with different wave offsets
    for (let layer = 0; layer < 4; layer++) {
      const offset = layer * 35;
      const width = 22 - layer * 4;
      const layerAlpha = 0.04 + layer * 0.03;
      const waveAmp = 8 + layer * 3;
      const waveFreq = 0.15 + layer * 0.05;

      ctx.beginPath();

      // Top edge with wave
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const life = 1 - p.age / 50;
        const wave = Math.sin(this.time * 0.025 + i * waveFreq + layer * 1.2) * waveAmp * life;
        const nx = p.x + wave;
        const ny = p.y - width / 2 + Math.sin(this.time * 0.018 + i * 0.12 + layer) * 4;
        if (i === 0) ctx.moveTo(nx, ny);
        else ctx.lineTo(nx, ny);
      }

      // Bottom edge (reverse)
      for (let i = pts.length - 1; i >= 0; i--) {
        const p = pts[i];
        const life = 1 - p.age / 50;
        const wave = Math.sin(this.time * 0.025 + i * waveFreq + layer * 1.2 + 2) * waveAmp * life;
        ctx.lineTo(p.x + wave, p.y + width / 2 + Math.sin(this.time * 0.018 + i * 0.12 + layer + 2) * 4);
      }

      ctx.closePath();

      // Gradient fill
      const hue = (baseHue + offset) % 360;
      const grd = ctx.createLinearGradient(pts[0].x, pts[0].y, pts[pts.length - 1].x, pts[pts.length - 1].y);
      grd.addColorStop(0, hsl(hue, 80, 55, 0));
      grd.addColorStop(0.2, hsl(hue + 20, 85, 60, layerAlpha));
      grd.addColorStop(0.5, hsl(hue + 50, 80, 55, layerAlpha * 1.2));
      grd.addColorStop(0.8, hsl(hue + 80, 85, 60, layerAlpha));
      grd.addColorStop(1, hsl(hue + 100, 80, 55, 0));
      ctx.fillStyle = grd;
      ctx.fill();
    }

    // Core bright spine
    ctx.beginPath();
    drawSmoothPath(ctx, pts);
    ctx.strokeStyle = rgba(this.color, 0.35);
    ctx.lineWidth = 2.5;
    ctx.lineCap = "round";
    ctx.stroke();

    // Thin white highlight
    ctx.beginPath();
    drawSmoothPath(ctx, pts);
    ctx.strokeStyle = `rgba(255,255,255,${0.1 + Math.sin(this.time * 0.06) * 0.08})`;
    ctx.lineWidth = 1;
    ctx.stroke();

    ctx.restore();
  }
}

// ─── Click Effect Renderer ──────────────────────────────────────

interface ConfettiParticle {
  x: number; y: number;
  vx: number; vy: number;
  age: number; maxAge: number;
  color: string; size: number;
  rotation: number; rotSpeed: number;
  shape: "rect" | "circle" | "triangle";
}

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
    if (this.style === "confetti") this.spawnConfetti(x, y);
  }

  private simulateInterval: ReturnType<typeof setInterval> | null = null;
  startAutoSimulate(canvas: HTMLCanvasElement) {
    this.stopAutoSimulate();
    const rect = canvas.getBoundingClientRect();
    this.simulateInterval = setInterval(() => {
      const x = rect.width * (0.15 + Math.random() * 0.7);
      const y = rect.height * (0.15 + Math.random() * 0.7);
      this.addClick(x, y);
    }, 1400);
  }
  stopAutoSimulate() {
    if (this.simulateInterval) {
      clearInterval(this.simulateInterval);
      this.simulateInterval = null;
    }
  }

  private spawnConfetti(x: number, y: number) {
    const colors = [this.color, "#ff4455", "#4488ff", "#ffcc22", "#ff44cc", "#44ffcc", "#ff8844"];
    const shapes: ConfettiParticle["shape"][] = ["rect", "circle", "triangle"];
    for (let i = 0; i < 30; i++) {
      const angle = (Math.PI * 2 * i) / 30 + (Math.random() - 0.5) * 0.4;
      const speed = 2.5 + Math.random() * 5;
      this.confettiParticles.push({
        x, y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed - 2.5,
        age: 0,
        maxAge: 45 + Math.random() * 25,
        color: colors[Math.floor(Math.random() * colors.length)],
        size: 2 + Math.random() * 4,
        rotation: Math.random() * Math.PI * 2,
        rotSpeed: (Math.random() - 0.5) * 0.25,
        shape: shapes[Math.floor(Math.random() * shapes.length)],
      });
    }
  }

  update() {
    this.clicks = this.clicks.filter(c => { c.age++; return c.age < 65; });
    this.confettiParticles = this.confettiParticles.filter(p => {
      p.age++;
      p.x += p.vx;
      p.y += p.vy;
      p.vy += 0.13;
      p.vx *= 0.985;
      p.rotation += p.rotSpeed;
      return p.age < p.maxAge;
    });
  }

  draw(ctx: CanvasRenderingContext2D, _w?: number, _h?: number) {
    if (this.style === "none") return;

    for (const click of this.clicks) {
      switch (this.style) {
        case "ripple": this.drawRipple(ctx, click); break;
        case "spotlight": this.drawSpotlight(ctx, click); break;
        case "ring": this.drawRing(ctx, click); break;
        case "pulse": this.drawPulse(ctx, click); break;
        case "confetti": break;
      }
    }

    if (this.style === "confetti") {
      this.drawConfetti(ctx);
    }
  }

  private drawRipple(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 65;
    if (progress >= 1) return;

    // 3 expanding rings with staggered timing
    for (let i = 0; i < 3; i++) {
      const ringT = easeInOutQuad(Math.max(0, Math.min(1, (progress - i * 0.12) / 0.65)));
      if (ringT <= 0 || ringT >= 1) continue;

      const radius = ringT * 45;
      const alpha = (1 - ringT) * 0.7;
      const width = 3 - ringT * 2.5;

      ctx.beginPath();
      ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, alpha);
      ctx.lineWidth = width;
      ctx.stroke();
    }

    // Center flash with falloff
    if (progress < 0.2) {
      const flashT = 1 - progress / 0.2;
      const flashSize = 6 + (1 - flashT) * 4;
      const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, flashSize);
      grad.addColorStop(0, `rgba(255,255,255,${flashT * 0.9})`);
      grad.addColorStop(0.3, rgba(c.color, flashT * 0.7));
      grad.addColorStop(1, rgba(c.color, 0));
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(c.x, c.y, flashSize, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  private drawSpotlight(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 65;
    if (progress >= 1) return;

    const radius = 6 + easeOutCubic(progress) * 40;
    const alpha = (1 - progress) * 0.55;

    // Main spotlight gradient
    const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, radius);
    grad.addColorStop(0, `rgba(255,255,255,${alpha * 0.5})`);
    grad.addColorStop(0.15, rgba(c.color, alpha * 0.7));
    grad.addColorStop(0.4, rgba(c.color, alpha * 0.3));
    grad.addColorStop(0.75, rgba(c.color, alpha * 0.06));
    grad.addColorStop(1, rgba(c.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
    ctx.fill();

    // Cross flare rays
    if (progress < 0.6) {
      const rayAlpha = (1 - progress / 0.6) * 0.45;
      const rayLen = 4 + progress * 25;
      ctx.strokeStyle = rgba(c.color, rayAlpha);
      ctx.lineWidth = 1.5;
      for (let a = 0; a < 6; a++) {
        const angle = (a * Math.PI) / 3 + progress * 0.8;
        ctx.beginPath();
        ctx.moveTo(c.x + Math.cos(angle) * 2, c.y + Math.sin(angle) * 2);
        ctx.lineTo(c.x + Math.cos(angle) * rayLen, c.y + Math.sin(angle) * rayLen);
        ctx.stroke();
      }
    }
  }

  private drawRing(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 65;
    if (progress >= 1) return;

    // Expanding ring
    const outerR = 4 + easeOutCubic(progress) * 35;
    const outerAlpha = (1 - progress) * 0.75;
    ctx.beginPath();
    ctx.arc(c.x, c.y, outerR, 0, Math.PI * 2);
    ctx.strokeStyle = rgba(c.color, outerAlpha);
    ctx.lineWidth = 3.5 - progress * 2.5;
    ctx.stroke();

    // Inner contracting ring
    if (progress < 0.45) {
      const innerT = progress / 0.45;
      const innerR = 18 * (1 - easeOutCubic(innerT));
      const innerAlpha = (1 - innerT) * 0.6;
      ctx.beginPath();
      ctx.arc(c.x, c.y, Math.max(0, innerR), 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, innerAlpha);
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Center dot with white core
    const dotSize = 3.5 * (1 - progress);
    if (dotSize > 0.3) {
      ctx.beginPath();
      ctx.arc(c.x, c.y, dotSize, 0, Math.PI * 2);
      ctx.fillStyle = rgba(c.color, (1 - progress) * 0.9);
      ctx.fill();
      ctx.beginPath();
      ctx.arc(c.x, c.y, dotSize * 0.4, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${(1 - progress) * 0.8})`;
      ctx.fill();
    }
  }

  private drawPulse(ctx: CanvasRenderingContext2D, c: ClickEvent) {
    const progress = c.age / 65;
    if (progress >= 1) return;

    // Concentric pulsing rings
    for (let i = 0; i < 5; i++) {
      const phase = (progress * 1.5 + i * 0.18) % 1;
      const radius = phase * 30;
      const alpha = (1 - phase) * 0.55;

      ctx.beginPath();
      ctx.arc(c.x, c.y, radius, 0, Math.PI * 2);
      ctx.strokeStyle = rgba(c.color, alpha);
      ctx.lineWidth = 2.5 - phase * 2;
      ctx.stroke();
    }

    // Breathing center glow
    const breathe = Math.sin(progress * Math.PI * 8) * 0.3 + 0.7;
    const glowSize = 5 + breathe * 3;
    const grad = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, glowSize);
    grad.addColorStop(0, rgba(c.color, breathe * (1 - progress) * 0.8));
    grad.addColorStop(1, rgba(c.color, 0));
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(c.x, c.y, glowSize, 0, Math.PI * 2);
    ctx.fill();
  }

  private drawConfetti(ctx: CanvasRenderingContext2D) {
    for (const p of this.confettiParticles) {
      const life = 1 - p.age / p.maxAge;
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(p.rotation);
      ctx.globalAlpha = life * 0.9;
      ctx.fillStyle = p.color;

      if (p.shape === "rect") {
        ctx.fillRect(-p.size / 2, -p.size / 4, p.size, p.size / 2);
      } else if (p.shape === "circle") {
        ctx.beginPath();
        ctx.arc(0, 0, p.size / 2, 0, Math.PI * 2);
        ctx.fill();
      } else {
        ctx.beginPath();
        ctx.moveTo(0, -p.size / 2);
        ctx.lineTo(p.size / 2, p.size / 2);
        ctx.lineTo(-p.size / 2, p.size / 2);
        ctx.closePath();
        ctx.fill();
      }

      ctx.globalAlpha = 1;
      ctx.restore();
    }
  }
}

// ─── Background Grid (for preview canvases) ─────────────────────

export function drawPreviewBackground(ctx: CanvasRenderingContext2D, w: number, h: number) {
  // Subtle dot grid
  ctx.fillStyle = "rgba(255,255,255,0.025)";
  for (let x = 0; x < w; x += 16) {
    for (let y = 0; y < h; y += 16) {
      ctx.beginPath();
      ctx.arc(x, y, 0.5, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // Subtle vignette
  const grad = ctx.createRadialGradient(
    w / 2, h / 2, Math.min(w, h) * 0.15,
    w / 2, h / 2, Math.max(w, h) * 0.65
  );
  grad.addColorStop(0, "rgba(0,0,0,0)");
  grad.addColorStop(1, "rgba(0,0,0,0.18)");
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, w, h);
}
