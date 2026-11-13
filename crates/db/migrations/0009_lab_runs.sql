-- Dynamic attempts are decided once; approval remains an append-only OWNER_DECISION.
CREATE TABLE lab_runs (
    audit_id TEXT NOT NULL REFERENCES audits(id),
    number INTEGER NOT NULL CHECK(number >= 1),
    hypothesis_id TEXT NOT NULL REFERENCES hypotheses(hypothesis_id),
    completed_at TEXT,
    content TEXT NOT NULL CHECK(json_valid(content)),
    PRIMARY KEY(audit_id, number)
) STRICT;
CREATE TRIGGER lab_runs_no_delete BEFORE DELETE ON lab_runs
BEGIN SELECT RAISE(ABORT, 'R04: lab attempts are never deleted'); END;
CREATE TRIGGER lab_runs_decided_once BEFORE UPDATE ON lab_runs
WHEN OLD.completed_at IS NOT NULL OR NEW.audit_id != OLD.audit_id
    OR NEW.number != OLD.number OR NEW.hypothesis_id != OLD.hypothesis_id
    OR NEW.completed_at IS NULL
    OR json_remove(NEW.content, '$.completed_at', '$.execution')
        != json_remove(OLD.content, '$.completed_at', '$.execution')
    OR json_extract(NEW.content, '$.completed_at') != NEW.completed_at
    OR json_type(NEW.content, '$.execution') != 'object'
BEGIN SELECT RAISE(ABORT, 'R04: lab results and identity are immutable'); END;
