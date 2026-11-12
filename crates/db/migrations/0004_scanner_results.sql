-- 0004_scanner_results: normalized results of the deterministic scanners (spec §C.7 step 4,
-- §E.5). Raw outputs stay in files (`audits/<audit>/scanners/<tool>.json`); run metadata
-- (version, image, duration) lives in the SCANNER_RAN / SCANNER_FAILED events.
--
-- Append-only: the results of an audit never change (R04 spirit).

CREATE TABLE scanner_results (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_id          TEXT NOT NULL REFERENCES audits (id),
    scanner           TEXT NOT NULL,
    result_id         TEXT NOT NULL,
    rule_id           TEXT NOT NULL,
    message           TEXT NOT NULL,
    file              TEXT,
    start_line        INTEGER CHECK (start_line IS NULL OR start_line >= 0),
    end_line          INTEGER CHECK (end_line IS NULL OR end_line >= 0),
    reported_severity TEXT,
    cwe               TEXT,
    UNIQUE (audit_id, result_id)
) STRICT;

CREATE INDEX scanner_results_by_audit ON scanner_results (audit_id, id);

CREATE TRIGGER scanner_results_no_update BEFORE UPDATE ON scanner_results
BEGIN
    SELECT RAISE(ABORT, 'R04: scanner_results is append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER scanner_results_no_delete BEFORE DELETE ON scanner_results
BEGIN
    SELECT RAISE(ABORT, 'R04: scanner_results is append-only (DELETE is forbidden)');
END;
