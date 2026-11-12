-- Phase 7: indexed watcher decisions under BEGIN IMMEDIATE (ADR-0033).
-- Approved patch identities are stored in the existing versioned verification JSON;
-- serde defaults keep earlier records readable without rewriting decided history.
CREATE INDEX audits_app_branch ON audits(app, branch, created_at);
CREATE INDEX audits_app_status ON audits(app, status);
