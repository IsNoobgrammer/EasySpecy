import { useState, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { Rnd } from "react-rnd";
import { Icon } from "./Icon";

// ═══ TYPES ═══

export type WebcamShape = "circle" | "rounded" | "squircle";

export interface WebcamOverlayConfig {
  x: number;
  y: number;
  size: number;
  shape: WebcamShape;
  borderColor: string;
  borderWidth: number;
  opacity: number;
}

interface WebcamPreviewProps {
  onSave: (config: WebcamOverlayConfig) => void;
  onCancel: () => void;
  initial?: Partial<WebcamOverlayConfig>;
  /** Recording resolution for aspect ratio */
  recordingWidth?: number;
  recordingHeight?: number;
}

// ═══ SHAPE STYLES ═══

function getShapeStyle(shape: WebcamShape): React.CSSProperties {
  switch (shape) {
    case "circle":
      return { borderRadius: "50%" };
    case "rounded":
      return { borderRadius: "16px" };
    case "squircle":
      return { borderRadius: "25%" }; // CSS approximation
    default:
      return { borderRadius: "50%" };
  }
}

// ═══ SIZE PRESETS ═══

const SIZE_PRESETS = [
  { label: "S", value: 150 },
  { label: "M", value: 200 },
  { label: "L", value: 250 },
  { label: "XL", value: 300 },
];

const BORDER_COLORS = [
  "#00e88a", "#3b82f6", "#a855f7", "#ec4899",
  "#f97316", "#06b6d4", "#ffffff", "#eab308",
];

const SHAPES: { id: WebcamShape; label: string; icon: string }[] = [
  { id: "circle", label: "Circle", icon: "radio_button_unchecked" },
  { id: "rounded", label: "Rounded", icon: "crop_square" },
  { id: "squircle", label: "Squircle", icon: "rounded_corner" },
];

// ═══ MAIN COMPONENT ═══

export function WebcamPreview({ onSave, onCancel, initial, recordingWidth = 1920, recordingHeight = 1080 }: WebcamPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [stream, setStream] = useState<MediaStream | null>(null);
  const [cameraReady, setCameraReady] = useState(false);

  // Overlay state
  const [overlay, setOverlay] = useState<WebcamOverlayConfig>({
    x: initial?.x ?? 860,
    y: initial?.y ?? 440,
    size: initial?.size ?? 200,
    shape: initial?.shape ?? "circle",
    borderColor: initial?.borderColor ?? "#00e88a",
    borderWidth: initial?.borderWidth ?? 3,
    opacity: initial?.opacity ?? 1,
  });

  // Start webcam
  useEffect(() => {
    let mounted = true;
    navigator.mediaDevices.getUserMedia({
      video: { width: { ideal: 640 }, height: { ideal: 480 }, frameRate: { ideal: 30 } },
      audio: false,
    }).then((s) => {
      if (!mounted) return;
      setStream(s);
      if (videoRef.current) {
        videoRef.current.srcObject = s;
        videoRef.current.onloadedmetadata = () => setCameraReady(true);
      }
    }).catch(() => {
      // Camera not available — show placeholder
      setCameraReady(false);
    });
    return () => {
      mounted = false;
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  const update = <K extends keyof WebcamOverlayConfig>(key: K, value: WebcamOverlayConfig[K]) =>
    setOverlay((prev) => ({ ...prev, [key]: value }));

  // Calculate preview scale to fit in the preview area
  const PREVIEW_MAX_W = 900;
  const PREVIEW_MAX_H = 560;
  const scaleX = PREVIEW_MAX_W / recordingWidth;
  const scaleY = PREVIEW_MAX_H / recordingHeight;
  const scale = Math.min(scaleX, scaleY, 1);
  const previewW = recordingWidth * scale;
  const previewH = recordingHeight * scale;

  const shapeStyle = getShapeStyle(overlay.shape);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[9999] flex"
      style={{ background: "var(--bg-base)" }}
    >
      {/* ═══ LEFT SIDEBAR — Controls ═══ */}
      <div
        className="w-72 flex-shrink-0 overflow-y-auto flex flex-col"
        style={{
          background: "var(--bg-surface)",
          borderRight: "var(--border-thin) solid var(--border-default)",
        }}
      >
        {/* Header */}
        <div className="px-5 py-4 flex items-center justify-between" style={{ borderBottom: "var(--border-thin) solid var(--border-default)" }}>
          <div>
            <div className="font-mono text-sm font-semibold" style={{ color: "var(--text-primary)", letterSpacing: "0.05em" }}>
              WEBCAM OVERLAY
            </div>
            <div className="font-mono mt-0.5" style={{ color: "var(--text-muted)", fontSize: "0.6rem" }}>
              Configure position, shape & style
            </div>
          </div>
          <motion.button
            onClick={onCancel}
            className="p-1 cursor-pointer"
            style={{ color: "var(--text-muted)" }}
            whileHover={{ color: "var(--text-primary)", scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="close" size={18} />
          </motion.button>
        </div>

        <div className="flex-1 px-5 py-4 space-y-5">
          {/* ── Shape ── */}
          <Section label="SHAPE">
            <div className="grid grid-cols-3 gap-2">
              {SHAPES.map((s) => (
                <motion.button
                  key={s.id}
                  onClick={() => update("shape", s.id)}
                  className="flex flex-col items-center gap-1.5 py-2.5 px-2 cursor-pointer"
                  style={{
                    background: overlay.shape === s.id ? "var(--surface-container-high)" : "var(--bg-base)",
                    border: `var(--border-thin) solid ${overlay.shape === s.id ? "var(--accent-primary)" : "var(--border-default)"}`,
                    borderRadius: "var(--radius-sm)",
                    color: overlay.shape === s.id ? "var(--accent-primary)" : "var(--text-secondary)",
                  }}
                  whileHover={{ scale: 1.03 }}
                  whileTap={{ scale: 0.97 }}
                >
                  <Icon name={s.icon} size={20} />
                  <span className="font-mono" style={{ fontSize: "0.6rem", letterSpacing: "0.04em" }}>{s.label}</span>
                </motion.button>
              ))}
            </div>
          </Section>

          {/* ── Size ── */}
          <Section label="SIZE">
            <div className="grid grid-cols-4 gap-1.5 mb-3">
              {SIZE_PRESETS.map((p) => (
                <motion.button
                  key={p.value}
                  onClick={() => update("size", p.value)}
                  className="font-mono text-xs py-1.5 cursor-pointer"
                  style={{
                    background: overlay.size === p.value ? "var(--accent-primary)" : "var(--bg-base)",
                    color: overlay.size === p.value ? "var(--on-primary)" : "var(--text-secondary)",
                    border: "var(--border-thin) solid var(--border-default)",
                    borderRadius: "var(--radius-sm)",
                    letterSpacing: "0.05em",
                  }}
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                >
                  {p.label}
                </motion.button>
              ))}
            </div>
            <input
              type="range"
              min={80}
              max={400}
              value={overlay.size}
              onChange={(e) => update("size", Number(e.target.value))}
              className="w-full accent-current"
              style={{ accentColor: "var(--accent-primary)" }}
            />
            <div className="flex justify-between font-mono mt-1" style={{ fontSize: "0.55rem", color: "var(--text-muted)" }}>
              <span>80px</span>
              <span style={{ color: "var(--accent-primary)" }}>{overlay.size}px</span>
              <span>400px</span>
            </div>
          </Section>

          {/* ── Border ── */}
          <Section label="BORDER">
            <div className="flex items-center gap-3 mb-3">
              <span className="font-mono" style={{ fontSize: "0.65rem", color: "var(--text-secondary)" }}>Width</span>
              <input
                type="range"
                min={0}
                max={8}
                value={overlay.borderWidth}
                onChange={(e) => update("borderWidth", Number(e.target.value))}
                className="flex-1"
                style={{ accentColor: "var(--accent-primary)" }}
              />
              <span className="font-mono w-6 text-right" style={{ fontSize: "0.65rem", color: "var(--accent-primary)" }}>
                {overlay.borderWidth}
              </span>
            </div>
            <div className="flex gap-2 flex-wrap">
              {BORDER_COLORS.map((c) => (
                <motion.button
                  key={c}
                  onClick={() => update("borderColor", c)}
                  className="w-7 h-7 cursor-pointer"
                  style={{
                    background: c,
                    borderRadius: "50%",
                    border: overlay.borderColor === c ? "3px solid var(--text-primary)" : "2px solid var(--border-default)",
                    boxShadow: overlay.borderColor === c ? "0 0 0 2px var(--accent-primary)" : "none",
                  }}
                  whileHover={{ scale: 1.15 }}
                  whileTap={{ scale: 0.9 }}
                />
              ))}
            </div>
          </Section>

          {/* ── Opacity ── */}
          <Section label="OPACITY">
            <input
              type="range"
              min={30}
              max={100}
              value={Math.round(overlay.opacity * 100)}
              onChange={(e) => update("opacity", Number(e.target.value) / 100)}
              className="w-full"
              style={{ accentColor: "var(--accent-primary)" }}
            />
            <div className="flex justify-between font-mono mt-1" style={{ fontSize: "0.55rem", color: "var(--text-muted)" }}>
              <span>30%</span>
              <span style={{ color: "var(--accent-primary)" }}>{Math.round(overlay.opacity * 100)}%</span>
              <span>100%</span>
            </div>
          </Section>
        </div>

        {/* Bottom buttons */}
        <div className="px-5 py-4 flex gap-3" style={{ borderTop: "var(--border-thin) solid var(--border-default)" }}>
          <motion.button
            onClick={onCancel}
            className="flex-1 py-2.5 font-mono text-xs uppercase cursor-pointer"
            style={{
              background: "transparent",
              color: "var(--text-secondary)",
              border: "var(--border-thin) solid var(--border-default)",
              borderRadius: "var(--radius-sm)",
              letterSpacing: "0.05em",
            }}
            whileHover={{ borderColor: "var(--text-secondary)" }}
            whileTap={{ scale: 0.97 }}
          >
            Cancel
          </motion.button>
          <motion.button
            onClick={() => onSave(overlay)}
            className="flex-1 py-2.5 font-mono text-xs uppercase font-bold cursor-pointer"
            style={{
              background: "var(--accent-primary-container, #00e88a)",
              color: "var(--on-primary, #00391e)",
              border: "none",
              borderRadius: "var(--radius-sm)",
              letterSpacing: "0.05em",
            }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.97 }}
          >
            Apply
          </motion.button>
        </div>
      </div>

      {/* ═══ MAIN AREA — Preview ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center overflow-hidden" style={{ background: "var(--bg-base)" }}>
        {/* Recording simulation */}
        <div
          className="relative"
          style={{
            width: previewW,
            height: previewH,
            background: "var(--surface-container)",
            borderRadius: "var(--radius-md)",
            border: "var(--border-thin) solid var(--border-default)",
            overflow: "hidden",
          }}
        >
          {/* Placeholder "screen content" */}
          <div className="absolute inset-0 flex items-center justify-center" style={{ opacity: 0.15 }}>
            <div className="text-center">
              <Icon name="desktop_windows" size={64} />
              <div className="font-mono mt-2" style={{ fontSize: "0.7rem", color: "var(--text-muted)" }}>
                Screen Recording Area
              </div>
            </div>
          </div>

          {/* Grid lines for reference */}
          <div className="absolute inset-0 pointer-events-none" style={{ opacity: 0.05 }}>
            <div className="absolute" style={{ left: "50%", top: 0, bottom: 0, width: "1px", background: "var(--text-primary)" }} />
            <div className="absolute" style={{ top: "50%", left: 0, right: 0, height: "1px", background: "var(--text-primary)" }} />
          </div>

          {/* ═══ RND WEBCAM OVERLAY ═══ */}
          <Rnd
            size={{ width: overlay.size * scale, height: overlay.size * scale }}
            position={{ x: overlay.x * scale, y: overlay.y * scale }}
            onDragStop={(_, d) => {
              update("x", Math.round(d.x / scale));
              update("y", Math.round(d.y / scale));
            }}
            onResizeStop={(_, __, ref, ___, pos) => {
              const newSize = Math.round(parseInt(ref.style.width) / scale);
              update("size", newSize);
              update("x", Math.round(pos.x / scale));
              update("y", Math.round(pos.y / scale));
            }}
            minWidth={40}
            minHeight={40}
            maxWidth={400}
            maxHeight={400}
            lockAspectRatio
            bounds="parent"
            resizeHandleComponent={{
              bottomRight: <ResizeHandle />,
            }}
            style={{
              opacity: overlay.opacity,
              zIndex: 10,
            }}
          >
            <div
              className="w-full h-full relative"
              style={{
                ...shapeStyle,
                border: overlay.borderWidth > 0 ? `${overlay.borderWidth}px solid ${overlay.borderColor}` : "none",
                overflow: "hidden",
                boxShadow: `0 0 20px ${overlay.borderColor}33, 0 4px 12px rgba(0,0,0,0.3)`,
              }}
            >
              {cameraReady ? (
                <video
                  ref={videoRef}
                  autoPlay
                  muted
                  playsInline
                  className="w-full h-full object-cover"
                  style={shapeStyle}
                />
              ) : (
                <div
                  className="w-full h-full flex items-center justify-center"
                  style={{ background: "var(--surface-container-highest)" }}
                >
                  <div className="text-center">
                    <Icon name="person" size={48} />
                    <div className="font-mono mt-1" style={{ fontSize: "0.55rem", color: "var(--text-muted)" }}>
                      No Camera
                    </div>
                  </div>
                </div>
              )}
            </div>
          </Rnd>
        </div>

        {/* Caption */}
        <div className="mt-4 font-mono text-center" style={{ fontSize: "0.65rem", color: "var(--text-muted)", letterSpacing: "0.04em" }}>
          Drag to reposition · Scroll to resize
        </div>

        {/* Quick shape switch */}
        <div className="mt-3 flex gap-2">
          {SHAPES.map((s) => (
            <motion.button
              key={s.id}
              onClick={() => update("shape", s.id)}
              className="px-3 py-1.5 font-mono cursor-pointer flex items-center gap-1.5"
              style={{
                fontSize: "0.6rem",
                letterSpacing: "0.04em",
                background: overlay.shape === s.id ? "var(--accent-primary-container, #00e88a)" : "var(--bg-surface)",
                color: overlay.shape === s.id ? "var(--on-primary)" : "var(--text-secondary)",
                border: `var(--border-thin) solid ${overlay.shape === s.id ? "var(--accent-primary)" : "var(--border-default)"}`,
                borderRadius: "var(--radius-sm)",
              }}
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
            >
              <Icon name={s.icon} size={12} />
              {s.label}
            </motion.button>
          ))}
        </div>
      </div>
    </motion.div>
  );
}

// ═══ HELPERS ═══

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="font-mono mb-2" style={{ fontSize: "0.6rem", color: "var(--text-muted)", letterSpacing: "0.08em", fontWeight: 700 }}>
        {label}
      </div>
      {children}
    </div>
  );
}

function ResizeHandle() {
  return (
    <div
      className="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize"
      style={{
        borderRight: "3px solid var(--accent-primary)",
        borderBottom: "3px solid var(--accent-primary)",
        opacity: 0.8,
      }}
    />
  );
}
