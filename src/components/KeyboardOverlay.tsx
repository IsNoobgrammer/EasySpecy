import { useEffect, useRef } from "react";
import { useStore } from "../stores/recording";

/**
 * KeyboardOverlay — renders recent keypresses as a floating overlay.
 * Shows keys pressed in the last 5 seconds with fade-out animation.
 * Positioned at bottom-center of the recording view.
 */
export function KeyboardOverlay() {
  const recordingPhase = useStore((s) => s.recordingPhase);
  const keyboardEvents = useStore((s) => s.keyboardEvents);
  const pollKeyboardEvents = useStore((s) => s.pollKeyboardEvents);
  const containerRef = useRef<HTMLDivElement>(null);

  // Poll keyboard events at ~20Hz during recording
  useEffect(() => {
    if (recordingPhase !== "recording") return;
    const interval = setInterval(pollKeyboardEvents, 50);
    return () => clearInterval(interval);
  }, [recordingPhase, pollKeyboardEvents]);

  if (recordingPhase !== "recording" || keyboardEvents.length === 0) return null;

  // Group consecutive identical keys and deduplicate within 200ms windows
  const now = Date.now();
  const displayEvents = keyboardEvents
    .filter((e) => now - e.timestamp_ms < 5000) // Only show last 5 seconds
    .slice(-12); // Max 12 keys visible

  if (displayEvents.length === 0) return null;

  return (
    <div
      ref={containerRef}
      className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-1.5 pointer-events-none z-50"
    >
      {displayEvents.map((event, i) => {
        const age = now - event.timestamp_ms;
        const opacity = age > 4000 ? Math.max(0, 1 - (age - 4000) / 1000) : 1;

        return (
          <div
            key={`${event.timestamp_ms}-${i}`}
            className="px-2 py-1 font-mono text-[11px] font-bold"
            style={{
              background: "rgba(0, 0, 0, 0.75)",
              color: "#fff",
              borderRadius: "4px",
              border: "1px solid rgba(255, 255, 255, 0.15)",
              opacity,
              backdropFilter: "blur(4px)",
              transition: "opacity 0.3s ease-out",
              minWidth: 24,
              textAlign: "center",
              whiteSpace: "nowrap",
            }}
          >
            {formatKey(event.key)}
          </div>
        );
      })}
    </div>
  );
}

/**
 * Format key name for display — capitalize single letters, show special keys nicely
 */
function formatKey(key: string): string {
  // Single letter keys
  if (key.length === 1) return key.toUpperCase();

  // Arrow keys
  if (key === "←") return "←";
  if (key === "↑") return "↑";
  if (key === "→") return "→";
  if (key === "↓") return "↓";

  // Common modifiers — abbreviate
  const abbreviations: Record<string, string> = {
    Ctrl: "⌃",
    Shift: "⇧",
    Alt: "⌥",
    Win: "⊞",
    LCtrl: "⌃",
    RCtrl: "⌃",
    LShift: "⇧",
    RShift: "⇧",
    LAlt: "⌥",
    RAlt: "⌥",
    Enter: "↵",
    Backspace: "⌫",
    Delete: "⌦",
    Tab: "⇥",
    CapsLock: "⇪",
    Space: "␣",
    Esc: "⎋",
    PrintScreen: "⎙",
    Insert: "Ins",
    PageUp: "PgUp",
    PageDown: "PgDn",
    NumLock: "NumLk",
    ScrollLock: "ScrLk",
  };

  return abbreviations[key] || key;
}
