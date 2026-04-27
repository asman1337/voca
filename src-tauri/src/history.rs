//! Transcript history — SQLite-backed log of all VOCA dictation sessions.
//!
//! # Storage
//! Database lives at the platform data dir:
//! - Windows:  `%APPDATA%\voca\history.db`
//! - macOS:    `~/Library/Application Support/voca/history.db`
//! - Linux:    `~/.local/share/voca/history.db`
//!
//! # Schema
//! ```sql
//! CREATE TABLE history (
//!   id         INTEGER PRIMARY KEY AUTOINCREMENT,
//!   text       TEXT    NOT NULL,
//!   model_id   TEXT    NOT NULL DEFAULT '',
//!   language   TEXT    NOT NULL DEFAULT 'en',
//!   duration_s REAL    NOT NULL DEFAULT 0.0,
//!   created_at INTEGER NOT NULL            -- Unix timestamp (seconds)
//! );
//! ```

use std::{path::PathBuf, sync::Mutex};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ─── Path ────────────────────────────────────────────────────────────────────

pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voca")
        .join("history.db")
}

// ─── Types ───────────────────────────────────────────────────────────────────

/// A single history entry returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id:         i64,
    pub text:       String,
    pub model_id:   String,
    pub language:   String,
    pub duration_s: f64,
    /// Unix timestamp (seconds since epoch).
    pub created_at: i64,
}

/// Parameters for [`HistoryDb::insert`].
pub struct NewEntry<'a> {
    pub text:       &'a str,
    pub model_id:   &'a str,
    pub language:   &'a str,
    pub duration_s: f64,
}

// ─── Database handle ─────────────────────────────────────────────────────────

pub struct HistoryDb {
    conn: Mutex<Connection>,
}

impl HistoryDb {
    /// Open (or create) the SQLite database and run migrations.
    pub fn open() -> Result<Self, String> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let conn = Connection::open(&path).map_err(|e| e.to_string())?;

        // WAL mode — better concurrency, faster writes.
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| e.to_string())?;

        // Schema migration (idempotent).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
               id         INTEGER PRIMARY KEY AUTOINCREMENT,
               text       TEXT    NOT NULL,
               model_id   TEXT    NOT NULL DEFAULT '',
               language   TEXT    NOT NULL DEFAULT 'en',
               duration_s REAL    NOT NULL DEFAULT 0.0,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_history_created
               ON history (created_at DESC);",
        )
        .map_err(|e| e.to_string())?;

        log::info!("HistoryDb: opened at {:?}", path);
        Ok(Self { conn: Mutex::new(conn) })
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Append a new transcript to the history.
    pub fn insert(&self, entry: NewEntry<'_>) -> Result<i64, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (text, model_id, language, duration_s, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entry.text, entry.model_id, entry.language, entry.duration_s, now],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        log::debug!("HistoryDb: inserted row {id}");
        Ok(id)
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Return up to `limit` most-recent entries, optionally filtered by a
    /// case-insensitive `query` substring against the transcript text.
    pub fn list(&self, query: Option<&str>, limit: u32) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = if let Some(q) = query {
            let like = format!("%{q}%");
            let mut s = conn
                .prepare(
                    "SELECT id, text, model_id, language, duration_s, created_at
                     FROM history
                     WHERE text LIKE ?1 COLLATE NOCASE
                     ORDER BY created_at DESC
                     LIMIT ?2",
                )
                .map_err(|e| e.to_string())?;
            let rows = s
                .query_map(params![like, limit], row_to_entry)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            return Ok(rows);
        } else {
            conn.prepare(
                "SELECT id, text, model_id, language, duration_s, created_at
                 FROM history
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?
        };

        let rows = stmt
            .query_map(params![limit], row_to_entry)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(rows)
    }

    /// Total number of entries.
    pub fn count(&self) -> Result<u32, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    /// Delete a single entry by ID.
    pub fn delete(&self, id: i64) -> Result<(), String> {
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM history WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Wipe the entire history table.
    pub fn clear(&self) -> Result<(), String> {
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM history", [])
            .map_err(|e| e.to_string())?;
        log::info!("HistoryDb: history cleared");
        Ok(())
    }
}

// ─── Row mapper ──────────────────────────────────────────────────────────────

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id:         row.get(0)?,
        text:       row.get(1)?,
        model_id:   row.get(2)?,
        language:   row.get(3)?,
        duration_s: row.get(4)?,
        created_at: row.get(5)?,
    })
}
