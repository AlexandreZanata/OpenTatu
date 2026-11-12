-- 0002_llm_usage: accounting of every model call (spec §A.2, §D.2, §E.4; rule R17).
--
-- Never stores the API key or any prompt/response content (R16). Append-only, like the
-- event log: usage is history used for budgets (60% monthly plan share) and reports.

CREATE TABLE llm_usage (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_id         TEXT REFERENCES audits (id),
    role             TEXT,
    model            TEXT NOT NULL,
    input_tokens     INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens    INTEGER NOT NULL CHECK (output_tokens >= 0),
    reasoning_tokens INTEGER NOT NULL DEFAULT 0 CHECK (reasoning_tokens >= 0),
    cost_usd         REAL CHECK (cost_usd IS NULL OR cost_usd >= 0),
    duration_ms      INTEGER NOT NULL CHECK (duration_ms >= 0),
    occurred_at      TEXT NOT NULL
) STRICT;

CREATE INDEX llm_usage_by_audit ON llm_usage (audit_id, id);
CREATE INDEX llm_usage_by_model ON llm_usage (model, id);

CREATE TRIGGER llm_usage_no_update BEFORE UPDATE ON llm_usage
BEGIN
    SELECT RAISE(ABORT, 'R04: llm_usage is append-only (UPDATE is forbidden)');
END;

CREATE TRIGGER llm_usage_no_delete BEFORE DELETE ON llm_usage
BEGIN
    SELECT RAISE(ABORT, 'R04: llm_usage is append-only (DELETE is forbidden)');
END;
