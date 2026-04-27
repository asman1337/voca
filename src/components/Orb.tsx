import { useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { OrbState } from "../types";
import "./Orb.css";

// ─── Canvas Orb Renderer ──────────────────────────────────────────────────────
const PI = Math.PI;
const TAU = PI * 2;

class OrbRenderer {
  private ctx: CanvasRenderingContext2D;
  private t = 0;
  private alive = true;
  private stateRef: { current: OrbState };
  private readonly W = 80;
  private readonly H = 80;
  private readonly cx = 40;
  private readonly cy = 40;

  constructor(canvas: HTMLCanvasElement, stateRef: { current: OrbState }) {
    const dpr = window.devicePixelRatio || 1;
    canvas.width = this.W * dpr;
    canvas.height = this.H * dpr;
    canvas.style.width = `${this.W}px`;
    canvas.style.height = `${this.H}px`;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 2d context unavailable");
    this.ctx = ctx;
    this.ctx.scale(dpr, dpr);
    this.stateRef = stateRef;
    this.loop();
  }

  destroy() { this.alive = false; }

  private loop() {
    if (!this.alive) return;
    this.draw();
    requestAnimationFrame(() => this.loop());
  }

  private draw() {
    this.ctx.clearRect(0, 0, this.W, this.H);
    switch (this.stateRef.current) {
      case "idle":         this.drawIdle();       break;
      case "listening":    this.drawListen();     break;
      case "transcribing": this.drawTranscribe(); break;
      case "injected":     this.drawDone();       break;
      case "muted":        this.drawMuted();      break;
    }
    this.t += 0.016;
  }

  // ─── IDLE ────────────────────────────────────────────────────────────────
  private drawIdle() {
    const { ctx, cx, cy, t } = this;
    const R = 34;
    const bg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    bg.addColorStop(0, "#111124");
    bg.addColorStop(0.7, "#0a0a16");
    bg.addColorStop(1, "#06060e");
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.fillStyle = bg; ctx.fill();
    // faint orbiting dashed ring
    ctx.save(); ctx.translate(cx, cy); ctx.rotate(t * 0.3);
    ctx.strokeStyle = "rgba(139,92,246,.12)"; ctx.lineWidth = 1;
    ctx.setLineDash([3, 8]);
    ctx.beginPath(); ctx.arc(0, 0, R - 5, 0, TAU); ctx.stroke();
    ctx.setLineDash([]); ctx.restore();
    // outer rim
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.strokeStyle = "rgba(139,92,246,.18)"; ctx.lineWidth = 1; ctx.stroke();
    // sleeping nucleus breathe
    const breath = 0.5 + 0.5 * Math.sin(t * 0.5);
    const nr = 8 + breath * 1.5;
    const nc = ctx.createRadialGradient(cx, cy, 0, cx, cy, nr);
    nc.addColorStop(0, `rgba(167,139,250,${(0.25 + breath * 0.15).toFixed(2)})`);
    nc.addColorStop(1, "rgba(139,92,246,0)");
    ctx.beginPath(); ctx.arc(cx, cy, nr, 0, TAU);
    ctx.fillStyle = nc; ctx.fill();
    // tiny core dot
    ctx.beginPath(); ctx.arc(cx, cy, 2.7, 0, TAU);
    ctx.fillStyle = "rgba(167,139,250,.35)"; ctx.fill();
  }

  // ─── LISTEN ──────────────────────────────────────────────────────────────
  private drawListen() {
    const { ctx, cx, cy, t } = this;
    const R = 34;
    const bg = ctx.createRadialGradient(cx, cy - 3, 0, cx, cy, R);
    bg.addColorStop(0, "#1a0f3a");
    bg.addColorStop(0.5, "#0e0828");
    bg.addColorStop(1, "#07060f");
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.fillStyle = bg; ctx.fill();
    // breathing halo rings
    for (let i = 0; i < 3; i++) {
      const phase = t * 1.4 + i * (TAU / 3);
      const scale = 0.5 + 0.5 * Math.sin(phase);
      const alpha = 0.08 + 0.18 * scale;
      const rad = R - 2 + (i + 1) * 8 * scale;
      ctx.beginPath(); ctx.arc(cx, cy, rad, 0, TAU);
      ctx.strokeStyle = `rgba(139,92,246,${alpha.toFixed(2)})`; ctx.lineWidth = 1 + scale; ctx.stroke();
    }
    // active outer rim
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.strokeStyle = "rgba(167,139,250,.55)"; ctx.lineWidth = 1.5; ctx.stroke();
    // rotating arc sweep
    ctx.save(); ctx.translate(cx, cy); ctx.rotate(t * 2.5);
    ctx.beginPath(); ctx.arc(0, 0, R + 2.5, 0, PI * 0.6);
    ctx.strokeStyle = "rgba(139,92,246,.7)"; ctx.lineWidth = 1.5;
    ctx.lineCap = "round"; ctx.stroke(); ctx.restore();
    // waveform bars
    const bars = 7, bw = 2.5, gap = 3.5;
    const totalW = bars * bw + (bars - 1) * gap;
    const bx = cx - totalW / 2;
    for (let i = 0; i < bars; i++) {
      const phase = t * 4 + i * 0.55;
      const h = 5 + 14 * Math.abs(Math.sin(phase));
      const x = bx + i * (bw + gap);
      const alpha = 0.5 + 0.5 * Math.sin(phase + PI / 2);
      const barG = ctx.createLinearGradient(x, cy - h / 2, x, cy + h / 2);
      barG.addColorStop(0, `rgba(196,181,253,${alpha.toFixed(2)})`);
      barG.addColorStop(1, `rgba(139,92,246,${(alpha * 0.4).toFixed(2)})`);
      ctx.fillStyle = barG;
      ctx.beginPath();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (ctx as any).roundRect(x, cy - h / 2, bw, h, 1.2);
      ctx.fill();
    }
  }

  // ─── TRANSCRIBE ──────────────────────────────────────────────────────────
  private drawTranscribe() {
    const { ctx, cx, cy, t } = this;
    const R = 34;
    const bg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    bg.addColorStop(0, "#071e2a");
    bg.addColorStop(0.6, "#040f18");
    bg.addColorStop(1, "#020810");
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.fillStyle = bg; ctx.fill();
    // two counter-rotating arcs
    for (let i = 0; i < 2; i++) {
      const dir = i === 0 ? 1 : -1;
      ctx.save(); ctx.translate(cx, cy);
      ctx.rotate(t * 2.2 * dir + i * PI * 0.4);
      ctx.beginPath(); ctx.arc(0, 0, R - 2.5 - i * 7, -PI * 0.1, PI * (1 + i * 0.3));
      ctx.strokeStyle = i === 0 ? "rgba(34,211,238,.8)" : "rgba(103,232,249,.4)";
      ctx.lineWidth = i === 0 ? 2 : 1;
      ctx.lineCap = "round"; ctx.stroke(); ctx.restore();
    }
    // orbiting data nodes
    for (let i = 0; i < 6; i++) {
      const angle = (i / 6) * TAU + t * 1.8;
      const or = R - 14;
      const dx = cx + or * Math.cos(angle);
      const dy = cy + or * Math.sin(angle);
      const phase = Math.sin(t * 3 + i * 1.2);
      ctx.beginPath(); ctx.arc(dx, dy, 2 + Math.abs(phase), 0, TAU);
      ctx.fillStyle = `rgba(34,211,238,${(0.4 + 0.6 * Math.abs(phase)).toFixed(2)})`;
      ctx.fill();
    }
    // center scan pulse
    const sp = (t * 1.8) % 1;
    ctx.beginPath(); ctx.arc(cx, cy, sp * 17, 0, TAU);
    ctx.strokeStyle = `rgba(34,211,238,${(0.5 * (1 - sp)).toFixed(2)})`; ctx.lineWidth = 0.8; ctx.stroke();
    // tiny core
    ctx.beginPath(); ctx.arc(cx, cy, 3, 0, TAU);
    ctx.fillStyle = "rgba(103,232,249,.7)"; ctx.fill();
  }

  // ─── DONE / INJECTED ─────────────────────────────────────────────────────
  private drawDone() {
    const { ctx, cx, cy, t } = this;
    const R = 34;
    const bg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    bg.addColorStop(0, "#051a0f");
    bg.addColorStop(0.65, "#03100a");
    bg.addColorStop(1, "#020807");
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.fillStyle = bg; ctx.fill();
    // expanding completion rings
    for (let i = 0; i < 3; i++) {
      const phase = (t * 0.6 + i * 0.33) % 1;
      ctx.beginPath(); ctx.arc(cx, cy, 12 + phase * 25, 0, TAU);
      ctx.strokeStyle = `rgba(74,222,128,${(0.15 * (1 - phase)).toFixed(2)})`;
      ctx.lineWidth = 1; ctx.stroke();
    }
    // outer rim
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.strokeStyle = "rgba(74,222,128,.45)"; ctx.lineWidth = 1.5; ctx.stroke();
    ctx.save(); ctx.translate(cx, cy);
    // radial glow
    const glow = ctx.createRadialGradient(0, -1.5, 0, 0, -1.5, 17);
    glow.addColorStop(0, "rgba(74,222,128,.15)");
    glow.addColorStop(1, "rgba(74,222,128,0)");
    ctx.beginPath(); ctx.arc(0, 0, 17, 0, TAU);
    ctx.fillStyle = glow; ctx.fill();
    // checkmark
    ctx.strokeStyle = "rgba(134,239,172,.9)";
    ctx.lineWidth = 2.5; ctx.lineCap = "round"; ctx.lineJoin = "round";
    ctx.beginPath(); ctx.moveTo(-9, 0); ctx.lineTo(-3, 7); ctx.lineTo(10, -8);
    ctx.stroke(); ctx.restore();
  }

  // ─── MUTED ───────────────────────────────────────────────────────────────
  private drawMuted() {
    const { ctx, cx, cy, t } = this;
    const R = 34;
    const bg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    bg.addColorStop(0, "#1a0810");
    bg.addColorStop(0.6, "#0e0509");
    bg.addColorStop(1, "#080306");
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.fillStyle = bg; ctx.fill();
    // ember glow
    const ember = 0.5 + 0.5 * Math.sin(t * 0.7);
    const eg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R * 0.7);
    eg.addColorStop(0, `rgba(251,113,133,${(0.06 + ember * 0.06).toFixed(2)})`);
    eg.addColorStop(1, "rgba(251,113,133,0)");
    ctx.beginPath(); ctx.arc(cx, cy, R * 0.7, 0, TAU);
    ctx.fillStyle = eg; ctx.fill();
    // dashed rim
    ctx.setLineDash([2.5, 6]);
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, TAU);
    ctx.strokeStyle = "rgba(251,113,133,.25)"; ctx.lineWidth = 1; ctx.stroke();
    ctx.setLineDash([]);
    ctx.save(); ctx.translate(cx, cy);
    // mic body
    ctx.beginPath();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (ctx as any).roundRect(-4, -11, 8, 14, 4);
    ctx.strokeStyle = "rgba(251,113,133,.4)"; ctx.lineWidth = 1.5; ctx.stroke();
    // mic stand vertical
    ctx.beginPath(); ctx.moveTo(0, 3); ctx.lineTo(0, 8.5);
    ctx.strokeStyle = "rgba(251,113,133,.4)"; ctx.lineWidth = 1.5; ctx.stroke();
    // mic stand arch
    ctx.beginPath(); ctx.arc(0, 6.5, 4.5, PI, 0);
    ctx.strokeStyle = "rgba(251,113,133,.4)"; ctx.lineWidth = 1.5; ctx.stroke();
    // diagonal slash
    ctx.strokeStyle = "rgba(251,113,133,.85)"; ctx.lineWidth = 2.5; ctx.lineCap = "round";
    ctx.beginPath(); ctx.moveTo(-11.5, -11.5); ctx.lineTo(11.5, 11.5); ctx.stroke();
    ctx.restore();
  }
}

interface OrbProps {
  state: OrbState;
  clipboardText: string | null;
  onDismissCard: () => void;
  onOpenSettings: () => void;
}

export function Orb({ state, clipboardText, onDismissCard, onOpenSettings }: OrbProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const stateRef = useRef<OrbState>(state);
  stateRef.current = state;

  // ── Canvas renderer lifecycle ──────────────────────────────────────────────
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const renderer = new OrbRenderer(canvas, stateRef);
    return () => renderer.destroy();
  }, []);

  // Window resize is now managed in App.tsx.
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

  // ── Right-click → open settings ─────────────────────────────────────────
  const handleOrbContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    onOpenSettings();
  }, [onOpenSettings]);

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
      <canvas
        ref={canvasRef}
        className="orb-canvas"
        onClick={handleOrbClick}
        onContextMenu={handleOrbContextMenu}
        title={ORB_TOOLTIPS[state]}
      />

      {/* Clipboard fallback card — shown when injected but no active input */}
      {state === "injected" && clipboardText && (
        <div className="orb-card" onClick={(e) => e.stopPropagation()}>
          <div className="orb-card-header">
            <div className="card-state-dot" />
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
