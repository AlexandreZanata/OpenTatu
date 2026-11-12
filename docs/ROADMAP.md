# Roadmap

Each phase ends with something testable. Work one phase at a time; at the end of a phase,
stop and show the owner what was done, how to test it, the done criterion reached and the
decisions taken ([recipes/finish-phase.md](recipes/finish-phase.md)). Wait for approval
before the next phase. **Phases 8, 9 and 10 only on the owner's explicit request.**

Status: ✅ done · 🟡 in progress · ⬜ not started

| Phase | Status | Scope | Done when |
|---|---|---|---|
| Base | ✅ | Workspace, layered crates, domain core, database foundation (events append-only), dashboard skeleton, prompts, guardrails (`cargo xtask check`), documentation | `cargo xtask check` passes; `opentatu db migrate` and `opentatu dashboard` work |
| 0 — Foundation and research | ✅ | Part D research recorded in [opencode-go.md](opencode-go.md); `config/models.toml` filled; OpenCode Go client in `opentatu-llm`; `llm_usage` table and accounting | `cargo run -p opentatu -- llm ping` answers with DeepSeek Flash and records tokens (verified 2026-11-12) |
| 1 — Apps, audits and memory | ✅ | App registration, snapshot by commit, audit creation with ID and dates, append-only event log use cases, `MEMORY.md` generation, timeline in the dashboard, `opentatu serve` and `opentatu worker` | Creating an "empty" audit produces `AUD-<SLUG>-<YYYYMMDD>-<HHMM>`, the right events, and shows in the timeline with date and time in Brasília time (verified 2026-11-12) |
| 2 — Static scanners | ✅ | Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner in isolated containers; normalization to the common format | An audit saves the four outputs and shows the normalized results in the dashboard (verified 2026-11-12) |
| 3 — Cartographer and coverage | 🟡 | Context selection, versioned map, applicable ASVS requirements, coverage matrix | The map lists the real entry points of one of the owner's apps and the owner confirms they are right |
| 4 — Hunter, verifier (static) and skeptic | ⬜ | Hypotheses, verdicts, evidence, severity by the matrix, fingerprint and dedup, report; OpenTatu audits itself from here on | A FULL audit produces `report.md` with findings, comparison and coverage |
| 5 — Fix trail and MCP | ⬜ | Planner, `MANIFEST.json`, MCP server, reverification, regression detection, regression tests | A fixing agent takes a task over MCP, fixes, submits, and OpenTatu approves or rejects on its own |
| 6 — Benchmark | ⬜ | Run against intentionally vulnerable apps (e.g. OWASP Juice Shop, WebGoat), isolated only, compared with known flaws | A scoreboard exists (found, missed, false positives) per prompt version and per model |
| 7 — Incremental audit | ⬜ | Diff between commits, trigger by push (webhook or schedule) | A push with a new flaw produces an INCREMENTAL audit that finds only what changed |
| 8 — Dynamic lab | ⬜ (on request) | Separate VM running a copy of the app with fictitious data; verifier tests; ZAP against the lab | An access-control finding is confirmed by an automated test executed in the lab |
| 9 — Host inspection (read-only) | ⬜ (on request) | Restricted SSH user, closed allowlist of commands, every command recorded | — |
| 10 — Monitoring and defense | ⬜ (on request) | Integration with established sensors (e.g. Wazuh, Falco, osquery, CrowdSec), detection rules from the map, reversible response playbooks with owner approval | — |

## Base: what exists

- Domain core with tests: IDs (§B.2), time and `Clock` (§B.3, R03), finding state machine
  (§A.6, R06–R09), severity adjustments (§C.8, R18), event types (§B.6), fingerprint with a
  golden test (§B.8), secret masking (R15).
- SQLite foundation: `apps`, `snapshots`, `audits`, `events` with append-only triggers (R04).
- Use case layer with a health check; dashboard (home + `/health`) with UI components and
  security middleware; CLI with `db migrate` and `dashboard` (other commands answer with
  their phase).
- LLM abstraction (router + config + fake client), prompt catalog with strict rendering and
  fencing, scanner isolation flags and port.
- Guardrails: layering, dependency hygiene, documentation and vocabulary checks; clippy bans
  on direct clock reads, process spawning and environment reads.

## Phase 0: what exists

- Research record [opencode-go.md](opencode-go.md) (endpoints, model ids, limits, terms,
  retention, live checks) and `config/models.toml` filled from it.
- `opentatu-llm`: `OpenCodeGoClient` (Chat Completions, Bearer auth, product User-Agent,
  mandatory `x-opencode-session`), `Llm` single call path with model allowlist, guard against
  models that train on prompts, capped exponential backoff, usage recording port.
- `llm_usage` table (append-only, migration 0002), `Services::record_llm_usage` and
  `Services::llm_usage_this_month`; `LlmUsage`, `UsageTotals` and `Secret` in core.
- `opentatu llm ping [--model <id>]`: verified with `deepseek-v4.1-flash` and
  `muse-spark-1.3-contributor`; models outside the allowlist are refused locally.
- Base open questions Q-001–Q-010 resolved (ADR-0019): audit status machine, fix priority,
  `MCP_TOOL_CALLED`, shared AI output fields (base prompt 1.1.0), dashboard `Origin` check.
- Model per role decided by the owner (Q-011, ADR-0022).

## Phase 1: what exists

- `opentatu app add|list`, `opentatu audit create|list`, `opentatu worker [--once]`,
  `opentatu serve` (dashboard + worker in one process).
- Core: `NewApp`/`App` (validated profile, git-option injection rejected), `Audit` record,
  `CommitHash` (R02), `DataLayout` (every path of §B.4), compact local time formats.
- Database: `apps`, `audits` (queue + status machine + checkpoint + limitations, migration
  0003), `snapshots` repositories; event queries per app and per audit.
- `opentatu-snapshots` (new crate): bare mirror per app, revision → full commit, read-only
  checkout of the exact commit, hardened git calls (ADR-0020).
- Use cases: register app, create audit (ID stamped with `created_at`, `-N` suffix on
  collisions, previous audit linked), claim, snapshot, finish/fail, timelines.
- Worker pipeline: snapshot → final status (PARTIAL with one limitation per missing step,
  R13/R14) → artifacts `app.json`, `audit.json`, `history.jsonl`, `MEMORY.md` (ADR-0021).
- Dashboard: apps list, recent audits, app page with profile, audits table and event
  timeline in Brasília time; 404 page.
- Verified end to end: OpenTatu registered as an app, audits `AUD-OPENTATU-20261112-0823`
  and `-0823-2` queued, run by `worker --once` and by `serve`, shown in the timeline.

## Phase 2: what exists

- `config/scanners.toml`: images pinned by digest, versions, Semgrep ruleset (`p/default`),
  OSV ecosystems, engine and timeout (ADR-0023).
- `opentatu-scanners`: one adapter per tool (Semgrep, Gitleaks, Trivy fs, OSV-Scanner) with
  offline analysis runs, refresh runs, readiness and normalization; `ScannerSuite` port,
  `Scanners` (containers) and `testing::FakeSuite`; runs as the data directory's owner with
  a `/tmp` tmpfs; network runs can never mount the source.
- `opentatu scanners update [--only <tool>]` and `opentatu scanners status`.
- Core `ScannerKind`, `ScannerResult`, `ScannerRun`; `DataLayout::{scanner_output_dir,
  scanner_cache_dir}`; append-only `scanner_results` table (migration 0004).
- Use cases: `record_scanner_run`, `record_scanner_failure`, `scanner_results`,
  `scanner_runs` (derived from `SCANNER_RAN`/`SCANNER_FAILED` events).
- Worker step `SCANNERS` after the snapshot: raw outputs in `scanners/<tool>.json`, results
  stored, scanner versions in `audit.json`; a scanner that cannot run becomes a limitation.
- Dashboard: audit page `/audits/{id}` (summary, limitations, scanner runs, normalized
  results, events), linked from every audits table.
- Verified end to end with real containers: `opentatu scanners update` (73 s, ~2 GB) then
  audit `AUD-DEMO-20261112-0855` of a synthetic vulnerable repository — four raw outputs
  saved, 23 normalized results (Semgrep 5, Gitleaks 1, Trivy 10, OSV-Scanner 7) on the audit
  page, versions in `audit.json`, the planted token absent from every output; the audit took
  21 s.

## Phase 3: what exists

- OWASP ASVS 5.0.0 catalog embedded (`config/asvs/`, CC BY-SA 4.0); levels 1–3 applicable
  (Q-013).
- `opentatu-context`: safe walk of the snapshot (no symlinks, secrets never read), file
  tree, configuration files and code ranked by entry-point and security signals, packed in
  an explicit budget with line numbers; what is left out becomes a limitation.
- `opentatu-investigator`: `run_role` (render, fence, JSON mode, validation by the serde
  output type, at most 2 retries), base-prompt values, cartographer inputs/output;
  `prompts/cartographer.md` 2.1.0 with typed sections, the `<asvs>` block and one entry
  point per WebSocket message type.
- Core `SecurityMap` with `MapContent::sanitize` (drops invented files, lines and ASVS ids),
  `Coverage::compute` (requirements without a result are NOT_TESTED, R14),
  `SuspiciousInstruction`; append-only `maps` table (migration 0005).
- Worker step `MAP` (§C.7 step 5): map stored with `MAP_UPDATED`, suspicious instructions
  as events (R11), `map/current.json` and `map/versions/`, `coverage.json`,
  `llm_usage.jsonl`, `audit.json` with models, prompt versions, usage and coverage;
  `MEMORY.md` with mapped entry points and coverage. Without an API key or when the role
  fails, the audit continues with a limitation.
- Dashboard: map page `/maps/{id}` (entry points table and every map section), coverage
  page `/audits/{id}/coverage`, map/coverage cards on the app and audit pages.
- Verified on the owner's app TBG-Tupi-Battle-Grounds (commit `b9041af`, 3,980 files):
  full audit `AUD-TBG-20261112-0938` in 5m45s — four scanners, then one cartographer call
  (284K input tokens, US$0.038) producing 80 entry points (HTTP API, admin API, static
  fallback, both WebSocket servers with one entry per message type, CLIs, deploy jobs),
  none dropped by validation, and 163 applicable ASVS requirements, all NOT_TESTED. The
  first call of an earlier run returned ports as numbers and was retried (now accepted).
- Done criterion pending: the owner confirms that the entry points are right.
- Next: phase 4 (hunter, verifier and skeptic) — waits for the owner's approval.
