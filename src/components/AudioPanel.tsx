import { useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useStore } from "../stores/recording";
import { Icon } from "./Icon";

/** What audio will be recorded — two clear toggles with live levels, so "is my mic on?"
 *  is answered before recording, not after. */
export function AudioPanel({ disabled }: { disabled?: boolean }) {
  const { config, saveConfig, startAudioMonitor, stopAudioMonitor } = useStore(useShallow((s) => ({
    config: s.config, saveConfig: s.saveConfig, startAudioMonitor: s.startAudioMonitor, stopAudioMonitor: s.stopAudioMonitor,
  })));
  if (!config) return null;
  const src = config.audio_source;
  const micOn = config.audio_enabled && (src === "Mic" || src === "Both");
  const sysOn = config.audio_enabled && (src === "System" || src === "Both");

  const setSources = async (mic: boolean, sys: boolean) => {
    const next = { ...config, audio_enabled: mic || sys, audio_source: (mic && sys ? "Both" : mic ? "Mic" : sys ? "System" : src) as typeof src };
    await saveConfig(next);
    await stopAudioMonitor();
    if (next.audio_enabled) await startAudioMonitor(); // meters follow the new sources
  };

  return (
    <div className="w-full max-w-xl grid grid-cols-2 gap-3" role="group" aria-label="Audio sources">
      <SourceRow kind="mic" icon="mic" label="Microphone" on={micOn} disabled={disabled}
        onToggle={() => setSources(!micOn, sysOn)}
        silentHint="No sound from your mic — is it muted or the wrong device?" />
      <SourceRow kind="sys" icon="volume_up" label="System audio" on={sysOn} disabled={disabled}
        onToggle={() => setSources(micOn, !sysOn)}
        silentHint="Nothing playing — only sound your PC plays gets recorded" />
    </div>
  );
}

function SourceRow({ kind, icon, label, on, disabled, onToggle, silentHint }: {
  kind: "mic" | "sys"; icon: string; label: string; on: boolean; disabled?: boolean; onToggle: () => void; silentHint: string;
}) {
  const db = useStore((s) => (kind === "mic" ? s.audioLevels.micDb : s.audioLevels.sysDb));
  const frac = on ? Math.max(0, Math.min(1, (db + 60) / 60)) : 0;

  // "Silent" = no level above -50 dB for 3 s while this source is on
  const lastSound = useRef(Date.now());
  if (db > -50) lastSound.current = Date.now();
  const [silent, setSilent] = useState(false);
  useEffect(() => {
    if (!on) { setSilent(false); return; }
    lastSound.current = Date.now();
    const id = setInterval(() => setSilent(Date.now() - lastSound.current > 3000), 1000);
    return () => clearInterval(id);
  }, [on]);

  const accent = kind === "mic" ? "var(--accent-primary)" : "var(--accent-info)";
  return (
    <button
      type="button"
      onClick={onToggle}
      disabled={disabled}
      aria-pressed={on}
      className="text-left px-4 py-3 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 transition-colors focus-visible:outline-2 focus-visible:outline-offset-2"
      style={{
        border: `var(--border-width) solid ${on ? accent : "var(--border-default)"}`,
        borderRadius: "var(--radius-md)",
        background: "var(--bg-surface)",
      }}
    >
      <div className="flex items-center gap-2">
        <Icon name={icon} size={16} style={{ color: on ? accent : "var(--text-muted)" }} />
        <span className="font-mono text-[11px] font-bold uppercase" style={{ color: on ? "var(--text-primary)" : "var(--text-muted)", letterSpacing: "0.05em" }}>
          {label}
        </span>
        <span className="ml-auto font-mono text-[10px] font-bold" style={{ color: on ? accent : "var(--text-muted)" }}>
          {on ? "ON" : "OFF"}
        </span>
      </div>
      <div className="h-1.5 w-full rounded-full overflow-hidden mt-2" style={{ background: "var(--bg-elevated)" }} aria-hidden>
        <div className="h-full w-full origin-left transition-transform duration-75" style={{ transform: `scaleX(${frac})`, background: accent }} />
      </div>
      <div className="font-mono text-[10px] mt-1.5 min-h-[14px]" style={{ color: on && silent ? "var(--accent-warning)" : "var(--text-muted)" }}>
        {!on ? "Not recorded — tap to turn on" : silent ? silentHint : "Recording-ready"}
      </div>
    </button>
  );
}
