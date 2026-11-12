# Roadmap

Each phase ends with something testable. Work one phase at a time; at the end of a phase,
stop and show the owner what was done, how to test it, the done criterion reached and the
decisions taken ([recipes/finish-phase.md](recipes/finish-phase.md)). Wait for approval
before the next phase. **Phases 8, 9 and 10 only on the owner's explicit request.**

Status: ✅ done · 🟡 in progress · ⬜ not started

Future expansion planning is recorded in the
[Sentinel master plan](sentinel/SENTINEL_MASTER_PLAN.md) and its
[S0–S11 implementation roadmap](sentinel/SENTINEL_IMPLEMENTATION_ROADMAP.md).
These are proposals after completion and owner acceptance of Audit phases 0–7; they do
not advance the phase statuses below or authorize optional phases 8–10. The owner selected
a separate VPN-accessed Sentinel VPS and a read-only monitoring/Android-alert MVP, with
the custom mobile client later. Remaining gates are in [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md).

| Phase | Status | Scope | Done when |
|---|---|---|---|
| Base | ✅ | Workspace, layered crates, domain core, database foundation (events append-only), dashboard skeleton, prompts, guardrails (`cargo xtask check`), documentation | `cargo xtask check` passes; `opentatu db migrate` and `opentatu dashboard` work |
| 0 — Foundation and research | ✅ | Part D research recorded in [opencode-go.md](opencode-go.md); `config/models.toml` filled; OpenCode Go client in `opentatu-llm`; `llm_usage` table and accounting | `cargo run -p opentatu -- llm ping` answers with DeepSeek Flash and records tokens (verified 2026-11-12) |
| 1 — Apps, audits and memory | ✅ | App registration, snapshot by commit, audit creation with ID and dates, append-only event log use cases, `MEMORY.md` generation, timeline in the dashboard, `opentatu serve` and `opentatu worker` | Creating an "empty" audit produces `AUD-<SLUG>-<YYYYMMDD>-<HHMM>`, the right events, and shows in the timeline with date and time in Brasília time (verified 2026-11-12) |
| 2 — Static scanners | ✅ | Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner in isolated containers; normalization to the common format | An audit saves the four outputs and shows the normalized results in the dashboard (verified 2026-11-12) |
| 3 — Cartographer and coverage | ✅ | Context selection, versioned map, applicable ASVS requirements, coverage matrix | The map lists the real entry points of one of the owner's apps and the owner confirms they are right (TBG map reviewed; owner approved phase 4 on 2026-11-12) |
| 4 — Hunter, verifier (static) and skeptic | ✅ | Hypotheses, verdicts, evidence, severity by the matrix, fingerprint and dedup, report; OpenTatu audits itself from here on | A FULL audit produces `report.md` with findings, comparison and coverage (verified 2026-11-12 on the owner's app Abastevo) |
| 5 — Fix trail and MCP | ✅ | Planner, `MANIFEST.json`, MCP server, reverification, regression detection, regression tests | A fixing agent takes a task over MCP, fixes, submits, and OpenTatu approves or rejects on its own (verified 2026-11-12) |
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
- The owner reviewed the TBG map and approved moving to phase 4 (2026-11-12).

## Phase 4: what exists

- Context per entry point (`entry_point_code`): handler files and what they reach (local
  imports, same package) up to depth 2.
- Hunter rounds (at most 3, prioritized batches, stop rule of §C.7), hypotheses validated
  against the commit and fingerprinted from its code; memory decisions: persistent,
  regression (R09), known false positive (R08), accepted risk.
- Verifier (up to 4 calls at a time) with the R05 evidence check; skeptic with the final
  severity computed by the system (R18); findings `<SLUG>-SEC-NNNN` with live records.
- Coverage statuses VERIFIED / FAILED / NOT_TESTED; comparison with the previous audit;
  `report.md` (reporter body + R17 header, deterministic fallback), `findings.sarif`,
  `findings.json`, `hypotheses.json`, `evidence/`, `findings/<ID>.json`, `results` in
  `audit.json`; open findings and false positives in `MEMORY.md`.
- Per-audit budget guard: `BUDGET_EXHAUSTED`, remaining AI steps skipped, audit PARTIAL (R13).
- Prompts: hunter 2.1.0, verifier 1.1.0, skeptic 2.0.0, reporter 2.1.0.
- Dashboard: findings and hypotheses on the audit page, open findings on the app page,
  `/findings/{id}`, `/audits/{id}/report`.
- Verified on the owner's app Abastevo (commit `6a19bcc`, Go backend, 139 mapped entry
  points): FULL audit `AUD-ABASTEVO-20261112-1025` in 29 min, 28 model calls, US$0.235 —
  3 hunter rounds over 60 prioritized entry points, 20 hypotheses (2 confirmed, 5
  dismissed, 13 inconclusive with what is missing), 2 findings downgraded to INFO by the
  skeptic, ASVS coverage 43% (36 of 83 verified), `report.md` with comparison, SARIF and
  live records. An earlier run showed hunter answers can exceed 5 minutes: the provider
  timeout is now 900 s and rounds hold 20 entry points. After the run, prompts were refined
  (hunter 2.1.0 asks for the violated ASVS ids; reporter 2.1.0 fixes the headings) and model
  text accepts `null`; covered by tests, not re-run end to end.
- The owner approved moving to phase 5 (2026-11-12).


## Phase 5: what exists

- Core: `Task` (OPEN → CLAIMED → FIX_SUBMITTED → DONE; a rejected fix reopens it),
  `Reverification::decide` (R06, R09), `Agent` and `AgentToken` (random 256-bit, stored as
  a hash, R16), `Finding::code_present_in` (R09), events `AGENT_REGISTERED`/`AGENT_REVOKED`.
- Database (migration 0007): `tasks` (never deleted), `verifications` (decided once),
  `agents` (token hash only); the queue serves REVERIFICATION first (§A.2).
- Use cases: `record_tasks` (system ids and priority, dependencies resolved), `claim_task`,
  `submit_fix` (queues the REVERIFICATION audit of the exact commit), `record_reverification`
  (the only path to CLOSED), agent registration/authentication/revocation,
  `record_mcp_call`; `create_audit` refuses hand-made reverifications.
- Planner step (§C.7 step 12, `prompts/planner.md` 2.0.0) in batches of 5 findings; tasks
  validated against the batch and the commit; `tasks/MANIFEST.json` and
  `tasks/<TASK>.md` (pt-BR, with rejection reasons and how to deliver); tasks in the
  finding records.
- REVERIFICATION path: snapshot of the fix commit, verifier with the fix as untrusted
  `<fix>` data (`prompts/verifier.md` 1.2.0; a refutation must cite lines of the commit),
  closed findings' code searched (R09), changed test files (R19), system report body.
- `opentatu-mcp` over `rmcp` 3.9 (both protocol eras): the eight tools of §E.6, one
  event per call; `opentatu mcp`, `opentatu agent add|list|revoke`,
  `opentatu task list|show`.
- Dashboard: tasks and reverifications on the finding, app and audit pages.
- An audit is COMPLETED only without limitations; FULL audits stay PARTIAL until the
  dynamic lab (phase 8).
- Verified end to end on a synthetic vulnerable Flask app (local repository, fictitious
  data): FULL audit `AUD-LOJADEMO-20261112-1132` (6 min, 15 calls, US$0.028) confirmed both
  planted flaws (IDOR and SQL injection, CRITICAL) plus two real ones it was not told about,
  and planned 4 tasks whose regression test reproduced the IDOR. A scripted MCP client
  (both protocol eras) listed, claimed and submitted as `agent:claude-e2e`; each
  reverification took one verifier call (~US$0.001–0.0025, under a minute): the real IDOR
  fix was approved and the finding closed; a fake SQL-injection fix (a reassuring comment,
  reported as 2 suspicious instructions) was rejected; a real fix that undid the IDOR fix
  was rejected as a regression (R09); the final fix was approved. That run exposed a false
  regression on branches cut before another fix — now decided by git ancestry, with a test.
- Open: Q-030 (an approved fix that is never merged).
