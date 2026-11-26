//! Tool adapters turn each tool's session files into normalized events (spec §7.3).

pub mod claude_code;

use crate::{Boundary, Event, Redactor, ToolId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub use claude_code::ClaudeCode;

/// A session file found on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRef {
    pub tool: ToolId,
    pub session_id: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime_ms: Option<i64>,
}

/// What Grace knows about a session as a whole. Text fields are redacted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionInfo {
    pub session_id: String,
    pub project_path: Option<String>,
    pub title: Option<String>,
    pub first_prompt: Option<String>,
    pub last_prompt: Option<String>,
    pub version: Option<String>,
    pub entrypoint: Option<String>,
    pub git_branch: Option<String>,
    pub models: Vec<String>,
    pub started_at: Option<String>,
    pub last_active_at: Option<String>,
    pub turns: u32,
    pub compactions: u32,
}

/// Where to resume reading a session, plus everything the next read needs,
/// so a growing transcript is read incrementally with identical results.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParseState {
    /// Byte offset just after the last complete line read.
    pub offset: u64,
    /// Number of the last complete line read.
    pub line_no: u64,
    /// The next event's `seq`.
    pub seq: u64,
    pub turn: u32,
    pub last_model: Option<String>,
    pub started: bool,
    /// Recent `(tool_use_id, tool name)` pairs, so results can name their tool.
    pub tool_names: Vec<(String, String)>,
    pub custom_title: Option<String>,
    pub summary_title: Option<String>,
    pub info: SessionInfo,
}

impl ParseState {
    pub fn new(session_id: &str) -> Self {
        Self {
            info: SessionInfo {
                session_id: session_id.to_string(),
                ..SessionInfo::default()
            },
            ..Self::default()
        }
    }
}

/// One item from a parse, in file order.
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Event(Event),
    Boundary(Boundary),
}

/// Counts from one parse call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ParseStats {
    pub lines: u64,
    pub bad_lines: u64,
    pub ignored: u64,
    pub unknown: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    pub state: ParseState,
    pub stats: ParseStats,
}

/// Reads one tool's sessions. Discovery and parsing are read-only.
pub trait Adapter: Send + Sync {
    fn tool(&self) -> ToolId;

    /// Finds session files, newest first.
    fn discover(&self) -> Result<Vec<SessionRef>>;

    /// Streams a session from `state` (None = from the start) and calls `sink`
    /// for every event and context change, in order.
    fn parse(
        &self,
        session: &SessionRef,
        state: Option<ParseState>,
        redactor: &Redactor,
        sink: &mut dyn FnMut(Parsed) -> Result<()>,
    ) -> Result<ParseOutcome>;
}
