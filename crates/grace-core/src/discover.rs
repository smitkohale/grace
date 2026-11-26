//! Read-only discovery of the AI tools on this machine (`grace doctor` and the
//! "Find your tools" step). It reads session files' first lines and runs
//! `--version` on known executables. It never opens credential files (R8).

use crate::adapters::{Adapter, ClaudeCode};
use crate::{ToolId, paths, timefmt};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// One tool's status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolStatus {
    pub tool: ToolId,
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
    /// The folder Grace reads (or will read) for this tool.
    pub data_dir: Option<String>,
    pub sessions: Option<u64>,
    pub executable: Option<String>,
    /// Whether this version of Grace can read the tool's sessions.
    pub readable_now: bool,
    /// Short plain-language facts for the CLI.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub grace_home: String,
    pub grace_home_exists: bool,
    pub tools: Vec<ToolStatus>,
}

/// Where to look. Built from the real system, or by hand in tests.
#[derive(Debug, Clone)]
pub struct DiscoveryEnv {
    pub home: PathBuf,
    pub grace_home: PathBuf,
    pub path_dirs: Vec<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub claude_home: PathBuf,
    pub codex_home: PathBuf,
    pub gemini_home: PathBuf,
    pub ollama_addr: SocketAddr,
}

impl DiscoveryEnv {
    pub fn from_system() -> Self {
        let home = paths::home_dir().unwrap_or_default();
        Self {
            grace_home: paths::grace_home(),
            path_dirs: std::env::var_os("PATH")
                .map(|path| std::env::split_paths(&path).collect())
                .unwrap_or_default(),
            local_app_data: std::env::var_os("LOCALAPPDATA")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .or_else(dirs::data_local_dir),
            claude_home: paths::claude_home().unwrap_or_else(|| home.join(".claude")),
            codex_home: paths::codex_home().unwrap_or_else(|| home.join(".codex")),
            gemini_home: paths::gemini_home().unwrap_or_else(|| home.join(".gemini")),
            ollama_addr: std::env::var("GRACE_OLLAMA_ADDR")
                .ok()
                .and_then(|addr| addr.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 11434))),
            home,
        }
    }

    /// The Claude Code adapter for this environment.
    pub fn claude_code(&self) -> ClaudeCode {
        ClaudeCode::new(self.claude_home.join("projects"))
    }

    fn find_executable(&self, name: &str) -> Option<PathBuf> {
        let names: Vec<String> = if cfg!(windows) {
            vec![format!("{name}.exe"), format!("{name}.cmd")]
        } else {
            vec![name.to_string()]
        };
        self.path_dirs
            .iter()
            .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
            .find(|path| path.is_file())
    }

    /// A path under `%LOCALAPPDATA%` (Windows), if it exists.
    fn local(&self, parts: &[&str]) -> Option<PathBuf> {
        let mut path = self.local_app_data.clone()?;
        for part in parts {
            path.push(part);
        }
        path.exists().then_some(path)
    }
}

/// Checks every tool. Read-only and quick (a few seconds at most).
pub fn doctor(env: &DiscoveryEnv) -> DoctorReport {
    DoctorReport {
        grace_home: env.grace_home.display().to_string(),
        grace_home_exists: env.grace_home.exists(),
        tools: vec![
            claude_code(env),
            codex(env),
            antigravity(env),
            ollama(env),
            cursor(env),
        ],
    }
}

fn status(tool: ToolId) -> ToolStatus {
    ToolStatus {
        tool,
        name: tool.display_name().to_string(),
        found: false,
        version: None,
        data_dir: None,
        sessions: None,
        executable: None,
        readable_now: tool == ToolId::ClaudeCode,
        notes: Vec::new(),
    }
}

fn claude_code(env: &DiscoveryEnv) -> ToolStatus {
    let mut s = status(ToolId::ClaudeCode);
    let adapter = env.claude_code();
    let dir = adapter.root().to_path_buf();
    s.data_dir = Some(dir.display().to_string());
    let exe = env.find_executable("claude");
    s.executable = exe.as_ref().map(|p| p.display().to_string());
    let sessions = adapter.discover().unwrap_or_default();
    s.found = dir.is_dir() || exe.is_some();
    if s.found {
        s.sessions = Some(sessions.len() as u64);
    }
    if let Some(newest) = sessions.first() {
        let lines = first_json_lines(&newest.path, 256 * 1024);
        s.version = lines.iter().find_map(|line| str_field(line, "version"));
        if lines
            .iter()
            .any(|line| str_field(line, "entrypoint").as_deref() == Some("claude-desktop"))
        {
            s.notes
                .push("Sessions come from the Claude desktop app.".into());
        }
    }
    if s.found && exe.is_none() {
        s.notes.push(
            "The `claude` command isn't installed. That's fine: Grace reads the session files."
                .into(),
        );
    }
    s
}

fn codex(env: &DiscoveryEnv) -> ToolStatus {
    let mut s = status(ToolId::Codex);
    let dir = env.codex_home.join("sessions");
    s.data_dir = Some(dir.display().to_string());
    let exe = env.find_executable("codex");
    s.executable = exe.as_ref().map(|p| p.display().to_string());
    let extension = vscode_extension(env, "openai.chatgpt-");
    let mut rollouts = Vec::new();
    collect_rollouts(&dir, 0, &mut rollouts);
    s.found = dir.is_dir() || exe.is_some() || extension.is_some();
    if s.found {
        s.sessions = Some(rollouts.len() as u64);
    }
    rollouts.sort_by_key(|(_, mtime)| std::cmp::Reverse(*mtime));
    if let Some((newest, _)) = rollouts.first() {
        s.version = first_json_lines(newest, 64 * 1024)
            .first()
            .and_then(|meta| meta["payload"]["cli_version"].as_str().map(String::from));
    }
    if extension.is_some() {
        s.notes
            .push("Found the Codex extension for VS Code.".into());
    }
    s.notes
        .push("Grace reads Codex sessions from a later update.".into());
    s
}

fn antigravity(env: &DiscoveryEnv) -> ToolStatus {
    let mut s = status(ToolId::Antigravity);
    let cli_dir = env.gemini_home.join("antigravity-cli");
    let ide_dir = env.gemini_home.join("antigravity");
    let data_dir = if cli_dir.is_dir() || !ide_dir.is_dir() {
        cli_dir
    } else {
        ide_dir.clone()
    };
    s.data_dir = Some(data_dir.display().to_string());
    let exe = env
        .find_executable("agy")
        .or_else(|| env.local(&["agy", "bin", "agy.exe"]));
    let ide = env.local(&["Programs", "Antigravity", "Antigravity.exe"]);
    s.executable = exe.as_ref().map(|p| p.display().to_string());
    s.found = data_dir.is_dir() || ide_dir.is_dir() || exe.is_some() || ide.is_some();
    s.version = exe.as_deref().and_then(command_version);
    if ide.is_some() || ide_dir.is_dir() {
        s.notes.push("Found the Antigravity app.".into());
    }
    s.notes
        .push("Grace reads Antigravity sessions from a later update.".into());
    s
}

fn ollama(env: &DiscoveryEnv) -> ToolStatus {
    let mut s = status(ToolId::Ollama);
    s.data_dir = Some(env.home.join(".ollama").display().to_string());
    let exe = env
        .find_executable("ollama")
        .or_else(|| env.local(&["Programs", "Ollama", "ollama.exe"]))
        .or_else(|| Some(PathBuf::from("/Applications/Ollama.app")).filter(|p| p.exists()));
    s.executable = exe.as_ref().map(|p| p.display().to_string());
    let api_version = ollama_version(env.ollama_addr);
    s.found = exe.is_some() || api_version.is_some();
    match &api_version {
        Some(_) => s.notes.push(format!("Running at {}.", env.ollama_addr)),
        None if s.found => s.notes.push("Installed, but not running right now.".into()),
        None => {}
    }
    s.version = api_version;
    s
}

fn cursor(env: &DiscoveryEnv) -> ToolStatus {
    let mut s = status(ToolId::Cursor);
    let dir = env.home.join(".cursor");
    s.data_dir = Some(dir.display().to_string());
    let exe = env
        .find_executable("cursor")
        .or_else(|| env.local(&["Programs", "cursor", "Cursor.exe"]))
        .or_else(|| Some(PathBuf::from("/Applications/Cursor.app")).filter(|p| p.exists()));
    s.executable = exe.as_ref().map(|p| p.display().to_string());
    s.found = dir.is_dir() || exe.is_some();
    s
}

/// `rollout-*.jsonl` and `rollout-*.jsonl.zst` under `sessions/YYYY/MM/DD`.
fn collect_rollouts(dir: &Path, depth: usize, out: &mut Vec<(PathBuf, i64)>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            collect_rollouts(&path, depth + 1, out);
        } else {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("rollout-")
                && (name.ends_with(".jsonl") || name.ends_with(".jsonl.zst"))
            {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(timefmt::system_time_ms)
                    .unwrap_or(0);
                out.push((path, mtime));
            }
        }
    }
}

fn vscode_extension(env: &DiscoveryEnv, prefix: &str) -> Option<PathBuf> {
    let dir = env.home.join(".vscode").join("extensions");
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| entry.path())
}

/// Parses the complete JSON lines within the first `max_bytes` of a file.
fn first_json_lines(path: &Path, max_bytes: u64) -> Vec<Value> {
    if paths::is_credential_file(path) || path.extension().is_some_and(|e| e == "zst") {
        return Vec::new();
    }
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    if file.take(max_bytes).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let complete = text.rfind('\n').map_or("", |end| &text[..end]);
    complete
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect()
}

fn str_field(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(String::from)
}

static VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d+\.\d+(?:\.\d+)?(?:[-+][0-9A-Za-z.-]+)?").expect("version pattern is valid")
});

/// Runs `<exe> --version` with a 4-second limit (`agy` takes about 2.3 s).
/// Only real executables, never shell scripts.
fn command_version(exe: &Path) -> Option<String> {
    let is_binary = if cfg!(windows) {
        exe.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    } else {
        true
    };
    if !is_binary {
        return None;
    }
    let mut command = Command::new(exe);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_console_window(&mut command);
    let mut child = command.spawn().ok()?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    VERSION.find(&output).map(|m| m.as_str().to_string())
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

/// Asks a local Ollama for its version (`GET /api/version`), with short timeouts.
fn ollama_version(addr: SocketAddr) -> Option<String> {
    if !addr.ip().is_loopback() {
        return None;
    }
    let timeout = Duration::from_millis(700);
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    write!(
        stream,
        "GET /api/version HTTP/1.0\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.take(64 * 1024).read_to_string(&mut response).ok()?;
    let body = response.split("\r\n\r\n").nth(1)?;
    let json: Value = serde_json::from_str(body.trim()).ok()?;
    json["version"].as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_versions_from_command_output() {
        assert_eq!(
            VERSION.find("agy version 1.1.5\n").map(|m| m.as_str()),
            Some("1.1.5")
        );
        assert_eq!(
            VERSION
                .find("codex-cli 0.146.0-alpha.3")
                .map(|m| m.as_str()),
            Some("0.146.0-alpha.3")
        );
    }

    #[test]
    fn remote_ollama_is_never_contacted() {
        assert_eq!(
            ollama_version(SocketAddr::from(([192, 0, 2, 1], 11434))),
            None
        );
    }
}
