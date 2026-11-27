//! Grace's local database: one SQLite file in WAL mode, shared safely by the
//! app and the CLI (spec §7.5). Only redacted text is ever written.

mod migrations;

use crate::adapters::{ParseOutcome, ParseState, ParseStats, Parsed, SessionRef};
use crate::{BoundaryKind, EventKind, ToolId, timefmt};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Duration;

use migrations::MIGRATIONS;

/// What the store remembers about a session file between scans.
#[derive(Debug, Clone)]
pub struct StoredSession {
    pub id: i64,
    pub file_size: u64,
    pub file_mtime_ms: Option<i64>,
    pub head_hash: Option<String>,
    pub state: Option<ParseState>,
}

/// One row of the Sessions list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionRow {
    pub id: i64,
    pub tool: ToolId,
    pub session_id: String,
    pub title: Option<String>,
    pub project_path: Option<String>,
    pub project_name: Option<String>,
    pub started_at: Option<String>,
    pub last_active_at: Option<String>,
    pub turns: u32,
    pub compactions: u32,
    pub event_count: u64,
    pub version: Option<String>,
    pub entrypoint: Option<String>,
    pub models: Vec<String>,
    pub source_file: String,
    pub first_prompt: Option<String>,
    pub last_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventRow {
    pub seq: u64,
    pub kind: EventKind,
    pub label: String,
    pub line_no: u64,
    pub turn: Option<u32>,
    pub timestamp: Option<String>,
    pub model: Option<String>,
    pub summary: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoundaryRow {
    pub kind: BoundaryKind,
    pub event_seq: u64,
    pub line_no: u64,
    pub timestamp: Option<String>,
    pub detail: Value,
}

/// A session with its context changes and one page of events.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionDetail {
    pub session: SessionRow,
    pub boundaries: Vec<BoundaryRow>,
    pub events: Vec<EventRow>,
    /// Pass as `after_seq` to get the next page; `None` when this is the last.
    pub next_after: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionQuery {
    /// Matches title, prompts and project name.
    pub q: Option<String>,
    /// Exact project path.
    pub project: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftRow {
    pub tool: String,
    pub label: String,
    pub version: String,
    pub count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Result of [`Store::write_session`].
#[derive(Debug, Clone, Default)]
pub struct WriteResult {
    pub events_added: u64,
    pub stats: ParseStats,
    /// The tool version the session was written with, for drift records.
    pub version: Option<String>,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (creating if needed) the database at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("couldn't create {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("couldn't open the database at {}", path.display()))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.busy_timeout(Duration::from_secs(5))?;
        // Returns the resulting mode ("memory" for in-memory databases).
        conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
        let mut store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<()> {
        let version = self.schema_version()?;
        let known = MIGRATIONS.len() as i64;
        if version > known {
            bail!(
                "this database was made by a newer version of Grace (schema {version}, this version knows {known}); update Grace to read it"
            );
        }
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", index as i64 + 1)?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// The saved scan position for a session file, if it was scanned before.
    pub fn stored(&self, source_file: &str) -> Result<Option<StoredSession>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, file_size, file_mtime_ms, head_hash, parse_state FROM sessions WHERE source_file = ?1",
                [source_file],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(id, size, mtime, head, state)| StoredSession {
            id,
            file_size: u64::try_from(size).unwrap_or(0),
            file_mtime_ms: mtime,
            head_hash: head,
            // A state from an older format is simply re-parsed from the start.
            state: state.and_then(|json| serde_json::from_str(&json).ok()),
        }))
    }

    /// Writes one session's new events in a single transaction. With `reset`,
    /// existing events are dropped first. `parse` receives a writer to push
    /// items into and returns the parse outcome.
    pub fn write_session(
        &mut self,
        session: &SessionRef,
        head_hash: Option<&str>,
        reset: bool,
        parse: impl FnOnce(&mut SessionWriter<'_>) -> Result<ParseOutcome>,
    ) -> Result<WriteResult> {
        let source_file = session.path.to_string_lossy().into_owned();
        let tx = self.conn.transaction()?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM sessions WHERE source_file = ?1",
                [&source_file],
                |row| row.get(0),
            )
            .optional()?;
        let session_ref = match existing {
            Some(id) => {
                if reset {
                    tx.execute("DELETE FROM events WHERE session_ref = ?1", [id])?;
                    tx.execute("DELETE FROM boundaries WHERE session_ref = ?1", [id])?;
                    tx.execute(
                        "UPDATE sessions SET event_count = 0, bad_lines = 0, parse_state = NULL WHERE id = ?1",
                        [id],
                    )?;
                }
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO sessions (tool, session_id, source_file) VALUES (?1, ?2, ?3)",
                    params![session.tool.as_str(), session.session_id, source_file],
                )?;
                tx.last_insert_rowid()
            }
        };
        let next_boundary: i64 = tx.query_row(
            "SELECT COALESCE(MAX(idx) + 1, 0) FROM boundaries WHERE session_ref = ?1",
            [session_ref],
            |row| row.get(0),
        )?;

        let mut writer = SessionWriter {
            conn: &tx,
            session_ref,
            next_boundary,
            events_added: 0,
        };
        let outcome = parse(&mut writer)?;
        let events_added = writer.events_added;

        let info = &outcome.state.info;
        let project_id = match info.project_path.as_deref() {
            Some(path) => Some(upsert_project(&tx, path)?),
            None => None,
        };
        tx.execute(
            "UPDATE sessions SET
                session_id = ?2, project_id = ?3, title = ?4, first_prompt = ?5, last_prompt = ?6,
                version = ?7, entrypoint = ?8, git_branch = ?9, models = ?10, started_at = ?11,
                last_active_at = ?12, turns = ?13, compactions = ?14,
                event_count = event_count + ?15, bad_lines = bad_lines + ?16,
                file_size = ?17, file_mtime_ms = ?18, head_hash = ?19, parse_state = ?20, scanned_at = ?21
             WHERE id = ?1",
            params![
                session_ref,
                info.session_id,
                project_id,
                info.title,
                info.first_prompt,
                info.last_prompt,
                info.version,
                info.entrypoint,
                info.git_branch,
                serde_json::to_string(&info.models)?,
                info.started_at,
                info.last_active_at,
                info.turns,
                info.compactions,
                events_added as i64,
                outcome.stats.bad_lines as i64,
                session.size as i64,
                session.mtime_ms,
                head_hash,
                serde_json::to_string(&outcome.state)?,
                timefmt::now_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(WriteResult {
            events_added,
            version: outcome.state.info.version.clone(),
            stats: outcome.stats,
        })
    }

    /// Sessions, most recently active first.
    pub fn list_sessions(&self, query: &SessionQuery) -> Result<Vec<SessionRow>> {
        let pattern = query
            .q
            .as_deref()
            .filter(|q| !q.trim().is_empty())
            .map(like_pattern);
        let limit = if query.limit == 0 { 100 } else { query.limit };
        let sql = format!(
            "{SESSION_SELECT}
             WHERE (?1 IS NULL OR s.title LIKE ?1 ESCAPE '\\' OR s.first_prompt LIKE ?1 ESCAPE '\\'
                    OR s.last_prompt LIKE ?1 ESCAPE '\\' OR p.name LIKE ?1 ESCAPE '\\')
               AND (?2 IS NULL OR p.path = ?2)
             ORDER BY s.last_active_at DESC NULLS LAST, s.id DESC
             LIMIT ?3"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![pattern, query.project, limit], session_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// A session with its context changes and up to `limit` events after `after_seq`.
    pub fn session_detail(
        &self,
        id: i64,
        after_seq: Option<u64>,
        limit: u32,
    ) -> Result<Option<SessionDetail>> {
        let session = self
            .conn
            .query_row(
                &format!("{SESSION_SELECT} WHERE s.id = ?1"),
                [id],
                session_row,
            )
            .optional()?;
        let Some(session) = session else {
            return Ok(None);
        };

        let mut stmt = self.conn.prepare(
            "SELECT kind, event_seq, line_no, ts, detail FROM boundaries WHERE session_ref = ?1 ORDER BY idx",
        )?;
        let boundaries = stmt
            .query_map([id], |row| {
                Ok(BoundaryRow {
                    kind: BoundaryKind::parse(&row.get::<_, String>(0)?)
                        .unwrap_or(BoundaryKind::SessionStart),
                    event_seq: row.get::<_, i64>(1)? as u64,
                    line_no: row.get::<_, i64>(2)? as u64,
                    timestamp: row.get(3)?,
                    detail: json_column(row, 4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let limit = limit.clamp(1, 1_000);
        let after = after_seq.map_or(-1, |seq| seq as i64);
        let mut stmt = self.conn.prepare(
            "SELECT seq, kind, label, line_no, turn, ts, model, summary, detail FROM events
             WHERE session_ref = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3",
        )?;
        let mut events = stmt
            .query_map(params![id, after, i64::from(limit) + 1], |row| {
                Ok(EventRow {
                    seq: row.get::<_, i64>(0)? as u64,
                    kind: EventKind::parse(&row.get::<_, String>(1)?).unwrap_or(EventKind::Unknown),
                    label: row.get(2)?,
                    line_no: row.get::<_, i64>(3)? as u64,
                    turn: row.get(4)?,
                    timestamp: row.get(5)?,
                    model: row.get(6)?,
                    summary: row.get(7)?,
                    detail: json_column(row, 8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let next_after = if events.len() > limit as usize {
            events.truncate(limit as usize);
            events.last().map(|event| event.seq)
        } else {
            None
        };
        Ok(Some(SessionDetail {
            session,
            boundaries,
            events,
            next_after,
        }))
    }

    /// Drops sessions of `tool` whose files are no longer in `present`.
    pub fn remove_missing(&mut self, tool: ToolId, present: &[String]) -> Result<u64> {
        let present: HashSet<&str> = present.iter().map(String::as_str).collect();
        let mut stmt = self
            .conn
            .prepare("SELECT id, source_file FROM sessions WHERE tool = ?1")?;
        let gone: Vec<i64> = stmt
            .query_map([tool.as_str()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(Result::ok)
            .filter(|(_, file)| !present.contains(file.as_str()))
            .map(|(id, _)| id)
            .collect();
        drop(stmt);
        let tx = self.conn.transaction()?;
        for id in &gone {
            tx.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        }
        tx.execute(
            "DELETE FROM projects WHERE id NOT IN (SELECT project_id FROM sessions WHERE project_id IS NOT NULL)",
            [],
        )?;
        tx.commit()?;
        Ok(gone.len() as u64)
    }

    /// Counts entry types an adapter didn't recognize (format drift).
    pub fn record_drift(
        &self,
        tool: ToolId,
        version: Option<&str>,
        unknown: &BTreeMap<String, u64>,
    ) -> Result<()> {
        if unknown.is_empty() {
            return Ok(());
        }
        let now = timefmt::now_rfc3339();
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO drift (tool, label, version, count, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(tool, label, version) DO UPDATE SET count = count + excluded.count, last_seen = excluded.last_seen",
        )?;
        for (label, count) in unknown {
            stmt.execute(params![
                tool.as_str(),
                label,
                version.unwrap_or(""),
                *count as i64,
                now
            ])?;
        }
        Ok(())
    }

    pub fn drift(&self) -> Result<Vec<DriftRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT tool, label, version, count, first_seen, last_seen FROM drift ORDER BY count DESC, label",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DriftRow {
                    tool: row.get(0)?,
                    label: row.get(1)?,
                    version: row.get(2)?,
                    count: row.get::<_, i64>(3)? as u64,
                    first_seen: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

/// Receives one session's parsed items inside [`Store::write_session`].
pub struct SessionWriter<'a> {
    conn: &'a Connection,
    session_ref: i64,
    next_boundary: i64,
    events_added: u64,
}

impl SessionWriter<'_> {
    pub fn push(&mut self, item: Parsed) -> Result<()> {
        match item {
            Parsed::Event(event) => {
                self.conn
                    .prepare_cached(
                        "INSERT OR REPLACE INTO events
                         (session_ref, seq, kind, label, line_no, turn, ts, uuid, parent_uuid, model, summary, detail)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    )?
                    .execute(params![
                        self.session_ref,
                        event.seq as i64,
                        event.kind.as_str(),
                        event.label,
                        event.line_no as i64,
                        event.turn,
                        event.timestamp,
                        event.uuid,
                        event.parent_uuid,
                        event.model,
                        event.summary,
                        event.detail.to_string(),
                    ])?;
                self.events_added += 1;
            }
            Parsed::Boundary(boundary) => {
                self.conn
                    .prepare_cached(
                        "INSERT OR REPLACE INTO boundaries (session_ref, idx, kind, event_seq, line_no, ts, detail)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    )?
                    .execute(params![
                        self.session_ref,
                        self.next_boundary,
                        boundary.kind.as_str(),
                        boundary.event_seq as i64,
                        boundary.line_no as i64,
                        boundary.timestamp,
                        boundary.detail.to_string(),
                    ])?;
                self.next_boundary += 1;
            }
        }
        Ok(())
    }
}

const SESSION_SELECT: &str =
    "SELECT s.id, s.tool, s.session_id, s.title, p.path, p.name, s.started_at,
        s.last_active_at, s.turns, s.compactions, s.event_count, s.version, s.entrypoint, s.models,
        s.source_file, s.first_prompt, s.last_prompt
     FROM sessions s LEFT JOIN projects p ON p.id = s.project_id";

fn session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    let tool: String = row.get(1)?;
    let models: String = row.get(13)?;
    Ok(SessionRow {
        id: row.get(0)?,
        tool: ToolId::parse(&tool).unwrap_or(ToolId::ClaudeCode),
        session_id: row.get(2)?,
        title: row.get(3)?,
        project_path: row.get(4)?,
        project_name: row.get(5)?,
        started_at: row.get(6)?,
        last_active_at: row.get(7)?,
        turns: row.get(8)?,
        compactions: row.get(9)?,
        event_count: row.get::<_, i64>(10)? as u64,
        version: row.get(11)?,
        entrypoint: row.get(12)?,
        models: serde_json::from_str(&models).unwrap_or_default(),
        source_file: row.get(14)?,
        first_prompt: row.get(15)?,
        last_prompt: row.get(16)?,
    })
}

fn json_column(row: &Row<'_>, index: usize) -> rusqlite::Result<Value> {
    let text: String = row.get(index)?;
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

/// `%text%` for LIKE, with `%`, `_` and `\` escaped.
fn like_pattern(text: &str) -> String {
    let escaped = text
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

/// The project's display name: the last folder of its path.
pub fn project_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn upsert_project(conn: &Connection, path: &str) -> Result<i64> {
    Ok(conn.query_row(
        "INSERT INTO projects (path, name) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET name = excluded.name RETURNING id",
        params![path, project_name(path)],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_names() {
        assert_eq!(
            project_name(r"C:\Users\Test User\Projects\Maple & Moss"),
            "Maple & Moss"
        );
        assert_eq!(project_name("/home/test/api/"), "api");
        assert_eq!(like_pattern("50%_off"), r"%50\%\_off%");
    }
}
