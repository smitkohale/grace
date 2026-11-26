//! Tool discovery with a hand-built environment.

use grace_core::ToolId;
use grace_core::discover::{DiscoveryEnv, doctor};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

fn env_for(root: &Path) -> DiscoveryEnv {
    let home = root.join("Test User");
    DiscoveryEnv {
        grace_home: root.join("grace"),
        path_dirs: vec![root.join("empty")],
        local_app_data: Some(root.join("local")),
        claude_home: home.join(".claude"),
        codex_home: home.join(".codex"),
        gemini_home: home.join(".gemini"),
        ollama_addr: SocketAddr::from(([127, 0, 0, 1], 9)),
        home,
    }
}

#[test]
fn doctor_reports_missing_tools() {
    let dir = tempfile::tempdir().unwrap();
    let report = doctor(&env_for(dir.path()));
    assert_eq!(report.tools.len(), 5);
    assert!(report.tools.iter().all(|t| !t.found), "{:#?}", report.tools);
    assert!(!report.grace_home_exists);
    let claude = &report.tools[0];
    assert_eq!(claude.tool, ToolId::ClaudeCode);
    assert_eq!(claude.sessions, None);
    assert!(claude.data_dir.as_deref().unwrap().ends_with("projects"));
}

#[test]
fn doctor_finds_claude_codex_antigravity() {
    let dir = tempfile::tempdir().unwrap();
    let env = env_for(dir.path());
    let project = env.claude_home.join("projects").join("p");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("a.jsonl"),
        "{\"type\":\"user\",\"version\":\"2.1.330\"}\n",
    )
    .unwrap();
    fs::create_dir_all(env.codex_home.join("sessions/2026/11/27")).unwrap();
    fs::create_dir_all(env.gemini_home.join("antigravity")).unwrap();

    let report = doctor(&env);
    let found: Vec<_> = report
        .tools
        .iter()
        .filter(|t| t.found)
        .map(|t| t.tool)
        .collect();
    assert_eq!(
        found,
        [ToolId::ClaudeCode, ToolId::Codex, ToolId::Antigravity]
    );
    assert_eq!(report.tools[0].sessions, Some(1));
    assert_eq!(report.tools[0].version.as_deref(), Some("2.1.330"));
    assert_eq!(report.tools[1].sessions, Some(0));
}

#[test]
fn doctor_never_opens_credential_files() {
    let dir = tempfile::tempdir().unwrap();
    let env = env_for(dir.path());
    let sessions = env.codex_home.join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    // Named like a rollout, but it's a credential file: discovery must skip its content.
    fs::write(env.codex_home.join("auth.json"), "{\"token\":\"SENTINEL\"}").unwrap();
    let report = doctor(&env);
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("SENTINEL"));
}
