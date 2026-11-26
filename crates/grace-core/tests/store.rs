//! The SQLite store: migrations, writes, queries and drift records.

use grace_core::adapters::{ParseOutcome, ParseState, ParseStats, Parsed, SessionRef};
use grace_core::store::{SessionQuery, Store};
use grace_core::{Boundary, BoundaryKind, Event, EventKind, ToolId};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

const PROJECT: &str = r"C:\Users\Test User\Projects\Fern & Fable";

fn session(path: &str) -> SessionRef {
    SessionRef {
        tool: ToolId::ClaudeCode,
        session_id: "s1".into(),
        path: PathBuf::from(path),
        size: 100,
        mtime_ms: Some(1),
    }
}

fn event(seq: u64) -> Event {
    Event {
        seq,
        kind: EventKind::UserMessage,
        label: "user/prompt".into(),
        line_no: seq + 1,
        turn: Some(1),
        timestamp: Some(format!("2026-11-23T07:45:0{}.000Z", seq % 10)),
        uuid: None,
        parent_uuid: None,
        model: None,
        summary: format!("prompt {seq}"),
        detail: json!({}),
    }
}

fn outcome(title: &str, next_seq: u64) -> ParseOutcome {
    let mut state = ParseState::new("s1");
    state.seq = next_seq;
    state.info.project_path = Some(PROJECT.into());
    state.info.title = Some(title.into());
    state.info.first_prompt = Some("Fix the sprite sheet loader".into());
    state.info.turns = 1;
    state.info.last_active_at = Some("2026-11-23T07:46:00.000Z".into());
    ParseOutcome {
        state,
        stats: ParseStats::default(),
    }
}

fn write(store: &mut Store, path: &str, events: u64) -> u64 {
    store
        .write_session(&session(path), Some("4:abcd"), false, |writer| {
            writer.push(Parsed::Boundary(Boundary {
                kind: BoundaryKind::SessionStart,
                event_seq: 0,
                line_no: 1,
                timestamp: None,
                detail: json!({}),
            }))?;
            for seq in 0..events {
                writer.push(Parsed::Event(event(seq)))?;
            }
            Ok(outcome("Fix the sprite sheet", events))
        })
        .unwrap()
        .events_added
}

#[test]
fn migrations_are_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new folder").join("grace.db");
    {
        let store = Store::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), 1);
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 1);
}

#[test]
fn write_and_list_roundtrip() {
    let mut store = Store::open_in_memory().unwrap();
    assert_eq!(write(&mut store, "/tmp/a.jsonl", 2), 2);
    let rows = store.list_sessions(&SessionQuery::default()).unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.title.as_deref(), Some("Fix the sprite sheet"));
    assert_eq!(row.event_count, 2);
    assert_eq!(row.turns, 1);
    assert_eq!(row.tool, ToolId::ClaudeCode);
    let stored = store.stored("/tmp/a.jsonl").unwrap().expect("stored");
    assert_eq!(stored.file_size, 100);
    assert_eq!(stored.state.unwrap().seq, 2);
}

#[test]
fn paths_with_spaces_roundtrip() {
    let mut store = Store::open_in_memory().unwrap();
    let file = r"C:\Users\Test User\.claude\projects\C--Fern---Fable\s1.jsonl";
    write(&mut store, file, 1);
    let rows = store.list_sessions(&SessionQuery::default()).unwrap();
    assert_eq!(rows[0].project_path.as_deref(), Some(PROJECT));
    assert_eq!(rows[0].project_name.as_deref(), Some("Fern & Fable"));
    assert_eq!(rows[0].source_file, file);
    assert!(store.stored(file).unwrap().is_some());
    let by_project = store
        .list_sessions(&SessionQuery {
            project: Some(PROJECT.into()),
            ..SessionQuery::default()
        })
        .unwrap();
    assert_eq!(by_project.len(), 1);
}

#[test]
fn search_matches_title_prompt_and_project() {
    let mut store = Store::open_in_memory().unwrap();
    write(&mut store, "/tmp/a.jsonl", 1);
    let find = |q: &str| {
        store
            .list_sessions(&SessionQuery {
                q: Some(q.into()),
                ..SessionQuery::default()
            })
            .unwrap()
            .len()
    };
    assert_eq!(find("sprite"), 1);
    assert_eq!(find("LOADER"), 1);
    assert_eq!(find("fern &"), 1);
    assert_eq!(find("nothing like this"), 0);
    assert_eq!(find("50%_off"), 0);
}

#[test]
fn detail_pages_events() {
    let mut store = Store::open_in_memory().unwrap();
    write(&mut store, "/tmp/a.jsonl", 5);
    let id = store.list_sessions(&SessionQuery::default()).unwrap()[0].id;
    let seqs = |after: Option<u64>| {
        let detail = store.session_detail(id, after, 2).unwrap().unwrap();
        (
            detail.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
            detail.next_after,
            detail.boundaries.len(),
        )
    };
    assert_eq!(seqs(None), (vec![0, 1], Some(1), 1));
    assert_eq!(seqs(Some(1)), (vec![2, 3], Some(3), 1));
    assert_eq!(seqs(Some(3)), (vec![4], None, 1));
    assert!(store.session_detail(999, None, 10).unwrap().is_none());
}

#[test]
fn removing_missing_sessions_cascades() {
    let mut store = Store::open_in_memory().unwrap();
    write(&mut store, "/tmp/a.jsonl", 3);
    assert_eq!(
        store
            .remove_missing(ToolId::ClaudeCode, &["/tmp/b.jsonl".into()])
            .unwrap(),
        1
    );
    assert!(
        store
            .list_sessions(&SessionQuery::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn drift_is_recorded() {
    let store = Store::open_in_memory().unwrap();
    let unknown = BTreeMap::from([("future-thing".to_string(), 2u64)]);
    store
        .record_drift(ToolId::ClaudeCode, Some("2.1.330"), &unknown)
        .unwrap();
    store
        .record_drift(ToolId::ClaudeCode, Some("2.1.330"), &unknown)
        .unwrap();
    let rows = store.drift().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "future-thing");
    assert_eq!(rows[0].count, 4);
    assert_eq!(rows[0].version, "2.1.330");
}
