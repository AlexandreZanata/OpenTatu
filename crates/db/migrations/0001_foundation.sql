-- 0001_foundation: phase 1 tables (apps, snapshots, audits) and the append-only event log
-- (spec §B.1, §B.6, §E.4; rule R04).
--
-- NEVER edit a migration that was already applied somewhere: add a new numbered file
-- instead (docs/recipes/add-migration.md). sqlx verifies checksums and will refuse to start.
--
-- Conventions: STRICT tables; timestamps are ISO 8601 TEXT with offset (spec §B.3);
-- JSON columns are TEXT guarded by json_valid(); enum values are validated in Rust
-- (opentatu-core is their single source), so there are no enum CHECK constraints here.

CREATE TABLE apps (
    slug           TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    repo_url       TEXT NOT NULL,
    branch         TEXT NOT NULL,
    stack          TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(stack)),
    sensitive_data TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(sensitive_data)),
    lab_url        TEXT,
    created_at     TEXT NOT NULL
) STRICT;

CREATE TABLE snapshots (
    id          INTEGER PRIMARY KEY,
    app         TEXT NOT NULL REFERENCES apps (slug),
    -- R02: always the full commit hash (40 hex for SHA-1, 64 for SHA-256 repositories).
    commit_hash TEXT NOT NULL CHECK (length(commit_hash) IN (40, 64)),
    captured_at TEXT NOT NULL,
    UNIQUE (app, commit_hash)
) STRICT;

CREATE TABLE audits (
    id                TEXT PRIMARY KEY,
    app               TEXT NOT NULL REFERENCES apps (slug),
    kind              TEXT NOT NULL,
    status            TEXT NOT NULL,
    commit_hash       TEXT NOT NULL CHECK (length(commit_hash) IN (40, 64)),
    branch            TEXT,
    previous_audit_id TEXT REFERENCES audits (id),
    created_at        TEXT NOT NULL,
    started_at        TEXT,
    completed_at      TEXT,
    budget            TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(budget)),
    usage             TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(usage)),
    summary           TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(summary))
) STRICT;

CREATE INDEX audits_by_app ON audits (app, created_at);

-- Source of truth (spec §B.1). The text ID is EVT-<id zero-padded to 8 digits>.
CREATE TABLE events (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    kind                TEXT NOT NULL,
    app                 TEXT REFERENCES apps (slug),
    audit_id            TEXT REFERENCES audits (id),
    occurred_at         TEXT NOT NULL,
    actor               TEXT NOT NULL,
    target              TEXT,
    data                TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(data)),
    model               TEXT,
    prompt_version      TEXT,
    references_event_id INTEGER REFERENCES events (id)
) STRICT;

CREATE INDEX events_by_app ON events (app, id);
CREATE INDEX events_by_audit ON events (audit_id, id);
CREATE INDEX events_by_target ON events (target, id);

-- R04: the event log is append-only. Corrections are new events (references_event_id).
CREATE TRIGGER events_no_update BEFORE UPDATE ON events
BEGIN
    SELECT RAISE(ABORT, 'R04: events are append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER events_no_delete BEFORE DELETE ON events
BEGIN
    SELECT RAISE(ABORT, 'R04: events are append-only (DELETE is forbidden)');
END;
