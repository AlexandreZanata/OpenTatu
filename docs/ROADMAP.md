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
| 6 — Benchmark | ✅ | Run against intentionally vulnerable apps (e.g. OWASP Juice Shop, WebGoat), isolated only, compared with known flaws | A scoreboard exists (found, missed, false positives) per prompt version and per model |
| 7 — Incremental audit | ✅ | Diff between commits, trigger by configured-branch polling (no webhook); a finding whose approved fix reaches the app's branch becomes CLOSED (Q-030, ADR-0032) | A push with a new flaw produces an INCREMENTAL audit that finds only what changed (verified 2026-11-12 on a local Abastevo clone; fix approval, never-merged, squash and normal merge verified) |
| 8 — Dynamic lab | ✅ | Separate VM running a copy of the app with fictitious data; verifier tests; ZAP against the lab | An access-control finding is confirmed by an automated test executed in the lab (verified 2026-11-13 on a fictitious Abastevo fixture) |
| 9 — Host inspection (read-only) | ✅ | Restricted SSH user, closed OS/kernel/listener queries, every attempt recorded | Explicit registered-app inspection collects all three queries; wrong host key, shell/write/subsystem/forwarding requests and revoked client key fail closed in disposable local SSH (verified 2026-11-13) |
| 10 — Monitoring and defense | ✅ | CrowdSec Local API, exact map-bound correlations, bounded continuous polling, owner-approved temporary single-IP response | Disposable HTTP 200→403→200 for owned rollback/native expiry, protected access preserved and foreign decision untouched; exact approval/recovery/history verified (2026-11-13) |

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
- Q-030 answered by the owner (2026-11-12): option (c) — "fix approved, awaiting merge" is
  separate from CLOSED; implemented in phase 7 (ADR-0032).
- The owner approved moving to phase 6 (2026-11-12).

## Phase 6: what exists

- `opentatu-benchmark` (pure) and `opentatu benchmark prepare|run|rescore|scoreboard`
  (ADR-0031): OWASP Juice Shop v20.4.0 pinned by commit, cloned and audited only (never
  run). Its 33 known flaws come from its own `vuln-code-snippet vuln-line` markers and
  challenge catalog (`benchmark/expected/juice-shop.json`, paths and lines only); the
  audited copy has the markers stripped (lines kept) and the answer files left out
  (`codefixes`, catalog, i18n, tests, `rsn`, AI agent files); each run is a fresh app.
- Scoring by file and ±3 lines; one result file per run with models and prompt versions
  from `audit.json`, usage, every match and what each role decided; `benchmark/SCOREBOARD.md`
  per run, per prompt version and model, per category.
- Per-run overrides recorded in the result: `--role-model role=model` (allowlist and
  training consent still apply) and `--max-tokens` / `--max-minutes` for the FULL budget.
- Found by the benchmark and fixed: transport failures of the provider are now retried
  like rate limits (timeouts are not); recorded trees keep files their `.gitignore` hides.
- Verified with three real runs (2026-11-12, US$2.52 in total): recall 21–27% (7–9 of 33),
  Injection 5/5 in every run, Broken Access Control 2–3/5, Broken Authentication 1/6 in one
  run, every other category 0 — the missed flaws are mostly outside the server routes the
  hunter is given (security questions YAML, Solidity contracts, Angular routing, Terraform,
  docker-compose). Run 1 used the default budget, which ran out during verification (Q-031).
  Runs 2 and 3 compared the skeptic: Muse Spark downgraded 1 of 51 findings and dismissed
  none; DeepSeek changed none of 56. By file and CWE, most findings outside the list point
  at Juice Shop flaws it does not mark (eval injection, hard-coded JWT key, SSRF, NoSQL
  injection…) — not yet reviewed one by one — so list precision (10–18%) is a floor, not a
  false-positive rate. Runs 1–3 used the binary before the transport retry (run 2 lost 3
  verifier calls, run 3 one verifier and one planner call) and run 1 a copy without 9
  vendored frontend libraries.
- Q-031 answered by the owner (2026-11-12): the FULL budget stays; Abastevo is the base
  application of every real test from now on.

## Phase 7: what exists

- Hardened changed-file/unified-diff queries and stable patch identity in snapshots;
  context excludes protected paths before requesting diff bytes and records budget
  omissions, renames and deletions (ADR-0033).
- INCREMENTAL uses the previous final non-reverification audit. It preserves unchanged
  map parts, hunts affected/new entry points through depth-2 related code, and falls back
  to changed source seeds. Scanner execution stays unchanged; only changed-path results
  reach the hunter. Later roles process new work; persistent findings retain their original
  confirmed evidence. Empty diffs reuse the map without AI calls.
- `watch --once`, `worker/serve --watch-minutes N`, and `OPENTATU_WATCH_MINUTES` (default
  10, zero disables polling). Services transactionally skips missing baselines, active
  audits and unchanged heads; no automatic FULL or inbound webhook.
- ADR-0032: reverification approves (`FIX_APPROVED`, task DONE), then configured-branch
  FULL/INCREMENTAL audits close by ancestry, all-commit rebase identity, whole-diff squash
  identity, or code absence plus evidence-checked semantic reverification. UI, working
  memory, scope/merge reports and event provenance show this distinction.
- Automated tests cover diff safety/budgets, scope/fallback/map preservation, watcher
  guards, approval/closure, every merge-detection layer, absent evidence, never-merged
  fixes and post-merge regressions. PAUSED resumption remains Q-032; phases 8–10 have not
  started.
- Real Abastevo acceptance (2026-11-12): a local clone of `6a19bcc` and a scratch data
  directory; no application push or server execution. FULL baseline
  `AUD-ABASTEVO-20261112-1516`: 38m09s, 30 calls, 1,683,222 input + 293,540 output tokens,
  recorded US$0.25596559, 19 hypotheses, 5 confirmed findings, 60/114 entries examined.
  Local commit `b331d75` adds an unsafe file handler and its route; `watch --once` queued
  `AUD-ABASTEVO-20261112-1556` automatically. INCREMENTAL: 14m34s, 9 calls, 609,564 input
  + 83,193 output tokens, recorded US$0.08440039; 1/115 entries examined, 114 out of scope,
  3 hypotheses all in the new handler, 2 findings upheld (CRITICAL file read and MEDIUM
  resource consumption), 1 INCONCLUSIVE. The five baseline findings were not reverified,
  reviewed or replanned. Calls fell 70%, recorded cost 67% and elapsed time 62% in this
  run; scanner execution still covered the whole snapshot.
- Both investigative audits ended PARTIAL, honestly: limited static context, declared
  omissions and no dynamic lab. No budget was increased or exhausted. The inconclusive
  hypothesis was not promoted to a finding; this run does not claim unchanged code safe.
- Real fix trail: the MCP agent claimed `ABASTEVO-SEC-0006-T1` and submitted local branch
  `phase7/fix-files` at `852e092` (handler + test in two commits). The regression test fails
  on the vulnerable handler (200) and passes on the fix (403, no fixture disclosure).
  REVERIFICATION `AUD-ABASTEVO-20261112-1611` completed in 37s: 1 call, 62,012 input +
  2,588 output tokens, recorded US$0.00651276; finding FIX_APPROVED, task DONE, no closure
  event, both commit patch IDs and the whole-diff ID stored. Unchanged-main audit
  `-1612` took 87s with zero calls: still FIX_APPROVED, no regression.
- Squash head `0020be2` does not contain the approved commit by ancestry. Audit `-1614`
  closed it with `patch_id_squash` in 92s, zero model calls. This check exposed a missing
  unexamined-entry list in reports without AI; the fallback now retains last-known map
  entries, protected by an integration test.
- A consistent approved-state checkpoint was replayed independently for normal merge
  `9775954`: audit `AUD-ABASTEVO-20261112-1617` closed by `ancestry` in 95s, zero calls,
  with all 115 known entries explicitly unexamined in its report. Only immutable snapshots
  and the offline scanner cache were shared. Both real merge audits deliberately ran
  without AI to prove deterministic detection, with all scanners enabled; they ended
  PARTIAL and did not claim a new AI investigation. Rebase and semantic detection are
  covered by hermetic integration tests, not claimed as additional real model runs.
- `cargo xtask check` passes (279 tests passed, one existing container/network smoke test
  ignored by default; all four real scanners ran above). Total recorded model cost for
  this workflow was US$0.34687874, counting the shared baseline/reverification only once.
  No production application was run and no Abastevo commit was pushed. Q-032 remains open;
  phase 8 was not started.


## Phase 8: what exists

- ADR-0034/Q-033/Q-034: disposable local offline QEMU/TCG, one OpenTatu executable also
  acting as the trusted guest controller. Owner approval binds one queued audit, exact
  commit and complete profile/controller digest. Models select an existing case ID; they
  cannot provide programs, URLs, headers or executable test code. Supported evidence is
  cross-identity read access (CWE-284/639/862), with exact file/handler/CWE matching.
- Core lab policy/context/receipts and positive-control outcome rules; migration 0009
  stores ordered, decided-once attempts with immutable identity/no-delete triggers.
  Services owns atomic grants, caps including interrupted attempts, Clock timestamps,
  computed outcomes and an independent persistence proof gate for verification/fixes.
- Context reuses safe snapshot text selection and produces a private bounded copy with a
  length-framed source digest. Scanners owns fixed NIC-less QEMU argv, pinned image and
  controller, read-only input, disposable root, no monitor/share, cleared environments,
  separate unprivileged app/ZAP identities and bounded lifetime/output/HTTP responses.
  A successful authorized control is mandatory; redirects, failures and unknown cases
  remain inconclusive. Raw bodies/credentials never enter history or model context.
- Verifier 1.3.0 selects a case, receives actual minimized execution through fenced input,
  then reviews it against cited source. Model test claims are discarded; only the system
  receipt is evidence. Actual-call prompt provenance survives a failed follow-up. Passive
  ZAP sends only the two approved GETs, waits and reports; alerts never establish a finding.
  The launcher's explicit heap argument enforces 512 MiB. Static containers stay unchanged.
- CLI `--lab-config`, `lab profile|approve`, `audit create --approve-lab`, strict hidden
  `lab guest`; dashboard/report/audit JSON and `lab_runs.json` show real attempts and
  limitations. Relative CLI data directories are canonicalized before scanner mount
  validation. Fix approval still means FIX_APPROVED, with closure only after merge (R06).
- Actual acceptance (2026-11-13): local Abastevo clone based on `6a19bcc`; an explicitly
  planted fictitious handler, no production services/data and no app push. Vulnerable
  local commit `82a6b51`, audit `AUD-ABASTEVO-20261113-0838`: 8m38s, authorized 200 and
  other identity 200 with the synthetic fixture, REPRODUCED; a finding was recorded only
  with its completed receipt and cited source. Fixed local commit `8f3687a`, audit
  `AUD-ABASTEVO-20261113-0850`: 9m00s, authorized 200 and other identity 403 without the
  fixture, NOT_REPRODUCED. Both built/ran the actual API and ZAP 2.17.0 inside separate
  NIC-less VMs; ZAP completed in 72.5/70.4 seconds with zero passive alerts. The prepared
  image's digest was unchanged after both runs. These known-case tests are not AI discovery
  of an existing production flaw; the two-call AI protocol/fix state gates use hermetic fakes.
- Earlier debug-controller checks reproduced the flaw, but the fixed run hit its 20-minute
  TCG deadline without approval. Temporary validation storage was then lost across a session
  environment change, interrupting a benchmark. Final checks use persistent ignored local
  storage, a release controller and an offline prewarmed public-dependency cache; failures
  remain limitations, never successful proof. Images/configuration/raw logs/app source are
  outside version control. See [LAB.md](LAB.md) for setup, resource caps and exact commands.
- Required prompt benchmark retained as
  [AUD-BENCH-JUICE-SHOP-1-20261113-0815](../benchmark/results/AUD-BENCH-JUICE-SHOP-1-20261113-0815.json):
  29m48s, 46 recorded calls, 2,766,560 input + 432,544 output tokens, recorded US$0.39999848.
  The unchanged FULL budget stopped new calls at 3M tokens; already in-flight calls
  completed. PARTIAL: 61 hypotheses, 32 confirmed, 9 inconclusive, 20 not verified;
  no skeptic/planner/reporter calls after exhaustion, plus one provider failure. Found
  3/33 known flaws (9%), four only as hypotheses, 26 missed. The 29 findings outside the
  coding-challenge list are unreviewed candidates, not established false positives.
- Benchmark recall is lower than the earlier default-budget run (9/33, 27%). This is an
  observed quality limitation, not an improvement claim: cartographer/hunter 2.2.0 also
  differ from that baseline's 2.1.0, and this run does not isolate the verifier's effect.
  No budget was increased, answer files exposed or evidence guard weakened to improve the
  score. The earlier interrupted run is not counted as a completed benchmark or included
  in the retained run's cost. Further quality tuning is separate work for the owner to scope.
- `cargo xtask check` passes: 298 tests, three opt-in infrastructure tests ignored by
  default; the two QEMU acceptance tests above passed individually, and all four actual
  static scanners ran in the benchmark. No production app was executed and no Abastevo
  commit was pushed. Q-032 remains open; phases 9/10 and Sentinel were not started.


## Phase 9: what exists

- Q-035 answered; ADR-0035 accepted: Linux controller/target, explicit owner invocation,
  OS ID/version, kernel release and numeric TCP/UDP listener inventory only. The same
  executable is the trusted unprivileged forced collector. No prompt/model change, new
  crate, internal dependency edge, general shell, host queue, scheduler or remediation.
- Core owns exact requests, validated bounded observations, coarse bind scopes, safe
  failure categories and dated inspection/command records. These are live state, separate
  from exact-commit audits; they cannot create/close findings or mark ASVS coverage safe.
- Migration 0010 stores numbered `host_inspections` and decided-once `host_commands`,
  with immutable identity/no-delete triggers. Services admits owner-only registered-app
  inspection, checks profile identity, enforces fixed order/no replay, validates results
  independently and computes collection coverage. Start events commit before SSH access;
  failures and interrupted attempts remain visible. Every explicit invocation is fresh.
- Scanners owns `HostsConfig`, `HostProfile`, `HostSuite`/`SshHosts`, fixed noninteractive
  OpenSSH argv and forced `collect_host`. Literal private/VPN/loopback endpoints only,
  strict independently pinned Ed25519 server identity, owner-private dedicated credential
  file, no inherited SSH config/agent/environment/proxy/forwarding/PTY/multiplexing.
  The remote account needs root-managed forced-command/authorized-key restrictions and
  no sudo or supplementary groups; client flags cannot enforce server provisioning.
- Shared `engine::bounded_execute` reuses the existing process constructor, with cleared
  environment, discarded stderr and bounded stdout/deadlines; the lab helper reuses it.
  Inventory has 10-second connection/20-second query deadlines, five-second collector
  deadlines, 64-KiB output and 512-socket caps. No IPs, processes, logs, environment,
  local private-key paths or raw diagnostics enter history or model context.
- Worker `inspect_host` orchestrates only explicit CLI invocations through the typed port
  and services, with no AI call or audit worker/polling coupling. `--hosts-config`,
  `host profile|inspect|history` and strict hidden `host collect` are available. Collector
  startup bypasses dotenv/database/provider wiring and refuses UID 0. The app page reuses
  Item/item-list and timeline components; collection failure never implies host safety.
- Actual local acceptance (2026-11-13): pinned Ubuntu 24.04 disposable SSH container,
  OpenSSH 9.6p1 Ubuntu-3ubuntu13.20, root-owned unprivileged account configuration,
  read-only filesystem, bounded tmpfs/resources, loopback listener, synthetic dedicated
  key and no application/data mount. Abastevo was registered from its local phase-8 clone,
  without executing its application, changing its repository or accessing production.
  All three inventory queries succeeded; wrong server key yielded explicit TRANSPORT
  failures. Custom-client empty/shell/write/injection/extra-argument requests, actual SFTP
  subsystem and forwarding were refused. The forbidden marker remained absent and a root
  collector invocation failed without stdout. A deliberately invalid account-home .env
  did not affect the collector, proving its dotenv bypass.
- Owner CLI acceptance retained inspection 1 with three OBSERVED results. An initial
  revocation fixture bind mount was rejected under its host-file ownership constraints;
  the final check used a valid root-owned empty authorized-key file in the server's tmpfs.
  Inspection 3 then failed all queries with TRANSPORT and CLI exit 1, with unchanged
  server key/profile digest and the original success retained. All three invocations,
  including that earlier failed fixture attempt, remain in local history. The disposable
  container/listener was removed. Keys/configuration/raw fixture logs remain ignored.
- `cargo xtask check` passes: 312 tests, five opt-in infrastructure tests ignored by
  default. Both new actual SSH tests passed individually. No production execution or
  provider calls/budget changes; the previously documented benchmark limitation remains.
  See [HOST_INSPECTION.md](HOST_INSPECTION.md) for setup, rotation and repeatable checks.
  Q-032 remains open; phase 10 and Sentinel were not started.

## Phase 10: what exists

- Q-036/Q-037 answered, ADR-0036 accepted: CrowdSec Local API v1, explicit dedicated
  app-scoped private profiles, owner-private native credentials and fixed bounded HTTP.
- Deterministic rules from the exact map/commit plus selected scenario bindings; exact
  owner activation, one-shot/explicit continuous polling, minimized operational detections,
  dedup/cursors and honest one-hour/100-alert coverage limits. No model/prompt changes.
- Concrete simulation/proposal, fresh exact single-use owner digest/justification,
  finite allowlisted single-IP bans up to 300 seconds, protected management/VPN access,
  native allowlist/conflict checks, no create retry, owned-ID rollback/discovery/recovery.
  Native expiry works independently of OpenTatu. API receipts never assert packet effect.
- Migration 0011 with immutable/decided-once records and CAS response projections;
  transactional append-only history. CLI and read-only app cards reuse existing components.
  No Finding/ASVS changes, MCP execution, external notifications or production provisioning.
- Actual local acceptance (2026-11-13): official CrowdSec 1.8.2 and OpenResty bouncer 1.2.3,
  loopback-only LAPI, dedicated internal network, synthetic credentials/alerts, exact local
  Abastevo snapshot and synthetic map of its real health route. App code was not executed,
  no model discovery claim or push to Abastevo. HTTP target 200→403→200 for rollback and
  independent five-second expiry; protected control stayed 200; foreign same-IP decision
  survived owned rollback. No controller loop was needed for native expiry.
  Native allowlist refusal created no alert/decision. `cargo xtask check` passed with
  337 tests and six opt-in tests ignored; the native phase-10 acceptance also passed
  separately. Runtime fixture resources were removed after recording private evidence.
- Setup, fixed limits, credentials/epoch rotation and uncertainty recovery:
  [MONITORING.md](MONITORING.md), [monitor.example.toml](../config/monitor.example.toml).
  Original phases 0–10 are delivered; Q-032 and Sentinel's separately gated expansion
  remain deferred. Stop here for owner review; do not start S0–S11 automatically.
