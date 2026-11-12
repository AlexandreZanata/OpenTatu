-- 0003_audit_progress: progress of an audit run by the worker (spec §C.7; rules R13, R14).
--
-- checkpoint  — last pipeline step completed (technical progress for resuming; not history).
-- limitations — owner-facing sentences describing what the audit did not cover (JSON array).

ALTER TABLE audits ADD COLUMN checkpoint TEXT;
ALTER TABLE audits ADD COLUMN limitations TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(limitations));

CREATE INDEX audits_by_status ON audits (status, created_at);
