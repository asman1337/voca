import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { AppConfig, ModelEntry, DownloadProgress } from "../types";
import "./Settings.css";

// ─── Helpers ─────────────────────────────────────────────────────────────────

function fmtBytes(n: number): string {
  if (n === 0) return "0 B";
  if (n < 1_048_576) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1_048_576).toFixed(1)} MB`;
}

function fmtSize(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`;
}

const LANGUAGES = [
  ["en", "English"],   ["fr", "French"],    ["de", "German"],
  ["es", "Spanish"],   ["it", "Italian"],   ["pt", "Portuguese"],
  ["zh", "Chinese"],   ["ja", "Japanese"],  ["ko", "Korean"],
  ["ru", "Russian"],   ["auto", "auto-detect"],
] as const;

// ─── Default fallback config (used in dev when Tauri is not available) ────────

const DEFAULT_CONFIG: AppConfig = {
  model_path:    "models/ggml-base.bin",
  language:      "en",
  hotkey:        "ctrl+shift+v",
  vad_threshold: 0.5,
  input_mode:    "inject",
  auto_launch:   false,
};

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  onClose: () => void;
}

export function Settings({ onClose }: Props) {
  const [config,   setConfig]   = useState<AppConfig>(DEFAULT_CONFIG);
  const [original, setOriginal] = useState<AppConfig>(DEFAULT_CONFIG);
  const [models,   setModels]   = useState<ModelEntry[]>([]);
  const [backend,  setBackend]  = useState("CPU");
  const [dirty,    setDirty]    = useState(false);
  const [saving,   setSaving]   = useState(false);
  const [saveOk,   setSaveOk]   = useState(false);
  const [saveErr,  setSaveErr]  = useState<string | null>(null);

  // Per-model download progress and errors
  const [progress,  setProgress]  = useState<Record<string, DownloadProgress>>({});
  const [dlErrors,  setDlErrors]  = useState<Record<string, string>>({});
  const [activeId,  setActiveId]  = useState<string | null>(null);

  // ── Load initial data ──────────────────────────────────────────────────────
  useEffect(() => {
    Promise.all([
      invoke<AppConfig>("cmd_get_config"),
      invoke<ModelEntry[]>("cmd_list_models"),
      invoke<string>("cmd_get_stt_backend"),
    ]).then(([cfg, mods, be]) => {
      setConfig(cfg);
      setOriginal(cfg);
      setModels(mods);
      setBackend(be);
    }).catch(() => {
      // dev fallback — Tauri not running
    });
  }, []);

  // ── Subscribe to download events ───────────────────────────────────────────
  useEffect(() => {
    const pUnlisten = listen<DownloadProgress>("model_download_progress", ({ payload }) => {
      setProgress(prev => ({ ...prev, [payload.model_id]: payload }));
    });

    const dUnlisten = listen<{ model_id: string; path: string }>("model_download_done", ({ payload }) => {
      setActiveId(null);
      setProgress(prev => { const n = { ...prev }; delete n[payload.model_id]; return n; });
      invoke<ModelEntry[]>("cmd_list_models").then(setModels).catch(() => {});
    });

    const eUnlisten = listen<{ model_id: string; error: string }>("model_download_error", ({ payload }) => {
      setActiveId(null);
      setProgress(prev => { const n = { ...prev }; delete n[payload.model_id]; return n; });
      setDlErrors(prev => ({ ...prev, [payload.model_id]: payload.error }));
    });

    return () => {
      pUnlisten.then(f => f());
      dUnlisten.then(f => f());
      eUnlisten.then(f => f());
    };
  }, []);

  // ── Config helpers ─────────────────────────────────────────────────────────
  const patch = useCallback((delta: Partial<AppConfig>) => {
    setConfig(prev => ({ ...prev, ...delta }));
    setDirty(true);
    setSaveOk(false);
    setSaveErr(null);
  }, []);

  // ── Save ───────────────────────────────────────────────────────────────────
  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveErr(null);
    try {
      await invoke("cmd_save_config", { newCfg: config });
      // Sync auto-launch to OS separately (save_config persists TOML only).
      await invoke("cmd_set_auto_launch", { enable: config.auto_launch });
      setOriginal(config);
      setDirty(false);
      setSaveOk(true);
    } catch (e) {
      setSaveErr(String(e));
    } finally {
      setSaving(false);
    }
  }, [config]);

  const handleReset = useCallback(() => {
    setConfig(original);
    setDirty(false);
    setSaveOk(false);
    setSaveErr(null);
  }, [original]);

  // ── Model actions ──────────────────────────────────────────────────────────
  const handleDownload = useCallback((modelId: string) => {
    if (activeId) invoke("cmd_cancel_download").catch(() => {});
    setActiveId(modelId);
    setDlErrors(prev => { const n = { ...prev }; delete n[modelId]; return n; });
    invoke("cmd_download_model", { modelId }).catch(() => {});
  }, [activeId]);

  const handleCancelDownload = useCallback(() => {
    invoke("cmd_cancel_download").catch(() => {});
    setActiveId(null);
  }, []);

  const handleDelete = useCallback(async (modelId: string) => {
    try {
      await invoke("cmd_delete_model", { modelId });
      const updated = await invoke<ModelEntry[]>("cmd_list_models");
      setModels(updated);
    } catch (e) {
      setSaveErr(String(e));
    }
  }, []);

  const handleSelectModel = useCallback((m: ModelEntry) => {
    if (!m.downloaded || !m.path) return;
    patch({ model_path: m.path });
  }, [patch]);

  // ── Drag region for window ─────────────────────────────────────────────────
  const handleHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const startX = e.clientX, startY = e.clientY;
    const cleanup = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    function onMove(me: MouseEvent) {
      if (Math.abs(me.clientX - startX) + Math.abs(me.clientY - startY) < 5) return;
      cleanup();
      getCurrentWebviewWindow().startDragging().catch(() => {});
    }
    function onUp() { cleanup(); }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }, []);

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="s-root">

      {/* ── HEADER ────────────────────────────────────────────────────────── */}
      <div className="s-header" onMouseDown={handleHeaderMouseDown}>
        <div className="s-header-left">
          <span className="s-logo-dot" />
          <span className="s-title">voca</span>
          <span className="s-title-sep">·</span>
          <span className="s-title-sub">settings</span>
        </div>
        <button className="s-close" onClick={onClose} title="Close">
          <span>×</span>
        </button>
      </div>

      {/* ── BODY ──────────────────────────────────────────────────────────── */}
      <div className="s-body">

        {/* ── MODEL SECTION ──────────────────────────────────────────────── */}
        <section className="s-section">
          <div className="s-section-label">
            <span className="s-comment">#</span> model
            <span className="s-badge s-badge--backend">{backend}</span>
          </div>
          <div className="s-model-list">
            {models.map(m => {
              const prog      = progress[m.id];
              const dlErr     = dlErrors[m.id];
              const isLoading = activeId === m.id;
              // A model is "active" if its path matches or the id is in the stored path.
              const isActive  = m.downloaded && (
                config.model_path === m.path ||
                config.model_path.includes(`ggml-${m.id}.bin`)
              );

              return (
                <div
                  key={m.id}
                  className={[
                    "s-model",
                    m.downloaded  ? "s-model--ready"  : "",
                    isActive      ? "s-model--active"  : "",
                    isLoading     ? "s-model--loading" : "",
                  ].join(" ")}
                  onClick={() => handleSelectModel(m)}
                  role={m.downloaded ? "button" : undefined}
                >
                  <div className="s-model-info">
                    <div className="s-model-name">
                      {isActive && <span className="s-model-active-pip" />}
                      {m.name}
                    </div>
                    <div className="s-model-desc">{m.description}</div>
                    {dlErr && <div className="s-model-err">{dlErr}</div>}

                    {/* Download progress bar */}
                    {isLoading && prog && (
                      <div className="s-progress">
                        <div className="s-progress-track">
                          <div
                            className="s-progress-fill"
                            style={{ width: `${prog.percent.toFixed(1)}%` }}
                          />
                        </div>
                        <span className="s-progress-label">
                          {fmtBytes(prog.bytes_done)}
                          {prog.bytes_total > 0 && ` / ${fmtBytes(prog.bytes_total)}`}
                          <span className="s-progress-pct">{prog.percent.toFixed(0)}%</span>
                        </span>
                      </div>
                    )}
                  </div>

                  <div className="s-model-actions">
                    <span className="s-model-size">{fmtSize(m.size_mb)}</span>
                    {m.downloaded ? (
                      <button
                        className="s-model-btn s-model-btn--del"
                        onClick={e => { e.stopPropagation(); handleDelete(m.id); }}
                        title="Delete model"
                      >⊗</button>
                    ) : isLoading ? (
                      <button
                        className="s-model-btn s-model-btn--stop"
                        onClick={e => { e.stopPropagation(); handleCancelDownload(); }}
                      >■</button>
                    ) : (
                      <button
                        className="s-model-btn s-model-btn--dl"
                        onClick={e => { e.stopPropagation(); handleDownload(m.id); }}
                        title="Download"
                      >↓</button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </section>

        {/* ── LANGUAGE ───────────────────────────────────────────────────── */}
        <section className="s-section">
          <div className="s-section-label">
            <span className="s-comment">#</span> language
          </div>
          <select
            className="s-select"
            value={config.language}
            onChange={e => patch({ language: e.target.value })}
          >
            {LANGUAGES.map(([code, label]) => (
              <option key={code} value={code}>{code} — {label}</option>
            ))}
          </select>
        </section>

        {/* ── HOTKEY ─────────────────────────────────────────────────────── */}
        <section className="s-section">
          <div className="s-section-label">
            <span className="s-comment">#</span> hotkey
            <span className="s-muted">(live rebinding in next release)</span>
          </div>
          <div className="s-hotkey-display">
            {config.hotkey.split("+").map((k, i) => (
              <span key={i} className="s-key">{k}</span>
            ))}
          </div>
        </section>

        {/* ── INPUT MODE ─────────────────────────────────────────────────── */}
        <section className="s-section">
          <div className="s-section-label">
            <span className="s-comment">#</span> input mode
          </div>
          <div className="s-radio-group">
            {(["inject", "clipboard"] as const).map(mode => (
              <label
                key={mode}
                className={`s-radio-pill ${config.input_mode === mode ? "s-radio-pill--on" : ""}`}
              >
                <input
                  type="radio"
                  name="input_mode"
                  value={mode}
                  checked={config.input_mode === mode}
                  onChange={() => patch({ input_mode: mode })}
                />
                {mode}
              </label>
            ))}
          </div>
        </section>

        {/* ── VAD THRESHOLD ──────────────────────────────────────────────── */}
        <section className="s-section s-section--row">
          <div className="s-section-label">
            <span className="s-comment">#</span> vad sensitivity
          </div>
          <div className="s-slider-wrap">
            <input
              type="range"
              className="s-slider"
              min={0} max={1} step={0.05}
              value={config.vad_threshold}
              onChange={e => patch({ vad_threshold: parseFloat(e.target.value) })}
            />
            <span className="s-slider-val">{config.vad_threshold.toFixed(2)}</span>
          </div>
        </section>

        {/* ── AUTO LAUNCH ────────────────────────────────────────────────── */}
        <section className="s-section s-section--row">
          <div className="s-section-label">
            <span className="s-comment">#</span> launch on login
          </div>
          <button
            className={`s-toggle ${config.auto_launch ? "s-toggle--on" : ""}`}
            onClick={() => patch({ auto_launch: !config.auto_launch })}
            role="switch"
            aria-checked={config.auto_launch}
          >
            <span className="s-toggle-knob" />
          </button>
        </section>

      </div>{/* end s-body */}

      {/* ── FOOTER ────────────────────────────────────────────────────────── */}
      <div className="s-footer">
        {saveErr && <div className="s-footer-err">{saveErr}</div>}
        <div className="s-footer-row">
          <button
            className={[
              "s-btn-save",
              saving  ? "s-btn-save--saving" : "",
              saveOk  ? "s-btn-save--ok"     : "",
              !dirty  ? "s-btn-save--dim"    : "",
            ].join(" ")}
            onClick={handleSave}
            disabled={saving || !dirty}
          >
            {saving ? "saving…" : saveOk ? "saved ✓" : "save changes"}
          </button>
          <button
            className="s-btn-ghost"
            onClick={handleReset}
            disabled={!dirty}
          >
            reset
          </button>
          <button className="s-btn-ghost s-btn-ghost--close" onClick={onClose}>
            close
          </button>
        </div>
      </div>

    </div>
  );
}
