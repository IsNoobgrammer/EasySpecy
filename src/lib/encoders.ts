import type { EncoderScan } from "../stores/recording";

const BASE: { id: string; name: string }[] = [
  { id: "H264", name: "H.264" },
  { id: "H265", name: "H.265" },
  { id: "H264_NVENC", name: "H.264 NVENC" },
  { id: "H265_NVENC", name: "H.265 NVENC" },
  { id: "AV1", name: "AV1" },
  { id: "AV1_NVENC", name: "AV1 NVENC" },
  { id: "VP9", name: "VP9" },
  { id: "MobileShareable", name: "Mobile (H.264)" },
];

/** Saving is a stream copy (~1 s) when the live capture already produces this codec. */
export function isInstant(id: string, scan: EncoderScan | null): boolean {
  if (!scan) return id.startsWith("H264") || id === "MobileShareable";
  if (id.startsWith("H264") || id === "MobileShareable") return scan.live_h264;
  if (id.startsWith("H265")) return scan.live_hevc;
  return false; // AV1 / VP9 always re-encode
}

/** Dropdown entries: only encoders that actually work here (after a scan), annotated. */
export function encoderOptions(scan: EncoderScan | null): { label: string; value: string }[] {
  return BASE.filter(({ id }) => {
    if (!scan) return true;
    const key = id === "MobileShareable" ? "H264" : id;
    return scan.encoders.find((e) => e.id === key)?.supported ?? false;
  }).map(({ id, name }) => {
    const r = scan?.encoders.find((e) => e.id === (id === "MobileShareable" ? "H264" : id));
    const how = isInstant(id, scan) ? "instant save" : r ? `re-encodes · ${Math.round(r.fps)} fps` : "re-encodes";
    const star = scan?.recommended === id ? " ★ recommended" : "";
    return { label: `${name} — ${how}${star}`, value: id };
  });
}

/** NVENC choices only take effect with the GPU flag on (config falls back otherwise). */
export const isGpuEncoder = (id: string) => id.endsWith("_NVENC");
