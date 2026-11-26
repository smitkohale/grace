//! Claude Code adapter v0 (spec §7.6).
//!
//! Finds transcripts at `<claude home>/projects/<encoded path>/<session id>.jsonl`
//! and turns each line into normalized events and context changes. Verified
//! against Claude Code 2.1.330. Unknown entry types are logged, never fatal.

use super::{Adapter, ParseOutcome, ParseState, ParseStats, Parsed, SessionRef};
use crate::{
    Boundary, BoundaryKind, Event, EventKind, Redactor, ToolId, paths, prefix_chars, timefmt,
    truncate_chars,
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const SUMMARY_CHARS: usize = 160;
const TITLE_CHARS: usize = 80;
const PROMPT_CHARS: usize = 200;
/// How much text to look at before summarizing (secrets are redacted within it).
const TEXT_WINDOW: usize = SUMMARY_CHARS * 4 + 64;
const REMEMBERED_TOOL_IDS: usize = 256;
const MAX_KEPT_LINE_CAPACITY: usize = 16 << 20;

/// Entry types that carry no event: bookkeeping, or context Grace doesn't need yet.
const IGNORED_TYPES: &[&str] = &[
    "api-request-blob",
    "agent-name",
    "mode",
    "relocated",
    "tools-used",
    "atis-latch",
    "queue-operation",
    "data-flow-records",
    "file-history-snapshot",
    "file-history-delta",
    "cost-state",
];
const IGNORED_SYSTEM: &[&str] = &["informational"];
const IGNORED_ATTACHMENTS: &[&str] = &[
    "environment",
    "date",
    "model",
    "skill_listing",
    "deferred_tools_record",
    "deferred_tools_delta",
    "agent_listing_delta",
    "mcp_instructions_delta",
    "auto_mode",
    "session_context",
    "credential_org",
    "remote_session_change",
    "silent_turn_reminder",
    "thinking_drop",
    "command_permissions",
    "read_truncation_notice",
    "file",
    "edited_text_file",
    "queued_command",
];
/// The agent's own task lists: the desktop app's plan tool and the CLI's task tools.
const TASK_TOOLS: &[&str] = &[
    "mcp__ccd_turn__update_plan",
    "TodoWrite",
    "TaskCreate",
    "TaskUpdate",
];

/// Reads Claude Code transcripts under `<claude home>/projects`.
pub struct ClaudeCode {
    root: PathBuf,
}

impl ClaudeCode {
    pub fn new(projects_dir: PathBuf) -> Self {
        Self { root: projects_dir }
    }

    pub fn from_env() -> Option<Self> {
        paths::claude_home().map(|home| Self::new(home.join("projects")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Adapter for ClaudeCode {
    fn tool(&self) -> ToolId {
        ToolId::ClaudeCode
    }

    fn discover(&self) -> Result<Vec<SessionRef>> {
        let mut sessions = Vec::new();
        let projects = match std::fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(sessions),
            Err(err) => {
                return Err(err).with_context(|| format!("couldn't list {}", self.root.display()));
            }
        };
        for project in projects.flatten() {
            if !project.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let Ok(files) = std::fs::read_dir(project.path()) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                let Ok(meta) = file.metadata() else { continue };
                if !meta.is_file() {
                    continue;
                }
                let session_id = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_string();
                sessions.push(SessionRef {
                    tool: ToolId::ClaudeCode,
                    session_id,
                    size: meta.len(),
                    mtime_ms: meta.modified().ok().and_then(timefmt::system_time_ms),
                    path,
                });
            }
        }
        sessions.sort_by(|a, b| {
            b.mtime_ms
                .cmp(&a.mtime_ms)
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(sessions)
    }

    fn parse(
        &self,
        session: &SessionRef,
        state: Option<ParseState>,
        redactor: &Redactor,
        sink: &mut dyn FnMut(Parsed) -> Result<()>,
    ) -> Result<ParseOutcome> {
        if paths::is_credential_file(&session.path) {
            bail!(
                "refusing to read a credential file: {}",
                session.path.display()
            );
        }
        let state = state.unwrap_or_else(|| ParseState::new(&session.session_id));
        let mut parser = Parser {
            state,
            stats: ParseStats::default(),
            redactor,
        };
        let file = File::open(&session.path)
            .with_context(|| format!("couldn't open {}", session.path.display()))?;
        let mut reader = BufReader::with_capacity(256 * 1024, file);
        reader.seek(SeekFrom::Start(parser.state.offset))?;

        let mut buf = Vec::with_capacity(64 * 1024);
        loop {
            buf.clear();
            let read = reader.read_until(b'\n', &mut buf)?;
            // Stop at the end, or before a line Claude Code is still writing.
            if read == 0 || buf.last() != Some(&b'\n') {
                break;
            }
            parser.state.offset += read as u64;
            parser.state.line_no += 1;
            parser.stats.lines += 1;

            let text = String::from_utf8_lossy(&buf);
            let line = text.trim();
            if line.is_empty() {
                continue;
            }
            if ignorable_without_parsing(line) {
                parser.stats.ignored += 1;
            } else {
                match serde_json::from_str::<Value>(line) {
                    Ok(entry) if entry.is_object() => parser.entry(&entry, sink)?,
                    _ => parser.stats.bad_lines += 1,
                }
            }
            if buf.capacity() > MAX_KEPT_LINE_CAPACITY {
                buf = Vec::with_capacity(64 * 1024);
            }
        }
        Ok(parser.finish())
    }
}

/// When `"type"` is the first key and names an ignored type, skip the JSON
/// parse entirely: these lines (message blobs, file history) can be megabytes.
fn ignorable_without_parsing(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(r#"{"type":""#) else {
        return false;
    };
    rest.find('"')
        .is_some_and(|end| IGNORED_TYPES.contains(&&rest[..end]))
}

/// An event before it gets its position.
struct Ev {
    kind: EventKind,
    label: String,
    summary: String,
    detail: Value,
    model: Option<String>,
}

fn ev(kind: EventKind, label: impl Into<String>, summary: String, detail: Value) -> Ev {
    Ev {
        kind,
        label: label.into(),
        summary,
        detail,
        model: None,
    }
}

type Sink<'a> = &'a mut dyn FnMut(Parsed) -> Result<()>;

struct Parser<'r> {
    state: ParseState,
    stats: ParseStats,
    redactor: &'r Redactor,
}

impl Parser<'_> {
    fn clean(&self, text: &str, max: usize) -> String {
        self.redactor.summary(text, max)
    }

    fn entry(&mut self, e: &Value, sink: Sink) -> Result<()> {
        self.observe(e);
        match str_at(e, "type").unwrap_or("(no type)") {
            "user" => self.user(e, sink),
            "assistant" => self.assistant(e, sink),
            "system" => self.system(e, sink),
            "attachment" => self.attachment(e, sink),
            "api-request" => self.api_request(e, sink),
            "api-request-shape" => self.api_request_shape(e, sink),
            "derived-from" => self.derived_from(e, sink),
            "custom-title" => {
                let title = str_at(e, "customTitle").map(|t| self.clean(t, TITLE_CHARS));
                if title.is_some() {
                    self.state.custom_title = title;
                }
                self.stats.ignored += 1;
                Ok(())
            }
            "summary" => {
                let title = str_at(e, "summary").map(|t| self.clean(t, TITLE_CHARS));
                if title.is_some() {
                    self.state.summary_title = title;
                }
                self.stats.ignored += 1;
                Ok(())
            }
            "last-prompt" => {
                let prompt = str_at(e, "lastPrompt").map(|t| self.clean(t, PROMPT_CHARS));
                if prompt.is_some() {
                    self.state.info.last_prompt = prompt;
                }
                self.stats.ignored += 1;
                Ok(())
            }
            kind if IGNORED_TYPES.contains(&kind) => {
                self.stats.ignored += 1;
                Ok(())
            }
            kind => {
                let label = kind.to_string();
                self.unknown(label, e, sink)
            }
        }
    }

    /// Session-wide facts carried on most entries.
    fn observe(&mut self, e: &Value) {
        let info = &mut self.state.info;
        if info.project_path.is_none()
            && let Some(cwd) = str_at(e, "cwd").filter(|cwd| !cwd.is_empty())
        {
            info.project_path = Some(cwd.to_string());
        }
        if let Some(version) = str_at(e, "version") {
            info.version = Some(version.to_string());
        }
        if info.entrypoint.is_none() {
            info.entrypoint = str_at(e, "entrypoint").map(String::from);
        }
        if let Some(branch) = str_at(e, "gitBranch").filter(|b| !b.is_empty()) {
            info.git_branch = Some(branch.to_string());
        }
        if let Some(ts) = str_at(e, "timestamp") {
            if info.started_at.is_none() {
                info.started_at = Some(ts.to_string());
            }
            info.last_active_at = Some(ts.to_string());
        }
    }

    fn ensure_started(&mut self, e: &Value, sink: Sink) -> Result<()> {
        if self.state.started {
            return Ok(());
        }
        self.state.started = true;
        sink(Parsed::Boundary(Boundary {
            kind: BoundaryKind::SessionStart,
            event_seq: self.state.seq,
            line_no: self.state.line_no,
            timestamp: timestamp(e),
            detail: json!({}),
        }))
    }

    /// A context change at the next event.
    fn boundary(&mut self, kind: BoundaryKind, e: &Value, detail: Value, sink: Sink) -> Result<()> {
        self.ensure_started(e, sink)?;
        sink(Parsed::Boundary(Boundary {
            kind,
            event_seq: self.state.seq,
            line_no: self.state.line_no,
            timestamp: timestamp(e),
            detail,
        }))
    }

    fn push(&mut self, ev: Ev, e: &Value, sink: Sink) -> Result<()> {
        self.ensure_started(e, sink)?;
        let event = Event {
            seq: self.state.seq,
            kind: ev.kind,
            label: ev.label,
            line_no: self.state.line_no,
            turn: (self.state.turn > 0).then_some(self.state.turn),
            timestamp: timestamp(e),
            uuid: str_at(e, "uuid").map(String::from),
            parent_uuid: str_at(e, "parentUuid").map(String::from),
            model: ev.model,
            summary: ev.summary,
            detail: ev.detail,
        };
        self.state.seq += 1;
        sink(Parsed::Event(event))
    }

    fn user(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let message = &e["message"];
        if e["isCompactSummary"].as_bool() == Some(true) {
            let chars = content_chars(&message["content"]);
            let summary = format!("Compaction summary ({chars} characters)");
            let detail = json!({ "chars": chars });
            return self.push(
                ev(
                    EventKind::Compaction,
                    "user/compact_summary",
                    summary,
                    detail,
                ),
                e,
                sink,
            );
        }
        let prompt_index = e["turnPosition"]["promptIndex"].as_u64();
        if let Some(index) = prompt_index {
            self.state.turn = u32::try_from(index + 1).unwrap_or(u32::MAX);
        }

        let mut had_tool_result = false;
        for block in message["content"].as_array().into_iter().flatten() {
            if str_at(block, "type") == Some("tool_result") {
                had_tool_result = true;
                self.tool_result(block, e, sink)?;
            }
        }
        if had_tool_result {
            return Ok(());
        }

        let text = content_text(&message["content"], PROMPT_CHARS * 4 + 64);
        if text.trim().is_empty() {
            return Ok(());
        }
        let human = e["isMeta"].as_bool() != Some(true) && is_human(e);
        if human {
            if prompt_index.is_none() {
                self.state.turn += 1;
            }
            let prompt = self.clean(&text, PROMPT_CHARS);
            if self.state.info.first_prompt.is_none() {
                self.state.info.first_prompt = Some(prompt.clone());
            }
            self.state.info.last_prompt = Some(prompt);
            self.state.info.turns = self.state.info.turns.max(self.state.turn);
        }
        let label = if human { "user/prompt" } else { "user/meta" };
        let summary = self.clean(&text, SUMMARY_CHARS);
        self.push(
            ev(
                EventKind::UserMessage,
                label,
                summary,
                json!({ "meta": !human }),
            ),
            e,
            sink,
        )
    }

    fn tool_result(&mut self, block: &Value, e: &Value, sink: Sink) -> Result<()> {
        let id = str_at(block, "tool_use_id").unwrap_or_default();
        let name = self.tool_name(id).unwrap_or("tool").to_string();
        let is_error = block["is_error"].as_bool() == Some(true);
        let text = content_text(&block["content"], TEXT_WINDOW);
        let head = if is_error {
            format!("{name} result (error)")
        } else {
            format!("{name} result")
        };
        let summary = if text.trim().is_empty() {
            head
        } else {
            truncate_chars(
                &format!("{head}: {}", self.clean(&text, SUMMARY_CHARS)),
                SUMMARY_CHARS,
            )
        };
        let detail = json!({ "tool": name, "tool_use_id": id, "is_error": is_error });
        self.push(
            ev(EventKind::ToolResult, "user/tool_result", summary, detail),
            e,
            sink,
        )
    }

    fn tool_name(&self, id: &str) -> Option<&str> {
        self.state
            .tool_names
            .iter()
            .rev()
            .find(|(known, _)| known == id)
            .map(|(_, name)| name.as_str())
    }

    fn remember_tool(&mut self, id: &str, name: &str) {
        self.state
            .tool_names
            .push((id.to_string(), name.to_string()));
        if self.state.tool_names.len() > REMEMBERED_TOOL_IDS {
            self.state.tool_names.remove(0);
        }
    }

    fn assistant(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let message = &e["message"];
        let model = str_at(message, "model")
            .filter(|m| *m != "<synthetic>")
            .map(String::from);
        let sidechain = e["isSidechain"].as_bool() == Some(true);
        if let Some(model) = &model {
            if !self.state.info.models.contains(model) {
                self.state.info.models.push(model.clone());
            }
            if !sidechain {
                if let Some(previous) = self.state.last_model.clone().filter(|p| p != model) {
                    let detail = json!({ "from": previous, "to": model });
                    self.boundary(BoundaryKind::ModelSwitch, e, detail.clone(), sink)?;
                    let summary = format!("Model changed: {previous} → {model}");
                    let switch = ev(
                        EventKind::ModelSwitch,
                        "assistant/model_switch",
                        summary,
                        detail,
                    );
                    self.push(
                        Ev {
                            model: Some(model.clone()),
                            ..switch
                        },
                        e,
                        sink,
                    )?;
                }
                self.state.last_model = Some(model.clone());
            }
        }

        match &message["content"] {
            Value::String(text) => self.assistant_text(text, &model, e, sink)?,
            Value::Array(blocks) => {
                for block in blocks {
                    match str_at(block, "type") {
                        Some("text") => {
                            let text = str_at(block, "text").unwrap_or_default();
                            self.assistant_text(text, &model, e, sink)?;
                        }
                        Some("thinking" | "redacted_thinking") => {
                            let chars = str_at(block, "thinking").map_or(0, |t| t.chars().count());
                            let summary = format!("(thinking, {chars} characters)");
                            let thinking = ev(
                                EventKind::AssistantMessage,
                                "assistant/thinking",
                                summary,
                                json!({ "chars": chars }),
                            );
                            self.push(
                                Ev {
                                    model: model.clone(),
                                    ..thinking
                                },
                                e,
                                sink,
                            )?;
                        }
                        Some("tool_use" | "server_tool_use") => {
                            self.tool_use(block, &model, e, sink)?;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn assistant_text(
        &mut self,
        text: &str,
        model: &Option<String>,
        e: &Value,
        sink: Sink,
    ) -> Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }
        let summary = self.clean(text, SUMMARY_CHARS);
        let message = ev(
            EventKind::AssistantMessage,
            "assistant/text",
            summary,
            json!({}),
        );
        self.push(
            Ev {
                model: model.clone(),
                ..message
            },
            e,
            sink,
        )
    }

    fn tool_use(
        &mut self,
        block: &Value,
        model: &Option<String>,
        e: &Value,
        sink: Sink,
    ) -> Result<()> {
        let name = str_at(block, "name").unwrap_or("tool").to_string();
        let id = str_at(block, "id").unwrap_or_default().to_string();
        self.remember_tool(&id, &name);
        let input = &block["input"];
        let preview = tool_preview(input);
        let summary = if preview.trim().is_empty() {
            name.clone()
        } else {
            truncate_chars(
                &format!("{name}: {}", self.clean(&preview, SUMMARY_CHARS)),
                SUMMARY_CHARS,
            )
        };
        let detail = json!({ "tool": name, "tool_use_id": id });
        let call = ev(EventKind::ToolCall, "assistant/tool_use", summary, detail);
        self.push(
            Ev {
                model: model.clone(),
                ..call
            },
            e,
            sink,
        )?;

        if TASK_TOOLS.contains(&name.as_str()) {
            let (summary, detail) = task_summary(&name, input);
            let summary = self.clean(&summary, SUMMARY_CHARS);
            let update = ev(
                EventKind::TaskUpdate,
                "assistant/task_update",
                summary,
                detail,
            );
            self.push(
                Ev {
                    model: model.clone(),
                    ..update
                },
                e,
                sink,
            )?;
        }
        Ok(())
    }

    fn system(&mut self, e: &Value, sink: Sink) -> Result<()> {
        match str_at(e, "subtype").unwrap_or("(none)") {
            "compact_boundary" => {
                let meta = &e["compactMetadata"];
                let trigger = str_at(meta, "trigger").unwrap_or("unknown").to_string();
                let (pre, post) = (meta["preTokens"].as_u64(), meta["postTokens"].as_u64());
                self.state.info.compactions += 1;
                let detail = json!({ "trigger": trigger, "pre_tokens": pre, "post_tokens": post });
                self.boundary(BoundaryKind::Compaction, e, detail.clone(), sink)?;
                let summary = match (pre, post) {
                    (Some(pre), Some(post)) => format!(
                        "Context compacted ({trigger}): {} → {} tokens",
                        short_count(pre),
                        short_count(post)
                    ),
                    _ => format!("Context compacted ({trigger})"),
                };
                self.push(
                    ev(
                        EventKind::Compaction,
                        "system/compact_boundary",
                        summary,
                        detail,
                    ),
                    e,
                    sink,
                )
            }
            "stop_hook_summary" => {
                let ran = e["hookCount"].as_u64().unwrap_or(0);
                let errors = e["hookErrors"].as_array().map_or(0, Vec::len);
                let stopped = e["preventedContinuation"].as_bool() == Some(true);
                let mut summary = format!(
                    "Stop hooks: {ran} ran, {errors} {}",
                    if errors == 1 { "error" } else { "errors" }
                );
                if stopped {
                    summary.push_str(", stopped the turn");
                }
                let detail = json!({
                    "hook_event": "Stop",
                    "ran": ran,
                    "errors": errors,
                    "prevented_continuation": stopped,
                });
                self.push(
                    ev(
                        EventKind::HookRun,
                        "system/stop_hook_summary",
                        summary,
                        detail,
                    ),
                    e,
                    sink,
                )
            }
            subtype if IGNORED_SYSTEM.contains(&subtype) => {
                self.stats.ignored += 1;
                Ok(())
            }
            subtype => {
                let label = format!("system/{subtype}");
                self.unknown(label, e, sink)
            }
        }
    }

    fn attachment(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let attachment = &e["attachment"];
        let kind = str_at(attachment, "type").unwrap_or("(none)").to_string();
        match kind.as_str() {
            "instructions" => {
                for file in attachment["files"].as_array().into_iter().flatten() {
                    let path = str_at(file, "path").unwrap_or_default();
                    let file_type = str_at(file, "type").unwrap_or("unknown");
                    let content = str_at(file, "content").unwrap_or_default();
                    let summary = self.clean(
                        &format!("Loaded {} ({file_type})", file_name(path)),
                        SUMMARY_CHARS,
                    );
                    let detail = json!({
                        "path": self.redactor.redact(path),
                        "type": file_type,
                        "chars": content.chars().count(),
                        "hash": fnv1a64(content),
                    });
                    self.push(
                        ev(
                            EventKind::InstructionLoad,
                            "attachment/instructions",
                            summary,
                            detail,
                        ),
                        e,
                        sink,
                    )?;
                }
                Ok(())
            }
            "prompt_snapshot" => {
                let parts = attachment["systemPrompt"].as_array().map_or(0, Vec::len);
                let tools = attachment["tools"].as_array().map_or(0, Vec::len);
                let summary = format!("System prompt snapshot ({parts} parts, {tools} tools)");
                self.push(
                    ev(
                        EventKind::RequestSnapshot,
                        "attachment/prompt_snapshot",
                        summary,
                        json!({ "parts": parts, "tools": tools }),
                    ),
                    e,
                    sink,
                )
            }
            "compact_file_reference" => {
                let path = str_at(attachment, "filename")
                    .or_else(|| str_at(attachment, "displayPath"))
                    .unwrap_or_default();
                let summary = self.clean(
                    &format!("Re-attached {} after compaction", file_name(path)),
                    SUMMARY_CHARS,
                );
                let detail = json!({ "path": self.redactor.redact(path) });
                self.push(
                    ev(
                        EventKind::Compaction,
                        "attachment/compact_file_reference",
                        summary,
                        detail,
                    ),
                    e,
                    sink,
                )
            }
            hook if hook.starts_with("hook_") => {
                let event = str_at(attachment, "hookEvent").unwrap_or("hook");
                let name = str_at(attachment, "hookName").unwrap_or(event);
                let outcome = match hook {
                    "hook_additional_context" => "added context",
                    "hook_success" => "ran",
                    "hook_error" | "hook_blocking_error" => "failed",
                    other => other.trim_start_matches("hook_"),
                };
                let summary = self.clean(&format!("Hook {name} {outcome}"), SUMMARY_CHARS);
                let detail = json!({ "hook_event": event, "hook_name": name });
                self.push(
                    ev(
                        EventKind::HookRun,
                        format!("attachment/{hook}"),
                        summary,
                        detail,
                    ),
                    e,
                    sink,
                )
            }
            ignored if IGNORED_ATTACHMENTS.contains(&ignored) => {
                self.stats.ignored += 1;
                Ok(())
            }
            other => {
                let label = format!("attachment/{other}");
                self.unknown(label, e, sink)
            }
        }
    }

    fn api_request(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let model = str_at(&e["params"], "model").map(String::from);
        let tail = e["tail"].as_array().map_or(0, Vec::len);
        let detail = json!({
            "request_id": str_at(e, "id"),
            "shape_hash": str_at(e, "shapeHash"),
            "keep": e["keep"].as_u64(),
            "tail": tail,
            "has_base": str_at(e, "base").is_some_and(|base| !base.is_empty()),
            "query_source": str_at(e, "querySource"),
        });
        let summary = format!(
            "Request to {} ({tail} new {})",
            model.as_deref().unwrap_or("the model"),
            if tail == 1 { "message" } else { "messages" }
        );
        let request = ev(EventKind::RequestSnapshot, "api-request", summary, detail);
        self.push(Ev { model, ..request }, e, sink)
    }

    fn api_request_shape(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let shape = &e["shape"];
        let parts = shape["system"].as_array().map_or(0, Vec::len);
        let tools = shape["tools"].as_array().map_or(0, Vec::len);
        let hash = str_at(e, "shapeHash").unwrap_or_default();
        let short: String = hash.chars().take(8).collect();
        let summary = format!(
            "System prompt ({parts} parts) and {tools} {} (shape {short})",
            if tools == 1 { "tool" } else { "tools" }
        );
        let detail = json!({ "shape_hash": hash, "parts": parts, "tools": tools });
        self.push(
            ev(
                EventKind::RequestSnapshot,
                "api-request-shape",
                summary,
                detail,
            ),
            e,
            sink,
        )
    }

    /// Lineage. `via: "compact"` repeats the compaction marker; anything else is
    /// a resume or fork (shape not yet seen on a real machine).
    fn derived_from(&mut self, e: &Value, sink: Sink) -> Result<()> {
        let mut resumed = false;
        for source in e["sessions"].as_array().into_iter().flatten() {
            let via = str_at(source, "via").unwrap_or("unknown");
            if via == "compact" {
                continue;
            }
            resumed = true;
            let from = str_at(source, "id").unwrap_or_default();
            let short: String = from.chars().take(8).collect();
            let detail = json!({ "from_session": from, "via": via });
            self.boundary(BoundaryKind::Resume, e, detail.clone(), sink)?;
            let summary = format!("Continued from session {short} ({via})");
            self.push(
                ev(EventKind::Resume, "derived-from", summary, detail),
                e,
                sink,
            )?;
        }
        if !resumed {
            self.stats.ignored += 1;
        }
        Ok(())
    }

    /// Logged as drift; the event carries no content from the entry.
    fn unknown(&mut self, label: String, e: &Value, sink: Sink) -> Result<()> {
        *self.stats.unknown.entry(label.clone()).or_insert(0) += 1;
        let summary = format!("Unknown entry: {label}");
        self.push(ev(EventKind::Unknown, label, summary, json!({})), e, sink)
    }

    fn finish(mut self) -> ParseOutcome {
        let title = self
            .state
            .custom_title
            .clone()
            .or_else(|| self.state.summary_title.clone())
            .or_else(|| {
                self.state
                    .info
                    .first_prompt
                    .as_deref()
                    .map(|prompt| truncate_chars(prompt, TITLE_CHARS))
            });
        self.state.info.title = title;
        ParseOutcome {
            state: self.state,
            stats: self.stats,
        }
    }
}

fn str_at<'v>(value: &'v Value, key: &str) -> Option<&'v str> {
    value.get(key)?.as_str()
}

fn timestamp(e: &Value) -> Option<String> {
    str_at(e, "timestamp").map(String::from)
}

/// A human typed it, rather than a hook, notification or the app itself.
fn is_human(e: &Value) -> bool {
    match str_at(&e["origin"], "kind") {
        Some(kind) => kind == "human",
        None => str_at(e, "turnOrigin").is_none_or(|origin| origin == "human"),
    }
}

/// Text of a message's content (a string, or text blocks), up to about `limit` characters.
fn content_text(content: &Value, limit: usize) -> String {
    let mut out = String::new();
    let mut add = |text: &str| {
        if out.chars().count() >= limit {
            return;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(prefix_chars(text, limit));
    };
    match content {
        Value::String(text) => add(text),
        Value::Array(blocks) => {
            for block in blocks {
                if str_at(block, "type") == Some("text") {
                    add(str_at(block, "text").unwrap_or_default());
                }
            }
        }
        _ => {}
    }
    out
}

fn content_chars(content: &Value) -> usize {
    match content {
        Value::String(text) => text.chars().count(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| str_at(block, "text"))
            .map(|text| text.chars().count())
            .sum(),
        _ => 0,
    }
}

/// The most telling part of a tool's input: the command, path, query and so on.
fn tool_preview(input: &Value) -> String {
    const KEYS: [&str; 9] = [
        "command",
        "file_path",
        "path",
        "pattern",
        "url",
        "query",
        "description",
        "prompt",
        "subject",
    ];
    KEYS.iter()
        .find_map(|key| str_at(input, key))
        .or_else(|| input.as_object()?.values().find_map(Value::as_str))
        .map(|text| prefix_chars(text, TEXT_WINDOW).to_string())
        .unwrap_or_default()
}

fn task_summary(tool: &str, input: &Value) -> (String, Value) {
    if let Some(items) = input["items"]
        .as_array()
        .or_else(|| input["todos"].as_array())
    {
        let count = |status: &str| {
            items
                .iter()
                .filter(|item| str_at(item, "status") == Some(status))
                .count()
        };
        let (total, done, active) = (items.len(), count("completed"), count("in_progress"));
        let noun = if total == 1 { "item" } else { "items" };
        let summary = format!("Task list: {total} {noun}, {done} done, {active} in progress");
        let detail = json!({ "tool": tool, "items": total, "done": done, "in_progress": active });
        return (summary, detail);
    }
    match tool {
        "TaskCreate" => {
            let subject = str_at(input, "subject")
                .or_else(|| str_at(input, "description"))
                .unwrap_or("a task");
            (format!("Task added: {subject}"), json!({ "tool": tool }))
        }
        "TaskUpdate" => {
            let status = str_at(input, "status").unwrap_or("updated");
            let id = match &input["taskId"] {
                Value::String(id) => id.clone(),
                Value::Number(id) => id.to_string(),
                _ => "?".to_string(),
            };
            (
                format!("Task {id}: {status}"),
                json!({ "tool": tool, "status": status }),
            )
        }
        _ => (
            "Task list progress updated".to_string(),
            json!({ "tool": tool }),
        ),
    }
}

/// `150000` → `150k`.
fn short_count(n: u64) -> String {
    if n >= 1_000 {
        format!("{}k", (n + 500) / 1_000)
    } else {
        n.to_string()
    }
}

/// The last component of a Windows or Unix path.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A stable, non-cryptographic fingerprint of instruction text.
fn fnv1a64(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_known_heavy_lines_without_parsing() {
        assert!(ignorable_without_parsing(
            r#"{"type":"api-request-blob","hash":"x"}"#
        ));
        assert!(!ignorable_without_parsing(r#"{"type":"user"}"#));
        assert!(!ignorable_without_parsing(
            r#"{"parentUuid":null,"type":"api-request-blob"}"#
        ));
    }

    #[test]
    fn helpers() {
        assert_eq!(short_count(150_000), "150k");
        assert_eq!(short_count(999), "999");
        assert_eq!(file_name(r"C:\a b\CLAUDE.md"), "CLAUDE.md");
        assert_eq!(file_name("/x/AGENTS.md"), "AGENTS.md");
        assert_eq!(fnv1a64(""), "cbf29ce484222325");
    }
}
