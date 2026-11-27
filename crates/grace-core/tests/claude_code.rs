//! Claude Code adapter v0 against synthetic fixtures shaped like 2.1.330 transcripts.

use grace_core::adapters::{Adapter, ClaudeCode, ParseOutcome, ParseState, Parsed, SessionRef};
use grace_core::paths::is_credential_file;
use grace_core::{Boundary, BoundaryKind, Event, EventKind, Redactor, ToolId};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude_code")
        .join(name)
}

fn session_ref(path: &Path) -> SessionRef {
    let meta = fs::metadata(path).unwrap();
    SessionRef {
        tool: ToolId::ClaudeCode,
        session_id: path.file_stem().unwrap().to_string_lossy().into_owned(),
        path: path.to_path_buf(),
        size: meta.len(),
        mtime_ms: None,
    }
}

struct Run {
    events: Vec<Event>,
    boundaries: Vec<Boundary>,
    outcome: ParseOutcome,
}

fn parse_with(path: &Path, state: Option<ParseState>) -> Run {
    let adapter = ClaudeCode::new(path.parent().unwrap().to_path_buf());
    let (mut events, mut boundaries) = (Vec::new(), Vec::new());
    let mut sink = |item: Parsed| -> anyhow::Result<()> {
        match item {
            Parsed::Event(event) => events.push(event),
            Parsed::Boundary(boundary) => boundaries.push(boundary),
        }
        Ok(())
    };
    let outcome = adapter
        .parse(&session_ref(path), state, &Redactor::default(), &mut sink)
        .unwrap();
    Run {
        events,
        boundaries,
        outcome,
    }
}

fn parse(path: &Path) -> Run {
    parse_with(path, None)
}

fn write_temp(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
    let path = dir.join(name);
    let mut text = lines.join("\n");
    text.push('\n');
    fs::write(&path, text).unwrap();
    path
}

/// A readable dump for snapshots: context changes, events, then session info.
fn render(run: &Run) -> String {
    let mut out = String::new();
    for b in &run.boundaries {
        out += &format!(
            "boundary {:<13} seq {:>2} line {:>2} {}\n",
            b.kind.as_str(),
            b.event_seq,
            b.line_no,
            b.detail
        );
    }
    for e in &run.events {
        let turn = e.turn.map_or_else(|| "-".to_string(), |t| t.to_string());
        out += &format!(
            "{:>2} L{:<2} t{:<2} {:<17} {:<34} {}\n",
            e.seq,
            e.line_no,
            turn,
            e.kind.as_str(),
            e.label,
            e.summary
        );
    }
    let info = &run.outcome.state.info;
    out += &format!(
        "info: title={:?} turns={} compactions={} models={:?}\n",
        info.title, info.turns, info.compactions, info.models
    );
    out += &format!(
        "info: version={:?} entrypoint={:?} branch={:?} project={:?}\n",
        info.version, info.entrypoint, info.git_branch, info.project_path
    );
    out += &format!(
        "info: started={:?} last={:?}\n",
        info.started_at, info.last_active_at
    );
    out += &format!(
        "info: first_prompt={:?} last_prompt={:?}\n",
        info.first_prompt, info.last_prompt
    );
    let stats = &run.outcome.stats;
    out += &format!(
        "stats: lines={} bad={} ignored={} unknown={:?}\n",
        stats.lines, stats.bad_lines, stats.ignored, stats.unknown
    );
    out
}

#[test]
fn parses_basic_fixture() {
    let run = parse(&fixture("basic.jsonl"));
    insta::assert_snapshot!(render(&run));

    let info = &run.outcome.state.info;
    assert_eq!(info.title.as_deref(), Some("Fix the sprite sheet"));
    assert_eq!(info.turns, 2);
    assert_eq!(
        info.project_path.as_deref(),
        Some(r"C:\Users\Test User\Projects\Maple & Moss")
    );
    let loads: Vec<_> = run
        .events
        .iter()
        .filter(|e| e.kind == EventKind::InstructionLoad)
        .map(|e| e.detail["type"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(loads, ["Project", "AutoMem"]);
    assert_eq!(run.outcome.stats.unknown.get("future-thing"), Some(&1));
    assert_eq!(run.boundaries[0].kind, BoundaryKind::SessionStart);
    assert_eq!(run.boundaries[0].event_seq, 0);
    assert!(run.events.iter().all(|e| e.line_no >= 1));
}

#[test]
fn detects_compaction_boundary() {
    let run = parse(&fixture("compaction.jsonl"));
    let kinds: Vec<_> = run.boundaries.iter().map(|b| (b.kind, b.line_no)).collect();
    assert_eq!(
        kinds,
        [
            (BoundaryKind::SessionStart, 1),
            (BoundaryKind::Compaction, 3)
        ]
    );
    let detail = &run.boundaries[1].detail;
    assert_eq!(detail["trigger"], "auto");
    assert_eq!(detail["pre_tokens"], 150_000);
    assert_eq!(detail["post_tokens"], 12_000);
    assert_eq!(run.outcome.state.info.compactions, 1);
    assert_eq!(run.outcome.state.info.turns, 2);
    insta::assert_snapshot!(render(&run));
}

#[test]
fn detects_model_switch() {
    let run = parse(&fixture("model_switch.jsonl"));
    let switch = run
        .boundaries
        .iter()
        .find(|b| b.kind == BoundaryKind::ModelSwitch)
        .expect("a model switch");
    assert_eq!(switch.detail["from"], "claude-opus-5-5");
    assert_eq!(switch.detail["to"], "claude-sonnet-5-5");
    assert_eq!(switch.line_no, 5);
    assert_eq!(
        run.outcome.state.info.models,
        ["claude-opus-5-5", "claude-sonnet-5-5"]
    );
}

/// The resume marker hasn't been seen on a real machine yet (spec §7.6).
#[test]
fn detects_resume_unverified() {
    let run = parse(&fixture("resume.jsonl"));
    let resume = run
        .boundaries
        .iter()
        .find(|b| b.kind == BoundaryKind::Resume)
        .expect("a resume");
    assert_eq!(resume.detail["via"], "resume");
    assert_eq!(
        resume.detail["from_session"],
        "33333333-3333-4333-8333-333333333333"
    );
}

#[test]
fn crlf_matches_lf() {
    let dir = tempfile::tempdir().unwrap();
    let lf = fs::read_to_string(fixture("basic.jsonl")).unwrap();
    let path = dir.path().join("crlf.jsonl");
    fs::write(&path, lf.replace("\r\n", "\n").replace('\n', "\r\n")).unwrap();
    let a = parse(&fixture("basic.jsonl"));
    let b = parse(&path);
    assert_eq!(a.events, b.events);
    assert_eq!(a.boundaries, b.boundaries);
}

#[test]
fn incremental_parse_resumes_without_duplicates() {
    let full = fs::read(fixture("basic.jsonl")).unwrap();
    // Cut 20 bytes into line 8, as if Claude Code were still writing it.
    let seventh_newline = full
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'\n')
        .nth(6)
        .unwrap()
        .0;
    let cut = seventh_newline + 20;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("growing.jsonl");
    fs::write(&path, &full[..cut]).unwrap();

    let first = parse(&path);
    assert_eq!(first.outcome.state.line_no, 7);

    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&full[cut..])
        .unwrap();
    let second = parse_with(&path, Some(first.outcome.state.clone()));
    let whole = parse(&fixture("basic.jsonl"));

    let mut events = first.events.clone();
    events.extend(second.events.clone());
    assert_eq!(events, whole.events);
    let mut boundaries = first.boundaries.clone();
    boundaries.extend(second.boundaries.clone());
    assert_eq!(boundaries, whole.boundaries);
    let mut expected = whole.outcome.state.info.clone();
    expected.session_id = "growing".to_string(); // the id comes from the file name
    assert_eq!(second.outcome.state.info, expected);
}

#[test]
fn redacts_secrets_on_ingest() {
    let run = parse(&fixture("secrets.jsonl"));
    let dump = serde_json::to_string(&run.events).unwrap()
        + &serde_json::to_string(&run.outcome.state).unwrap();
    for secret in [
        "sk-ant-api03",
        "sk-proj-",
        "ghp_0123",
        "test.user@example.com",
    ] {
        assert!(!dump.contains(secret), "leaked {secret}");
    }
    assert!(dump.contains("[redacted:api-key]"));
    assert!(dump.contains("[redacted:token]"));
}

#[test]
fn survives_garbage_lines() {
    let dir = tempfile::tempdir().unwrap();
    let good = fs::read_to_string(fixture("basic.jsonl")).unwrap();
    let first_prompt = good.lines().nth(2).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"not json at all\n\n{\"type\": \n");
    bytes.extend_from_slice(
        b"{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"caf\xff\xfe\"},\"origin\":{\"kind\":\"human\"}}\n",
    );
    bytes.extend_from_slice(first_prompt.as_bytes());
    bytes.push(b'\n');
    let path = dir.path().join("garbage.jsonl");
    fs::write(&path, bytes).unwrap();

    let run = parse(&path);
    assert_eq!(run.outcome.stats.bad_lines, 2);
    let prompts = run
        .events
        .iter()
        .filter(|e| e.label == "user/prompt")
        .count();
    assert_eq!(prompts, 2);
}

#[test]
fn task_updates_from_three_tools() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp(
        dir.path(),
        "tasks.jsonl",
        &[
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5","role":"assistant","content":[{"type":"tool_use","id":"t1","name":"mcp__ccd_turn__update_plan","input":{"items":[{"content":"a","status":"completed"},{"content":"b","status":"in_progress"}]}}]}}"#,
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5","role":"assistant","content":[{"type":"tool_use","id":"t2","name":"TodoWrite","input":{"todos":[{"content":"a","status":"pending"}]}}]}}"#,
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5","role":"assistant","content":[{"type":"tool_use","id":"t3","name":"TaskCreate","input":{"subject":"Write docs","description":"For the CLI"}}]}}"#,
        ],
    );
    let run = parse(&path);
    let tasks: Vec<_> = run
        .events
        .iter()
        .filter(|e| e.kind == EventKind::TaskUpdate)
        .map(|e| e.summary.clone())
        .collect();
    assert_eq!(
        tasks,
        [
            "Task list: 2 items, 1 done, 1 in progress",
            "Task list: 1 item, 0 done, 0 in progress",
            "Task added: Write docs",
        ]
    );
}

#[test]
fn discovers_top_level_sessions_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("proj-a/aaa/subagents")).unwrap();
    fs::create_dir_all(root.join("proj b & c")).unwrap();
    fs::write(root.join("proj-a/aaa.jsonl"), "{}\n").unwrap();
    fs::write(root.join("proj-a/aaa/subagents/sub.jsonl"), "{}\n").unwrap();
    fs::write(root.join("proj b & c/bbb.jsonl"), "{}\n").unwrap();
    fs::write(root.join("proj b & c/notes.txt"), "x").unwrap();
    fs::File::options()
        .write(true)
        .open(root.join("proj-a/aaa.jsonl"))
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();

    let found = ClaudeCode::new(root.to_path_buf()).discover().unwrap();
    let ids: Vec<_> = found.iter().map(|s| s.session_id.as_str()).collect();
    assert_eq!(ids, ["bbb", "aaa"]);
    assert!(
        found
            .iter()
            .all(|s| s.tool == ToolId::ClaudeCode && s.size == 3)
    );
}

#[test]
fn missing_folder_means_no_sessions() {
    let found = ClaudeCode::new(PathBuf::from("definitely/not/here"))
        .discover()
        .unwrap();
    assert!(found.is_empty());
}

#[test]
fn never_reads_credential_files() {
    assert!(is_credential_file(Path::new(".codex/auth.json")));
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp(dir.path(), "auth.json", &["{}"]);
    let err = ClaudeCode::new(dir.path().to_path_buf())
        .parse(&session_ref(&path), None, &Redactor::default(), &mut |_| {
            Ok(())
        })
        .unwrap_err();
    assert!(err.to_string().contains("credential"), "{err}");
}

#[test]
#[ignore = "speed check: cargo test --release -p grace-core -- --ignored"]
fn parses_100mb_under_10s() {
    use std::io::BufWriter;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.jsonl");
    let template = fs::read_to_string(fixture("basic.jsonl")).unwrap();
    let big_output = "lorem ipsum dolor sit amet ".repeat(150);
    let mut file = BufWriter::new(fs::File::create(&path).unwrap());
    let mut written = 0usize;
    let mut block = 0usize;
    while written < 100 * 1024 * 1024 {
        for line in template.lines() {
            writeln!(file, "{line}").unwrap();
            written += line.len() + 1;
        }
        let extra = [
            format!(
                r#"{{"type":"api-request-blob","sessionId":"s","hash":"h{block}","message":{{"role":"user","content":"{big_output}"}}}}"#
            ),
            format!(
                r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"tool_result","tool_use_id":"toolu_0001","content":"{big_output}"}}]}},"uuid":"r{block}"}}"#
            ),
        ];
        for line in extra {
            writeln!(file, "{line}").unwrap();
            written += line.len() + 1;
        }
        block += 1;
    }
    drop(file);

    let start = std::time::Instant::now();
    let mut items = 0u64;
    ClaudeCode::new(dir.path().to_path_buf())
        .parse(&session_ref(&path), None, &Redactor::default(), &mut |_| {
            items += 1;
            Ok(())
        })
        .unwrap();
    let elapsed = start.elapsed();
    eprintln!("parsed {written} bytes into {items} items in {elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "took {elapsed:?}");
}
