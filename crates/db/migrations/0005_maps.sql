-- 0005_maps: versioned security maps (spec §A.4, §C.7 step 5, §E.4). One map per audit;
-- the content is the validated `opentatu_core::SecurityMap` as JSON. Files under
-- `apps/<slug>/map/` mirror it for the owner and the AI roles.
--
-- Append-only: a map version never changes; the next audit records a new one (R04).

CREATE TABLE maps (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id      TEXT NOT NULL UNIQUE,
    app         TEXT NOT NULL REFERENCES apps (slug),
    audit_id    TEXT NOT NULL UNIQUE REFERENCES audits (id),
    commit_hash TEXT NOT NULL CHECK (length(commit_hash) IN (40, 64)),
    created_at  TEXT NOT NULL,
    content     TEXT NOT NULL CHECK (json_valid(content))
) STRICT;

CREATE INDEX maps_by_app ON maps (app, id);

CREATE TRIGGER maps_no_update BEFORE UPDATE ON maps
BEGIN
    SELECT RAISE(ABORT, 'R04: maps is append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER maps_no_delete BEFORE DELETE ON maps
BEGIN
    SELECT RAISE(ABORT, 'R04: maps is append-only (DELETE is forbidden)');
END;
