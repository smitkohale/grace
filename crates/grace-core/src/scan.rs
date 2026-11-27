//! Scanning: discover sessions, read only what changed, store the results.

use crate::adapters::{Adapter, ParseStats, SessionRef};
use crate::config::Config;
use crate::discover::DiscoveryEnv;
use crate::fingerprint::fnv1a64;
use crate::store::Store;
use crate::{Redactor, ToolId};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

/// How many leading bytes identify a file (to notice files replaced in place).
const HEAD_BYTES: u64 = 4_096;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ScanProgress {
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ScanReport {
    pub sessions_found: usize,
    pub parsed: usize,
    pub skipped: usize,
    pub removed: u64,
    pub events_added: u64,
    /// Entry types no adapter recognized (format drift), by label.
    pub unknown: BTreeMap<String, u64>,
    /// One plain line per session or tool that couldn't be read.
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// The adapters to scan with, given the user's settings. M0 reads Claude Code only.
pub fn adapters_for(config: &Config, env: &DiscoveryEnv) -> Vec<Box<dyn Adapter>> {
    let mut adapters: Vec<Box<dyn Adapter>> = Vec::new();
    if config.tool_enabled(ToolId::ClaudeCode) {
        adapters.push(Box::new(env.claude_code()));
    }
    adapters
}

/// Brings the store up to date with every adapter's sessions. One bad session
/// never stops the scan; it is reported in `errors`.
pub fn scan(
    store: &mut Store,
    adapters: &[Box<dyn Adapter>],
    redactor: &Redactor,
    progress: &mut dyn FnMut(&ScanProgress),
) -> Result<ScanReport> {
    let started = Instant::now();
    let mut report = ScanReport::default();
    for adapter in adapters {
        let tool = adapter.tool();
        let sessions = match adapter.discover() {
            Ok(sessions) => sessions,
            Err(err) => {
                report
                    .errors
                    .push(format!("{}: {err:#}", tool.display_name()));
                continue;
            }
        };
        report.sessions_found += sessions.len();
        let present: Vec<String> = sessions
            .iter()
            .map(|session| session.path.to_string_lossy().into_owned())
            .collect();
        report.removed += store.remove_missing(tool, &present)?;

        let total = sessions.len();
        for (done, session) in sessions.iter().enumerate() {
            progress(&ScanProgress {
                done,
                total,
                current: Some(session.session_id.clone()),
            });
            match scan_session(store, adapter.as_ref(), session, redactor) {
                Ok(Some((added, stats))) => {
                    report.parsed += 1;
                    report.events_added += added;
                    for (label, count) in stats.unknown {
                        *report.unknown.entry(label).or_insert(0) += count;
                    }
                }
                Ok(None) => report.skipped += 1,
                Err(err) => report
                    .errors
                    .push(format!("{}: {err:#}", session.path.display())),
            }
        }
        progress(&ScanProgress {
            done: total,
            total,
            current: None,
        });
    }
    report.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(report)
}

/// Reads one session if it changed. `Ok(None)` means it was unchanged.
fn scan_session(
    store: &mut Store,
    adapter: &dyn Adapter,
    session: &SessionRef,
    redactor: &Redactor,
) -> Result<Option<(u64, ParseStats)>> {
    let key = session.path.to_string_lossy().into_owned();
    let stored = store.stored(&key)?;
    let same_start = match stored.as_ref().and_then(|s| s.head_hash.as_deref()) {
        Some(head) => same_head(&session.path, head)?,
        None => false,
    };
    let current = adapter.parser_version();
    let stored = stored.map(|mut s| {
        // Saved by an older parser: forget the position so it's read again.
        if s.state
            .as_ref()
            .is_some_and(|state| state.parser_version != current)
        {
            s.state = None;
        }
        s
    });
    let (state, reset) = match stored {
        Some(s)
            if same_start
                && s.state.is_some()
                && s.file_size == session.size
                && s.file_mtime_ms == session.mtime_ms =>
        {
            return Ok(None);
        }
        // Grew since last time: carry on from where the last read stopped.
        Some(s)
            if same_start
                && session.size >= s.file_size
                && s.state
                    .as_ref()
                    .is_some_and(|state| state.offset <= session.size) =>
        {
            (s.state, false)
        }
        // Shrank, was replaced, or has no usable state: start over.
        Some(_) => (None, true),
        None => (None, false),
    };

    let head = head_of(&session.path)?;
    let result = store.write_session(session, Some(&head), reset, |writer| {
        adapter.parse(session, state, redactor, &mut |item| writer.push(item))
    })?;
    store.record_drift(
        adapter.tool(),
        result.version.as_deref(),
        &result.stats.unknown,
    )?;
    Ok(Some((result.events_added, result.stats)))
}

fn read_prefix(path: &Path, len: u64) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("couldn't open {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(len).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// `"<length>:<fingerprint>"` of the file's first bytes.
fn head_of(path: &Path) -> Result<String> {
    let bytes = read_prefix(path, HEAD_BYTES)?;
    Ok(format!("{}:{}", bytes.len(), fnv1a64(&bytes)))
}

/// Whether the file still starts with the bytes recorded in `stored`.
fn same_head(path: &Path, stored: &str) -> Result<bool> {
    let Some((len, hash)) = stored.split_once(':') else {
        return Ok(false);
    };
    let Ok(len) = len.parse::<u64>() else {
        return Ok(false);
    };
    let bytes = read_prefix(path, len)?;
    Ok(bytes.len() as u64 == len && fnv1a64(&bytes) == hash)
}
