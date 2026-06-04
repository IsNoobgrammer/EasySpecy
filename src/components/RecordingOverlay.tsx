import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  TrailRenderer, ClickEffectRenderer,
  type TrailStyle, type ClickEffect,
} from "../lib/effects";

/**
 * RecordingOverlay — transparent fullscreen overlay window.
 * Renders trail + click effects at cursor position during recording.
 * Created as a separate Tauri window (transparent, click-through, always-on-top).
 *
 * v3: Receives cursor data via Tauri events from the Rust mouse tracking thread.
 * This fixes the jitter/latency issue where DOM events don't fire on click-through windows.
 */
export function RecordingOverlay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const trailRef = useRef(new TrailRenderer());
  const clickRef = useRef(new ClickEffectRenderer());
  const configRef = useRef<{ trail_style: TrailStyle; click_effect: ClickEffect; color: string; secondaryColor: string }>({
    trail_style: "glow",
    click_effect: "ripple",
    color: "#00e88a",
    secondaryColor: "#ff4488",
  });

  // Load config on mount
  useEffect(() => {
    invoke<any>("get_config").then((cfg: any) => {
      configRef.current = {
        trail_style: cfg.trail_style || "glow",
        click_effect: cfg.click_effect || "ripple",
        color: cfg.cursor_trail_color || "#00e88a",
        secondaryColor: cfg.cursor_secondary_color || "#ff4488",
      };
      trailRef.current.setStyle(configRef.current.trail_style);
      trailRef.current.setColor(configRef.current.color);
      clickRef.current.setStyle(configRef.current.click_effect);
      clickRef.current.setColor(configRef.current.color);
    }).catch(() => {});
  }, []);

  // Resize canvas to fill screen
  useEffect(() => {
    const resize = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    };
    resize();
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);

  // Listen to Tauri events from Rust mouse tracking thread
  useEffect(() => {
    let unlistenMove: (() => void) | null = null;
    let unlistenClick: (() => void) | null = null;

    const setup = async () => {
      // Cursor movement events from Rust (~120Hz)
      unlistenMove = await listen<[number, number]>("cursor-move", (event) => {
        const [x, y] = event.payload;
        trailRef.current.addPoint(x, y);
      });

      // Click events from Rust
      unlistenClick = await listen<[number, number, string]>("cursor-click", (event) => {
        const [x, y, button] = event.payload;
        const color = button === "right" ? configRef.current.secondaryColor : configRef.current.color;
        clickRef.current.addClick(x, y, color);
      });
    };

    setup().catch(() => {
      // Fallback: DOM events (for debugging, won't work on click-through windows)
      const onMove = (e: MouseEvent) => trailRef.current.addPoint(e.clientX, e.clientY);
      const onClick = (e: MouseEvent) => {
        const color = e.button === 2 ? configRef.current.secondaryColor : configRef.current.color;
        clickRef.current.addClick(e.clientX, e.clientY, color);
      };
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mousedown", onClick);
    });

    return () => {
      unlistenMove?.();
      unlistenClick?.();
    };
  }, []);

  // Animation loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    let raf: number;

    const animate = () => {
      const w = canvas.width;
      const h = canvas.height;

      ctx.clearRect(0, 0, w, h);

      trailRef.current.update();
      trailRef.current.draw(ctx, w, h);

      clickRef.current.update();
      clickRef.current.draw(ctx, w, h);

      raf = requestAnimationFrame(animate);
    };

    animate();
    return () => cancelAnimationFrame(raf);
  }, []);

  // Make window click-through (ignore pointer events on canvas)
  useEffect(() => {
    const win = getCurrentWindow();
    win.setIgnoreCursorEvents(true).catch(() => {});
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        width: "100vw",
        height: "100vh",
        pointerEvents: "none",
        zIndex: 99999,
      }}
    />
  );
}
