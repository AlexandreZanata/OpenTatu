-- 0006_findings: the investigation (spec §A.6, §B.7, §C.7 steps 7–14, §E.4).
--
-- hypotheses and findings are DERIVED state: rows change (verdict, skeptic review, last
-- detection) but every change is written together with its event (ADR-0006), so the
-- state can be rebuilt from the event log. hunt_rounds and audit_reports are append-only.

CREATE TABLE hypotheses (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    hypothesis_id TEXT NOT NULL UNIQUE,
    audit_id      TEXT NOT NULL REFERENCES audits (id),
    fingerprint   TEXT NOT NULL,
    state         TEXT NOT NULL,
    finding_id    TEXT,
    created_at    TEXT NOT NULL,
    content       TEXT NOT NULL CHECK (json_valid(content))
) STRICT;

CREATE INDEX hypotheses_by_audit ON hypotheses (audit_id, id);
CREATE INDEX hypotheses_by_fingerprint ON hypotheses (fingerprint);

CREATE TABLE findings (
    finding_id     TEXT PRIMARY KEY,
    app            TEXT NOT NULL REFERENCES apps (slug),
    number         INTEGER NOT NULL CHECK (number >= 1),
    state          TEXT NOT NULL,
    severity       TEXT NOT NULL,
    fingerprint    TEXT NOT NULL,
    first_audit_id TEXT NOT NULL REFERENCES audits (id),
    last_audit_id  TEXT NOT NULL REFERENCES audits (id),
    content        TEXT NOT NULL CHECK (json_valid(content)),
    UNIQUE (app, number)
) STRICT;

CREATE INDEX findings_by_fingerprint ON findings (app, fingerprint);

-- Finding numbers are never reused (spec §B.2), even if a row could ever go away.
CREATE TRIGGER findings_no_delete BEFORE DELETE ON findings
BEGIN
    SELECT RAISE(ABORT, 'R04: findings are never deleted (dismissal is a state)');
END;

CREATE TABLE hunt_rounds (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_id   TEXT NOT NULL REFERENCES audits (id),
    round      INTEGER NOT NULL CHECK (round >= 1),
    created_at TEXT NOT NULL,
    content    TEXT NOT NULL CHECK (json_valid(content)),
    UNIQUE (audit_id, round)
) STRICT;

CREATE TABLE audit_reports (
    audit_id   TEXT PRIMARY KEY REFERENCES audits (id),
    created_at TEXT NOT NULL,
    content    TEXT NOT NULL CHECK (json_valid(content))
) STRICT;

CREATE TRIGGER hunt_rounds_no_update BEFORE UPDATE ON hunt_rounds
BEGIN
    SELECT RAISE(ABORT, 'R04: hunt_rounds is append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER hunt_rounds_no_delete BEFORE DELETE ON hunt_rounds
BEGIN
    SELECT RAISE(ABORT, 'R04: hunt_rounds is append-only (DELETE is forbidden)');
END;

CREATE TRIGGER audit_reports_no_update BEFORE UPDATE ON audit_reports
BEGIN
    SELECT RAISE(ABORT, 'R04: audit_reports is append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER audit_reports_no_delete BEFORE DELETE ON audit_reports
BEGIN
    SELECT RAISE(ABORT, 'R04: audit_reports is append-only (DELETE is forbidden)');
END;
