-- 0007_tasks: the fix trail (spec §A.6, §C.5, §E.4, §E.6; R06, R09, R19; ADR-0012).
--
-- tasks and verifications are DERIVED state: rows change (claim, fix, decision) but every
-- change is written together with its event (ADR-0006). agents keeps only the hash of each
-- MCP token, never the token (R16).

CREATE TABLE tasks (
    task_id    TEXT PRIMARY KEY,
    finding_id TEXT NOT NULL REFERENCES findings (finding_id),
    app        TEXT NOT NULL REFERENCES apps (slug),
    audit_id   TEXT NOT NULL REFERENCES audits (id),
    number     INTEGER NOT NULL CHECK (number >= 1),
    state      TEXT NOT NULL,
    priority   INTEGER NOT NULL CHECK (priority BETWEEN 1 AND 5),
    claimed_by TEXT,
    branch     TEXT,
    fix_commit TEXT,
    created_at TEXT NOT NULL,
    content    TEXT NOT NULL CHECK (json_valid(content)),
    UNIQUE (finding_id, number)
) STRICT;

CREATE INDEX tasks_by_app ON tasks (app, state, priority);
CREATE INDEX tasks_by_audit ON tasks (audit_id);

-- Task numbers are never reused (spec §B.2).
CREATE TRIGGER tasks_no_delete BEFORE DELETE ON tasks
BEGIN
    SELECT RAISE(ABORT, 'R04: tasks are never deleted (a rejected fix reopens the task)');
END;

-- One row per submitted fix; its REVERIFICATION audit is the key. `result` is NULL while
-- the reverification has not decided, and is written exactly once.
CREATE TABLE verifications (
    audit_id   TEXT PRIMARY KEY REFERENCES audits (id),
    task_id    TEXT NOT NULL REFERENCES tasks (task_id),
    result     TEXT CHECK (result IN ('PASSED', 'FAILED')),
    created_at TEXT NOT NULL,
    decided_at TEXT,
    content    TEXT NOT NULL CHECK (json_valid(content))
) STRICT;

CREATE INDEX verifications_by_task ON verifications (task_id, created_at);

CREATE TRIGGER verifications_decided_once BEFORE UPDATE ON verifications
WHEN OLD.result IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'R04: a reverification decision is final (UPDATE is forbidden)');
END;

CREATE TRIGGER verifications_no_delete BEFORE DELETE ON verifications
BEGIN
    SELECT RAISE(ABORT, 'R04: verifications are never deleted');
END;

CREATE TABLE agents (
    name          TEXT PRIMARY KEY,
    token_hash    TEXT NOT NULL UNIQUE,
    registered_at TEXT NOT NULL,
    revoked_at    TEXT
) STRICT;

CREATE TRIGGER agents_no_delete BEFORE DELETE ON agents
BEGIN
    SELECT RAISE(ABORT, 'R04: agents are revoked, never deleted');
END;
