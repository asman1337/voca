import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { OrbState } from "../types";
import "./Orb.css";

interface OrbProps {
  state: OrbState;
  clipboardText: string | null;
  onDismissCard: () => void;
}

export function Orb({ state, clipboardText, onDismissCard }: OrbProps) {
  // ── Dragging ────────────────────────────────────────────────────────────
  // We wait for actual mouse movement before calling startDragging() so
  // that a stationary click still fires onClick on the orb circle.
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      const target = e.target as HTMLElement;
      if (target.closest(".orb-card")) return;
      // Don't call preventDefault here — that would swallow the click event.
      const startX = e.clientX;
      const startY = e.clientY;

      const cleanup = () => {
        window.removeEventListener("mousemove", onMove);
        window.removeEventListener("mouseup", onUp);
      };

      // Only initiate a native drag after the mouse moves ≥5 px.
      function onMove(me: MouseEvent) {
        if (Math.abs(me.clientX - startX) + Math.abs(me.clientY - startY) < 5)
          return;
        cleanup();
        getCurrentWebviewWindow().startDragging().catch(() => {});
      }

      function onUp() {
        cleanup();
      }

      window.addEventListener("mousemove", onMove);
      window.addEventListener("mouseup", onUp);
    },
    []
  );

  // ── Click to toggle ──────────────────────────────────────────────────────
  const handleOrbClick = useCallback(async () => {
    try {
      await invoke("cmd_toggle_listening");
    } catch {
      // Backend not yet connected in dev
    }
  }, []);

  // ── Copy to clipboard ────────────────────────────────────────────────────
  const handleCopy = useCallback(async () => {
    if (!clipboardText) return;
    try {
      await navigator.clipboard.writeText(clipboardText);
    } catch {
      // Silently fail if clipboard API is unavailable
    }
    onDismissCard();
  }, [clipboardText, onDismissCard]);

  // ── Re-listen ────────────────────────────────────────────────────────────
  const handleRelisten = useCallback(async () => {
    onDismissCard();
    try {
      await invoke("cmd_start_listening");
    } catch {
      // pass
    }
  }, [onDismissCard]);

  return (
    <div className="orb-root" onMouseDown={handleMouseDown}>
      {/* The orb circle */}
      <div
        className={`orb orb--${state}`}
        onClick={handleOrbClick}
        title={ORB_TOOLTIPS[state]}
      >
        {state === "idle" && <div className="orb-dot" />}
        {state === "listening" && (
          <div className="waveform">
            {[6, 14, 22, 10, 20, 12, 6].map((h, i) => (
              <div key={i} className="bar" style={{ height: h }} />
            ))}
          </div>
        )}
        {state === "transcribing" && <div className="spinner" />}
        {state === "injected" && <div className="orb-dot" />}
        {state === "muted" && <div className="mute-icon" />}
      </div>

      {/* Clipboard fallback card — shown when injected but no active input */}
      {state === "injected" && clipboardText && (
        <div className="orb-card" onClick={(e) => e.stopPropagation()}>
          <div className="orb-card-header">
            <div className="orb orb--injected" style={{ width: 22, height: 22 }}>
              <div className="orb-dot" style={{ width: 8, height: 8 }} />
            </div>
            <span className="orb-card-label">VOCA · ready to paste</span>
          </div>
          <div className="orb-card-text">"{clipboardText}"</div>
          <div className="orb-card-actions">
            <button className="card-btn card-btn--primary" onClick={handleCopy}>
              Copy text
            </button>
            <button className="card-btn" onClick={onDismissCard}>
              Discard
            </button>
            <button className="card-btn" onClick={handleRelisten}>
              Re-listen
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

const ORB_TOOLTIPS: Record<OrbState, string> = {
  idle: "Click to start listening",
  listening: "Click to stop",
  transcribing: "Transcribing…",
  injected: "Text injected",
  muted: "Muted — click to unmute",
};
