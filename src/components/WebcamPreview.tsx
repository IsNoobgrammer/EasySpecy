import { useState, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { Rnd } from "react-rnd";
import { Icon } from "./Icon";
import { invoke } from "@tauri-apps/api/core";

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
  device: string;
  sharpen: number;
  brightness: number;
  contrast: number;
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
  const streamRef = useRef<MediaStream | null>(null);
  const [cameraReady, setCameraReady] = useState(false);
  const [webcamDevices, setWebcamDevices] = useState<{ index: string; name: string }[]>([]);

  // Overlay state
  const [overlay, setOverlay] = useState<WebcamOverlayConfig>({
    x: initial?.x ?? 860,
    y: initial?.y ?? 440,
    size: initial?.size ?? 200,
    shape: initial?.shape ?? "circle",
    borderColor: initial?.borderColor ?? "#00e88a",
    borderWidth: initial?.borderWidth ?? 3,
    opacity: initial?.opacity ?? 1.0,
    device: initial?.device ?? "default",
    sharpen: initial?.sharpen ?? 0.3,
    brightness: initial?.brightness ?? 5,
    contrast: initial?.contrast ?? 1.1,
  });

  // Query webcam devices list on mount
  useEffect(() => {
    invoke<{ index: string; name: string }[]>("get_webcam_devices")
      .then(setWebcamDevices)
      .catch((err) => console.error("Failed to load webcams inside preview:", err));
  }, []);

  // Start webcam + handle cleanup properly when active device changes
  useEffect(() => {
    let mounted = true;
    setCameraReady(false);
    
    // Stop existing stream if any
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }

    const devIndex = overlay.device;
    const constraints: MediaStreamConstraints = {
      video: devIndex === "default" ? { width: { ideal: 640 }, height: { ideal: 480 } } : { deviceId: devIndex, width: { ideal: 640 }, height: { ideal: 480 } },
      audio: false,
    };

    navigator.mediaDevices.getUserMedia(constraints)
      .then((s) => {
        if (!mounted) {
          s.getTracks().forEach((t) => t.stop());
          return;
        }
        streamRef.current = s;
        setCameraReady(true);
        if (videoRef.current) {
          videoRef.current.srcObject = s;
        }
      })
      .catch((err) => {
        console.error("Camera access failed inside preview:", err);
        if (mounted) setCameraReady(false);
      });

    return () => {
      mounted = false;
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((t) => t.stop());
        streamRef.current = null;
      }
    };
  }, [overlay.device]);

  // Re-attach stream when video element mounts
  useEffect(() => {
    if (cameraReady && streamRef.current && videoRef.current) {
      videoRef.current.srcObject = streamRef.current;
    }
  }, [cameraReady]);

  const update = <K extends keyof WebcamOverlayConfig>(key: K, value: WebcamOverlayConfig[K]) =>
    setOverlay((prev) => ({ ...prev, [key]: value }));

  // Calculate preview scale to fit in the preview area
  const PREVIEW_MAX_W = 900;
  const PREVIEW_MAX_H = 540;
  const scaleX = PREVIEW_MAX_W / recordingWidth;
  const scaleY = PREVIEW_MAX_H / recordingHeight;
  const scale = Math.min(scaleX, scaleY, 0.95);
  const previewW = recordingWidth * scale;
  const previewH = recordingHeight * scale;

  const shapeStyle = getShapeStyle(overlay.shape);

  // Combine brightness and contrast for CSS filter
  const filterStyle = `brightness(${100 + overlay.brightness}%) contrast(${overlay.contrast})`;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[9999] flex"
      style={{ background: "#0d0f1a", color: "#e1e1f2" }}
    >
      {/* ═══ LEFT SIDEBAR — Controls ═══ */}
      <div
        className="w-80 flex-shrink-0 overflow-y-auto flex flex-col scrollbar-thin"
        style={{
          background: "#151828",
          borderRight: "1px solid #2d314d",
        }}
      >
        {/* Header */}
        <div className="px-5 py-4 flex items-center justify-between border-b border-[#2d314d]">
          <div>
            <div className="font-mono text-xs font-bold uppercase tracking-wider text-[#e1e1f2]">
              WEBCAM OVERLAY PREVIEW
            </div>
            <div className="font-mono mt-0.5 text-[#bacbbc]" style={{ fontSize: "0.6rem" }}>
              Configure overlay geometry & image filters
            </div>
          </div>
          <motion.button
            onClick={onCancel}
            className="p-1 cursor-pointer text-[#bacbbc]"
            whileHover={{ color: "#e1e1f2", scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="close" size={18} />
          </motion.button>
        </div>

        <div className="flex-1 px-5 py-4 space-y-5">
          {/* ── Camera Device ── */}
          <Section label="CAMERA DEVICE">
            <select
              value={overlay.device}
              onChange={(e) => update("device", e.target.value)}
              className="px-3 py-1.5 font-mono text-xs cursor-pointer w-full outline-none border border-[#2d314d] bg-[#090b14] text-[#e1e1f2] rounded focus:border-[#00e88a]"
            >
              <option value="default">Default System Camera</option>
              {webcamDevices.map((d) => (
                <option key={d.index} value={d.index}>{d.name}</option>
              ))}
            </select>
          </Section>

          {/* ── Shape ── */}
          <Section label="SHAPE">
            <div className="grid grid-cols-3 gap-2">
              {SHAPES.map((s) => (
                <motion.button
                  key={s.id}
                  onClick={() => update("shape", s.id)}
                  className="flex flex-col items-center gap-1.5 py-2 px-1 cursor-pointer border rounded"
                  style={{
                    background: overlay.shape === s.id ? "rgba(0, 232, 138, 0.08)" : "#090b14",
                    borderColor: overlay.shape === s.id ? "#00e88a" : "#2d314d",
                    color: overlay.shape === s.id ? "#00e88a" : "#bacbbc",
                  }}
                  whileHover={{ scale: 1.03 }}
                  whileTap={{ scale: 0.97 }}
                >
                  <Icon name={s.icon} size={18} />
                  <span className="font-mono text-[9px] uppercase tracking-wider">{s.label}</span>
                </motion.button>
              ))}
            </div>
          </Section>

          {/* ── Size ── */}
          <Section label="SIZE">
            <div className="grid grid-cols-4 gap-1.5 mb-2.5">
              {SIZE_PRESETS.map((p) => (
                <motion.button
                  key={p.value}
                  onClick={() => update("size", p.value)}
                  className="font-mono text-[10px] py-1 cursor-pointer border rounded"
                  style={{
                    background: overlay.size === p.value ? "#00e88a" : "#090b14",
                    color: overlay.size === p.value ? "#00391e" : "#bacbbc",
                    borderColor: overlay.size === p.value ? "#00e88a" : "#2d314d",
                  }}
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                >
                  {p.label}
                </motion.button>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={80}
                max={400}
                value={overlay.size}
                onChange={(e) => update("size", Number(e.target.value))}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-10 text-right">{overlay.size}px</span>
            </div>
          </Section>

          {/* ── Border ── */}
          <Section label="BORDER STYLE">
            <div className="flex items-center gap-3 mb-2.5">
              <span className="font-mono text-[10px] text-[#bacbbc]">Width</span>
              <input
                type="range"
                min={0}
                max={8}
                value={overlay.borderWidth}
                onChange={(e) => update("borderWidth", Number(e.target.value))}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] w-6 text-right text-[#00e88a]">{overlay.borderWidth}px</span>
            </div>
            <div className="flex gap-2 flex-wrap">
              {BORDER_COLORS.map((c) => (
                <motion.button
                  key={c}
                  onClick={() => update("borderColor", c)}
                  className="w-6 h-6 rounded-full cursor-pointer border border-[#2d314d]"
                  style={{
                    background: c,
                    boxShadow: overlay.borderColor === c ? `0 0 8px ${c}` : "none",
                    borderColor: overlay.borderColor === c ? "#ffffff" : "transparent",
                  }}
                  whileHover={{ scale: 1.15 }}
                  whileTap={{ scale: 0.9 }}
                />
              ))}
            </div>
          </Section>

          {/* ── Opacity ── */}
          <Section label="OPACITY">
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={30}
                max={100}
                value={Math.round(overlay.opacity * 100)}
                onChange={(e) => update("opacity", Number(e.target.value) / 100)}
                className="flex-1 h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
              />
              <span className="font-mono text-[10px] text-[#bacbbc] w-8 text-right">{Math.round(overlay.opacity * 100)}%</span>
            </div>
          </Section>

          {/* ── Image Filters ── */}
          <Section label="IMAGE ADJUSTMENTS (POST-PROCESS)">
            <div className="space-y-3.5 pt-1">
              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[10px] text-[#bacbbc]">
                  <span>Sharpen Strength</span>
                  <span className="text-[#00e88a]">{overlay.sharpen.toFixed(2)}</span>
                </div>
                <input
                  type="range"
                  min={0.0}
                  max={1.0}
                  step={0.05}
                  value={overlay.sharpen}
                  onChange={(e) => update("sharpen", parseFloat(e.target.value))}
                  className="w-full h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
                />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[10px] text-[#bacbbc]">
                  <span>Brightness</span>
                  <span className="text-[#00e88a]">{overlay.brightness > 0 ? `+${overlay.brightness}` : overlay.brightness}</span>
                </div>
                <input
                  type="range"
                  min={-50}
                  max={50}
                  step={1}
                  value={overlay.brightness}
                  onChange={(e) => update("brightness", parseInt(e.target.value))}
                  className="w-full h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
                />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between font-mono text-[10px] text-[#bacbbc]">
                  <span>Contrast</span>
                  <span className="text-[#00e88a]">{overlay.contrast.toFixed(2)}×</span>
                </div>
                <input
                  type="range"
                  min={0.5}
                  max={2.0}
                  step={0.05}
                  value={overlay.contrast}
                  onChange={(e) => update("contrast", parseFloat(e.target.value))}
                  className="w-full h-1.5 cursor-pointer accent-[#00e88a] bg-[#2d314d] rounded-full"
                />
              </div>
            </div>
          </Section>
        </div>

        {/* Bottom buttons */}
        <div className="px-5 py-4 flex gap-3 border-t border-[#2d314d]">
          <motion.button
            onClick={onCancel}
            className="flex-1 py-2 font-mono text-xs uppercase cursor-pointer border border-[#2d314d] rounded text-[#bacbbc]"
            whileHover={{ borderColor: "#bacbbc", color: "#e1e1f2" }}
            whileTap={{ scale: 0.97 }}
          >
            Cancel
          </motion.button>
          <motion.button
            onClick={() => onSave(overlay)}
            className="flex-1 py-2 font-mono text-xs uppercase font-extrabold cursor-pointer rounded text-[#00391e]"
            style={{ background: "#00e88a" }}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.97 }}
          >
            Apply
          </motion.button>
        </div>
      </div>

      {/* ═══ MAIN AREA — Preview Canvas ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center overflow-hidden bg-[#0d0f1a] relative">
        <div className="absolute inset-0 pointer-events-none opacity-5 bg-[linear-gradient(to_right,#808080_1px,transparent_1px),linear-gradient(to_bottom,#808080_1px,transparent_1px)] bg-[size:24px_24px]"></div>
        
        {/* Recording Monitor Aspect-Video simulation */}
        <div
          className="relative shadow-2xl border border-[#2d314d]/50 bg-[#090b14]"
          style={{
            width: previewW,
            height: previewH,
            borderRadius: "8px",
            overflow: "hidden",
          }}
        >
          {/* Mock dashboard silhouette */}
          <div className="absolute inset-0 opacity-10 pointer-events-none p-6 grid grid-cols-12 gap-4">
            <div className="col-span-3 h-32 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-9 h-32 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
            <div className="col-span-4 h-40 bg-[#849587]/30 rounded-lg"></div>
          </div>

          {/* Reference guidelines */}
          <div className="absolute inset-0 pointer-events-none opacity-5 border border-dashed border-[#bacbbc]"></div>

          {/* ═══ RND DRAGGABLE WEBCAM OVERLAY ═══ */}
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
              className="w-full h-full relative group"
              style={{
                ...shapeStyle,
                border: overlay.borderWidth > 0 ? `${overlay.borderWidth}px solid ${overlay.borderColor}` : "none",
                overflow: "hidden",
                boxShadow: `0 0 20px ${overlay.borderColor}40, 0 4px 12px rgba(0,0,0,0.5)`,
              }}
            >
              {cameraReady ? (
                <video
                  ref={videoRef}
                  autoPlay
                  muted
                  playsInline
                  className="w-full h-full object-cover"
                  style={{
                    ...shapeStyle,
                    filter: filterStyle,
                  }}
                />
              ) : (
                <div
                  className="w-full h-full flex flex-col items-center justify-center bg-[#1b1d2e] text-[#bacbbc]"
                  style={{
                    filter: filterStyle,
                  }}
                >
                  <Icon name="person" size={48} style={{ color: overlay.borderColor }} />
                  <span className="font-mono text-[8px] mt-1 tracking-wider">PREVIEW FEED</span>
                </div>
              )}
            </div>
          </Rnd>
        </div>

        {/* Caption */}
        <div className="mt-4 font-mono text-center text-[#bacbbc]" style={{ fontSize: "0.65rem", letterSpacing: "0.05em" }}>
          DRAG OVERLAY TO POSITION · DRAG EDGE CORNER TO RESIZE
        </div>
      </div>
    </motion.div>
  );
}

// ═══ HELPERS ═══

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <div className="font-mono text-[9px] text-[#bacbbc] tracking-widest font-bold uppercase">
        {label}
      </div>
      {children}
    </div>
  );
}

function ResizeHandle() {
  return (
    <div
      className="absolute bottom-0 right-0 w-3.5 h-3.5 cursor-se-resize"
      style={{
        borderRight: "2px solid #00e88a",
        borderBottom: "2px solid #00e88a",
        opacity: 0.8,
      }}
    />
  );
}
