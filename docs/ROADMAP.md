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
| 1 — Apps, audits and memory | ⬜ | App registration, snapshot by commit, audit creation with ID and dates, append-only event log use cases, `MEMORY.md` generation, timeline in the dashboard, `opentatu serve` and `opentatu worker` | Creating an "empty" audit produces `AUD-<SLUG>-<YYYYMMDD>-<HHMM>`, the right events, and shows in the timeline with date and time in Brasília time |
| 2 — Static scanners | ⬜ | Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner in isolated containers; normalization to the common format | An audit saves the four outputs and shows the normalized results in the dashboard |
| 3 — Cartographer and coverage | ⬜ | Context selection, versioned map, applicable ASVS requirements, coverage matrix | The map lists the real entry points of one of the owner's apps and the owner confirms they are right |
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
- Next: phase 1 (apps, audits and memory) — waits for the owner's approval.
