import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HistoryEntry } from "../types";
import "./History.css";

interface Props {
  onClose: () => void;
}

const ITEMS_PER_PAGE = 50;

export default function History({ onClose }: Props) {
  const [entries, setEntries]     = useState<HistoryEntry[]>([]);
  const [query, setQuery]         = useState("");
  const [loading, setLoading]     = useState(false);
  const [clearing, setClearing]   = useState(false);
  const [page, setPage]           = useState(0);
  const [copiedId, setCopiedId]   = useState<number | null>(null);
  const [error, setError]         = useState<string | null>(null);

  // ── Load / search ─────────────────────────────────────────────────────────
  const load = useCallback(async (q: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = q.trim()
        ? await invoke<HistoryEntry[]>("cmd_search_history", { query: q.trim() })
        : await invoke<HistoryEntry[]>("cmd_get_history");
      setEntries(result);
      setPage(0);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(""); }, [load]);

  // Debounced search on query change
  useEffect(() => {
    const t = setTimeout(() => load(query), 250);
    return () => clearTimeout(t);
  }, [query, load]);

  // ── Copy ──────────────────────────────────────────────────────────────────
  const handleCopy = async (entry: HistoryEntry) => {
    try {
      await navigator.clipboard.writeText(entry.text);
      setCopiedId(entry.id);
      setTimeout(() => setCopiedId(null), 1500);
    } catch {
      // fallback — silently ignore
    }
  };

  // ── Delete ────────────────────────────────────────────────────────────────
  const handleDelete = async (id: number) => {
    try {
      await invoke("cmd_delete_history_entry", { id });
      setEntries(prev => prev.filter(e => e.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  // ── Clear all ─────────────────────────────────────────────────────────────
  const handleClearAll = async () => {
    if (!window.confirm("Delete all transcript history? This cannot be undone.")) return;
    setClearing(true);
    try {
      await invoke("cmd_clear_history");
      setEntries([]);
      setPage(0);
    } catch (e) {
      setError(String(e));
    } finally {
      setClearing(false);
    }
  };

  // ── Pagination ────────────────────────────────────────────────────────────
  const totalPages = Math.max(1, Math.ceil(entries.length / ITEMS_PER_PAGE));
  const visible    = entries.slice(page * ITEMS_PER_PAGE, (page + 1) * ITEMS_PER_PAGE);

  // ── Formatting ────────────────────────────────────────────────────────────
  const fmtDate = (ts: number) => {
    const d = new Date(ts * 1000);
    return d.toLocaleString(undefined, {
      month: "short", day: "numeric",
      hour:  "2-digit", minute: "2-digit",
    });
  };

  return (
    <div className="h-root" data-tauri-drag-region>

      {/* ── Header ─────────────────────────────────────────────────────── */}
      <div className="h-header">
        <span className="h-title">
          <span className="h-title-glyph">◈</span> transcript history
        </span>
        <button className="h-close" onClick={onClose} aria-label="close">×</button>
      </div>

      {/* ── Search bar ─────────────────────────────────────────────────── */}
      <div className="h-search-row">
        <span className="h-search-icon">⌕</span>
        <input
          className="h-search"
          type="text"
          placeholder="search transcripts…"
          value={query}
          onChange={e => setQuery(e.target.value)}
          spellCheck={false}
          autoComplete="off"
        />
        {query && (
          <button className="h-search-clear" onClick={() => setQuery("")} aria-label="clear search">
            ×
          </button>
        )}
      </div>

      {/* ── Error ──────────────────────────────────────────────────────── */}
      {error && <div className="h-error">{error}</div>}

      {/* ── List ───────────────────────────────────────────────────────── */}
      <div className="h-list">
        {loading && (
          <div className="h-empty h-empty--loading">loading…</div>
        )}
        {!loading && entries.length === 0 && (
          <div className="h-empty">
            {query ? "no matches" : "no transcripts yet"}
          </div>
        )}
        {!loading && visible.map(entry => (
          <div key={entry.id} className="h-item">
            <div className="h-item-text">{entry.text}</div>
            <div className="h-item-meta">
              <span className="h-item-date">{fmtDate(entry.created_at)}</span>
              {entry.language && entry.language !== "en" && (
                <span className="h-item-lang">{entry.language}</span>
              )}
            </div>
            <div className="h-item-actions">
              <button
                className={`h-btn-copy ${copiedId === entry.id ? "h-btn-copy--ok" : ""}`}
                onClick={() => handleCopy(entry)}
                title="copy to clipboard"
              >
                {copiedId === entry.id ? "✓" : "copy"}
              </button>
              <button
                className="h-btn-del"
                onClick={() => handleDelete(entry.id)}
                title="delete entry"
              >
                ×
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* ── Pagination ─────────────────────────────────────────────────── */}
      {totalPages > 1 && (
        <div className="h-pagination">
          <button
            className="h-pg-btn"
            onClick={() => setPage(p => Math.max(0, p - 1))}
            disabled={page === 0}
          >‹</button>
          <span className="h-pg-info">{page + 1} / {totalPages}</span>
          <button
            className="h-pg-btn"
            onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))}
            disabled={page >= totalPages - 1}
          >›</button>
        </div>
      )}

      {/* ── Footer ─────────────────────────────────────────────────────── */}
      <div className="h-footer">
        <span className="h-count">
          {entries.length} {entries.length === 1 ? "entry" : "entries"}
        </span>
        <button
          className="h-btn-clear"
          onClick={handleClearAll}
          disabled={clearing || entries.length === 0}
        >
          {clearing ? "clearing…" : "clear all"}
        </button>
        <button className="h-btn-ghost" onClick={onClose}>close</button>
      </div>

    </div>
  );
}
