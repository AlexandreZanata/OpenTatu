-- Q-036/Q-037, ADR-0036: permanent minimized operations, never Audit findings (R04).
CREATE TABLE monitor_rules (
    app TEXT NOT NULL REFERENCES apps(slug), profile TEXT NOT NULL, number INTEGER NOT NULL,
    map_id TEXT NOT NULL REFERENCES maps(map_id), content TEXT NOT NULL CHECK(json_valid(content)),
    PRIMARY KEY(app, profile, number)
) STRICT;
CREATE TRIGGER monitor_rules_no_update BEFORE UPDATE ON monitor_rules BEGIN SELECT RAISE(ABORT,'R04: rule activations are immutable'); END;
CREATE TRIGGER monitor_rules_no_delete BEFORE DELETE ON monitor_rules BEGIN SELECT RAISE(ABORT,'R04: rule activations are permanent'); END;
CREATE TABLE monitor_polls (
    app TEXT NOT NULL REFERENCES apps(slug), number INTEGER NOT NULL, profile TEXT NOT NULL,
    rules_digest TEXT NOT NULL, completed_at TEXT, content TEXT NOT NULL CHECK(json_valid(content)),
    PRIMARY KEY(app, number)
) STRICT;
CREATE INDEX monitor_polls_scope ON monitor_polls(app, profile, rules_digest, number DESC);
CREATE TRIGGER monitor_polls_no_delete BEFORE DELETE ON monitor_polls BEGIN SELECT RAISE(ABORT,'R04: polls are permanent'); END;
CREATE TRIGGER monitor_polls_decided_once BEFORE UPDATE ON monitor_polls
WHEN OLD.completed_at IS NOT NULL OR NEW.completed_at IS NULL
 OR NEW.app IS NOT OLD.app OR NEW.number IS NOT OLD.number OR NEW.profile IS NOT OLD.profile OR NEW.rules_digest IS NOT OLD.rules_digest
 OR json_remove(NEW.content,'$.completed_at','$.last_id','$.received','$.matched','$.failure') IS NOT json_remove(OLD.content,'$.completed_at','$.last_id','$.received','$.matched','$.failure')
 OR json_extract(NEW.content,'$.completed_at') IS NOT NEW.completed_at
BEGIN SELECT RAISE(ABORT,'R04: polls complete once with immutable identity'); END;
CREATE TABLE monitor_detections (
    app TEXT NOT NULL REFERENCES apps(slug), number INTEGER NOT NULL, profile TEXT NOT NULL,
    rules_digest TEXT NOT NULL, source_id INTEGER NOT NULL, entry_point TEXT NOT NULL,
    content TEXT NOT NULL CHECK(json_valid(content)), PRIMARY KEY(app, number),
    UNIQUE(app, profile, rules_digest, source_id, entry_point)
) STRICT;
CREATE TRIGGER monitor_detections_no_update BEFORE UPDATE ON monitor_detections BEGIN SELECT RAISE(ABORT,'R04: detections are immutable'); END;
CREATE TRIGGER monitor_detections_no_delete BEFORE DELETE ON monitor_detections BEGIN SELECT RAISE(ABORT,'R04: detections are permanent'); END;
CREATE TABLE response_plans (
    app TEXT NOT NULL REFERENCES apps(slug), number INTEGER NOT NULL, state TEXT NOT NULL,
    revision INTEGER NOT NULL, target TEXT NOT NULL, content TEXT NOT NULL CHECK(json_valid(content)),
    PRIMARY KEY(app, number)
) STRICT;
CREATE INDEX response_plans_pending ON response_plans(app,state,number);
CREATE TRIGGER response_plans_no_delete BEFORE DELETE ON response_plans BEGIN SELECT RAISE(ABORT,'R04: responses are permanent'); END;
CREATE TRIGGER response_plans_identity BEFORE UPDATE ON response_plans
WHEN NEW.app IS NOT OLD.app OR NEW.number IS NOT OLD.number OR NEW.target IS NOT OLD.target
 OR NEW.revision != OLD.revision+1 OR json_extract(NEW.content,'$.spec') IS NOT json_extract(OLD.content,'$.spec')
 OR json_extract(NEW.content,'$.state') IS NOT NEW.state OR json_extract(NEW.content,'$.revision') IS NOT NEW.revision
 OR (json_extract(OLD.content,'$.started_at') IS NOT NULL AND json_extract(NEW.content,'$.started_at') IS NOT json_extract(OLD.content,'$.started_at'))
 OR (json_extract(OLD.content,'$.expires_at') IS NOT NULL AND json_extract(NEW.content,'$.expires_at') IS NOT json_extract(OLD.content,'$.expires_at'))
 OR (json_extract(OLD.content,'$.decision_id') IS NOT NULL AND json_extract(NEW.content,'$.decision_id') IS NOT json_extract(OLD.content,'$.decision_id'))
BEGIN SELECT RAISE(ABORT,'R04: response spec, approval and owned identity are immutable'); END;
