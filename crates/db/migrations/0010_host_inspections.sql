-- Q-035 / ADR-0035: live host inventory, separate from commit-scoped audits (R03/R04).
CREATE TABLE host_inspections (
    app TEXT NOT NULL REFERENCES apps(slug),
    number INTEGER NOT NULL CHECK(number >= 1 AND number <= 4294967295),
    profile TEXT NOT NULL,
    profile_digest TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    PRIMARY KEY(app, number)
) STRICT;
CREATE TRIGGER host_inspections_no_delete BEFORE DELETE ON host_inspections
BEGIN SELECT RAISE(ABORT, 'R04: host inspections are never deleted'); END;
CREATE TRIGGER host_inspections_decided_once BEFORE UPDATE ON host_inspections
WHEN OLD.completed_at IS NOT NULL OR NEW.completed_at IS NULL
    OR NEW.app IS NOT OLD.app OR NEW.number IS NOT OLD.number
    OR NEW.profile IS NOT OLD.profile OR NEW.profile_digest IS NOT OLD.profile_digest
    OR NEW.started_at IS NOT OLD.started_at
BEGIN SELECT RAISE(ABORT, 'R04: host inspection identity is immutable and completed once'); END;

CREATE TABLE host_commands (
    app TEXT NOT NULL,
    inspection_number INTEGER NOT NULL,
    command TEXT NOT NULL,
    completed_at TEXT,
    content TEXT NOT NULL CHECK(json_valid(content)),
    PRIMARY KEY(app, inspection_number, command),
    FOREIGN KEY(app, inspection_number) REFERENCES host_inspections(app, number),
    CHECK(json_extract(content, '$.command') IS command),
    CHECK(json_extract(content, '$.completed_at') IS completed_at),
    CHECK((completed_at IS NULL AND json_type(content, '$.result') IS 'null')
        OR (completed_at IS NOT NULL AND json_type(content, '$.result') IS 'object'))
) STRICT;
CREATE TRIGGER host_commands_no_delete BEFORE DELETE ON host_commands
BEGIN SELECT RAISE(ABORT, 'R04: host commands are never deleted'); END;
CREATE TRIGGER host_commands_decided_once BEFORE UPDATE ON host_commands
WHEN OLD.completed_at IS NOT NULL OR NEW.completed_at IS NULL
    OR NEW.app IS NOT OLD.app OR NEW.inspection_number IS NOT OLD.inspection_number
    OR NEW.command IS NOT OLD.command
    OR json_remove(NEW.content, '$.completed_at', '$.result')
        IS NOT json_remove(OLD.content, '$.completed_at', '$.result')
BEGIN SELECT RAISE(ABORT, 'R04: host command identity and completed results are immutable'); END;
