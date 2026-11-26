//! Scanning real files: skipping, incremental reads, resets and removals.

use grace_core::Redactor;
use grace_core::adapters::{Adapter, ClaudeCode};
use grace_core::scan::{ScanReport, scan};
use grace_core::store::{SessionQuery, Store};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn fixture_text(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude_code")
        .join(name);
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

struct Setup {
    _dir: tempfile::TempDir,
    root: PathBuf,
    file: PathBuf,
    store: Store,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projects");
    let project = root.join("C--Users-Test-User-Projects-Fern---Fable");
    fs::create_dir_all(&project).unwrap();
    let file = project.join("11111111-1111-4111-8111-111111111111.jsonl");
    let store = Store::open(&dir.path().join("grace home").join("grace.db")).unwrap();
    Setup {
        _dir: dir,
        root,
        file,
        store,
    }
}

impl Setup {
    fn run(&mut self) -> ScanReport {
        let adapters: Vec<Box<dyn Adapter>> = vec![Box::new(ClaudeCode::new(self.root.clone()))];
        let report = scan(
            &mut self.store,
            &adapters,
            &Redactor::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        report
    }

    fn event_count(&self) -> u64 {
        self.store.list_sessions(&SessionQuery::default()).unwrap()[0].event_count
    }
}

fn first_lines(text: &str, count: usize) -> String {
    text.lines()
        .take(count)
        .map(|line| format!("{line}\n"))
        .collect()
}

#[test]
fn rescan_skips_unchanged() {
    let mut s = setup();
    fs::write(&s.file, fixture_text("basic.jsonl")).unwrap();
    let first = s.run();
    assert_eq!(
        (first.sessions_found, first.parsed, first.skipped),
        (1, 1, 0)
    );
    assert_eq!(first.events_added, 17);
    let second = s.run();
    assert_eq!(
        (second.parsed, second.skipped, second.events_added),
        (0, 1, 0)
    );
    assert_eq!(s.event_count(), 17);
}

#[test]
fn rescan_after_append_is_incremental() {
    let mut s = setup();
    let full = fixture_text("basic.jsonl");
    fs::write(&s.file, first_lines(&full, 9)).unwrap();
    assert_eq!(s.run().events_added, 12);

    let rest: String = full
        .lines()
        .skip(9)
        .map(|line| format!("{line}\n"))
        .collect();
    fs::OpenOptions::new()
        .append(true)
        .open(&s.file)
        .unwrap()
        .write_all(rest.as_bytes())
        .unwrap();
    assert_eq!(s.run().events_added, 5);
    assert_eq!(s.event_count(), 17);

    let id = s.store.list_sessions(&SessionQuery::default()).unwrap()[0].id;
    let detail = s.store.session_detail(id, None, 100).unwrap().unwrap();
    let seqs: Vec<u64> = detail.events.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (0..17).collect::<Vec<_>>());
    assert_eq!(detail.boundaries.len(), 1, "one session start, not two");
    assert_eq!(
        detail.session.title.as_deref(),
        Some("Fix the sprite sheet")
    );
}

#[test]
fn truncated_file_resets_session() {
    let mut s = setup();
    let full = fixture_text("basic.jsonl");
    fs::write(&s.file, &full).unwrap();
    s.run();
    fs::write(&s.file, first_lines(&full, 3)).unwrap();
    s.run();
    assert_eq!(s.event_count(), 4);
}

#[test]
fn replaced_file_is_rescanned_from_the_start() {
    let mut s = setup();
    fs::write(&s.file, fixture_text("basic.jsonl")).unwrap();
    s.run();
    // Different content, larger than before: must not resume mid-file.
    let replacement = fixture_text("compaction.jsonl").repeat(3);
    fs::write(&s.file, replacement).unwrap();
    s.run();
    let row = &s.store.list_sessions(&SessionQuery::default()).unwrap()[0];
    assert_eq!(row.title.as_deref(), Some("Build the level editor"));
    assert_eq!(row.event_count, 21);
}

#[test]
fn removed_file_removes_session() {
    let mut s = setup();
    fs::write(&s.file, fixture_text("basic.jsonl")).unwrap();
    s.run();
    fs::remove_file(&s.file).unwrap();
    let report = s.run();
    assert_eq!(report.removed, 1);
    assert!(
        s.store
            .list_sessions(&SessionQuery::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unknown_types_are_recorded_as_drift() {
    let mut s = setup();
    fs::write(&s.file, fixture_text("basic.jsonl")).unwrap();
    let report = s.run();
    assert_eq!(report.unknown.get("future-thing"), Some(&1));
    let drift = s.store.drift().unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(
        (drift[0].label.as_str(), drift[0].version.as_str()),
        ("future-thing", "2.1.330")
    );
}
