//! Schema migrations, applied in order and tracked with `PRAGMA user_version`.
//! Never edit a released migration: add a new one.

pub const MIGRATIONS: &[&str] = &[
    // v1 (M0): projects, sessions, events, context changes, format drift.
    r"
    CREATE TABLE projects (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL
    );

    CREATE TABLE sessions (
        id INTEGER PRIMARY KEY,
        tool TEXT NOT NULL,
        session_id TEXT NOT NULL,
        source_file TEXT NOT NULL UNIQUE,
        project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
        title TEXT,
        first_prompt TEXT,
        last_prompt TEXT,
        version TEXT,
        entrypoint TEXT,
        git_branch TEXT,
        models TEXT NOT NULL DEFAULT '[]',
        started_at TEXT,
        last_active_at TEXT,
        turns INTEGER NOT NULL DEFAULT 0,
        compactions INTEGER NOT NULL DEFAULT 0,
        event_count INTEGER NOT NULL DEFAULT 0,
        bad_lines INTEGER NOT NULL DEFAULT 0,
        file_size INTEGER NOT NULL DEFAULT 0,
        file_mtime_ms INTEGER,
        head_hash TEXT,
        parse_state TEXT,
        scanned_at TEXT
    );
    CREATE INDEX sessions_last_active ON sessions(last_active_at DESC);
    CREATE INDEX sessions_project ON sessions(project_id);

    CREATE TABLE events (
        session_ref INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        seq INTEGER NOT NULL,
        kind TEXT NOT NULL,
        label TEXT NOT NULL,
        line_no INTEGER NOT NULL,
        turn INTEGER,
        ts TEXT,
        uuid TEXT,
        parent_uuid TEXT,
        model TEXT,
        summary TEXT NOT NULL,
        detail TEXT NOT NULL DEFAULT '{}',
        PRIMARY KEY (session_ref, seq)
    ) WITHOUT ROWID;
    CREATE INDEX events_kind ON events(session_ref, kind);

    CREATE TABLE boundaries (
        session_ref INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        idx INTEGER NOT NULL,
        kind TEXT NOT NULL,
        event_seq INTEGER NOT NULL,
        line_no INTEGER NOT NULL,
        ts TEXT,
        detail TEXT NOT NULL DEFAULT '{}',
        PRIMARY KEY (session_ref, idx)
    ) WITHOUT ROWID;

    CREATE TABLE drift (
        tool TEXT NOT NULL,
        label TEXT NOT NULL,
        version TEXT NOT NULL DEFAULT '',
        count INTEGER NOT NULL DEFAULT 0,
        first_seen TEXT NOT NULL,
        last_seen TEXT NOT NULL,
        PRIMARY KEY (tool, label, version)
    ) WITHOUT ROWID;
    ",
];
