//! The `grace` binary against a fake home folder (with a space in its name).

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude_code")
        .join(name)
}

struct FakeHome {
    _dir: tempfile::TempDir,
    home: PathBuf,
    grace_home: PathBuf,
    local: PathBuf,
    empty: PathBuf,
}

fn fake_home() -> FakeHome {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("Test User");
    let grace_home = dir.path().join("grace home");
    let local = dir.path().join("local");
    let empty = dir.path().join("empty");
    for folder in [&home, &local, &empty] {
        fs::create_dir_all(folder).unwrap();
    }
    let project = home.join(".claude/projects/C--Users-Test-User-Projects-Maple---Moss");
    fs::create_dir_all(&project).unwrap();
    fs::copy(
        fixture("basic.jsonl"),
        project.join("11111111-1111-4111-8111-111111111111.jsonl"),
    )
    .unwrap();

    let day = home.join(".codex/sessions/2026/08/18");
    fs::create_dir_all(&day).unwrap();
    fs::write(
        day.join("rollout-2026-08-18T13-48-50-abc.jsonl"),
        "{\"timestamp\":\"2026-08-18T08:18:50Z\",\"type\":\"session_meta\",\"payload\":{\"originator\":\"codex_vscode\",\"cli_version\":\"0.146.0-alpha.3.1\"}}\n",
    )
    .unwrap();
    // A credential file Grace must never open.
    fs::write(
        home.join(".codex/auth.json"),
        r#"{"token":"SENTINEL-NEVER-READ"}"#,
    )
    .unwrap();
    fs::create_dir_all(home.join(".gemini/antigravity-cli/conversations")).unwrap();

    FakeHome {
        _dir: dir,
        home,
        grace_home,
        local,
        empty,
    }
}

impl FakeHome {
    fn grace(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_grace"));
        command
            .env("GRACE_HOME", &self.grace_home)
            .env("GRACE_DISCOVERY_HOME", &self.home)
            .env("CLAUDE_CONFIG_DIR", self.home.join(".claude"))
            .env("CODEX_HOME", self.home.join(".codex"))
            .env("LOCALAPPDATA", &self.local)
            .env("PATH", &self.empty)
            .env("GRACE_OLLAMA_ADDR", "127.0.0.1:9");
        command
    }
}

#[test]
fn doctor_json_lists_five_tools() {
    let home = fake_home();
    let out = home.grace().args(["doctor", "--json"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("SENTINEL"));
    let report: Value = serde_json::from_str(&text).unwrap();
    let tools = report["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 5);
    let tool = |id: &str| tools.iter().find(|t| t["tool"] == id).unwrap().clone();

    let claude = tool("claude_code");
    assert_eq!(claude["found"], true);
    assert_eq!(claude["sessions"], 1);
    assert_eq!(claude["version"], "2.1.330");
    assert_eq!(claude["readable_now"], true);
    let codex = tool("codex");
    assert_eq!(codex["found"], true);
    assert_eq!(codex["sessions"], 1);
    assert_eq!(codex["version"], "0.146.0-alpha.3.1");
    assert_eq!(codex["readable_now"], false);
    assert_eq!(tool("antigravity")["found"], true);
    assert_eq!(tool("ollama")["found"], false);
    assert_eq!(tool("cursor")["found"], false);
}

#[test]
fn doctor_table_is_plain_words() {
    let home = fake_home();
    let out = home.grace().arg("doctor").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("nothing was changed"), "{text}");
    assert!(text.contains("Claude Code"), "{text}");
    assert!(text.contains("Ollama       not found"), "{text}");
}

#[test]
fn sessions_json_lists_fixture_sessions() {
    let home = fake_home();
    let out = home.grace().args(["sessions", "--json"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["title"], "Fix the sprite sheet");
    assert_eq!(rows[0]["project_name"], "Maple & Moss");
    assert_eq!(rows[0]["turns"], 2);

    let again = home.grace().arg("sessions").output().unwrap();
    let text = String::from_utf8_lossy(&again.stdout);
    assert!(text.contains("Fix the sprite sheet"), "{text}");
    assert!(text.contains("2 turns"), "{text}");
}

fn http(port: u16, request: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    (status, body)
}

#[test]
fn serve_requires_token_and_local_host() {
    let home = fake_home();
    let mut child = home
        .grace()
        .args(["serve", "--port", "0", "--token", "test-token"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let port: u16 = first
        .split("127.0.0.1:")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("no port in {first:?}"));
    assert!(first.contains("#token=test-token"), "{first}");

    let get = |path: &str, extra: &str| {
        http(
            port,
            &format!(
                "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Connection: close\r\n\r\n"
            ),
        )
    };
    let auth = "Authorization: Bearer test-token\r\n";
    assert_eq!(get("/api/health", "").0, 401);
    let (status, body) = get("/api/health", auth);
    assert_eq!(status, 200);
    assert!(body.contains("\"ok\":true"), "{body}");
    let (status, _) = http(
        port,
        "GET /api/health HTTP/1.1\r\nHost: evil.example\r\nAuthorization: Bearer test-token\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(status, 403, "DNS-rebinding style Host must be refused");
    let (status, _) = get(
        "/api/health",
        &format!("{auth}Origin: https://evil.example\r\n"),
    );
    assert_eq!(status, 403, "unknown origins must be refused");
    let (status, body) = get("/api/sessions", auth);
    assert_eq!(status, 200);
    assert!(body.contains('['), "{body}");
    assert_eq!(get("/", "").0, 200);

    child.kill().unwrap();
    let _ = child.wait();
    drop(reader);
}
