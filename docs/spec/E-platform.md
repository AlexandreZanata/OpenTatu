# Part E — Platform

## E.1 Architecture principles

- OpenTatu controls scanners, permissions and evidence; the AI only uses the capabilities
  the system offers.
- Rust across the whole project, including the dashboard; local SQLite for persistence; no
  Node.js or Postgres.
- Start simple: one worker, audits in sequence, no parallelism.
- **Amended by ADR-0002:** OpenTatu runs as one local binary on the owner's machine, with the
  dashboard on loopback and MCP over stdio. Running it on a dedicated VPS (separate from
  production apps) remains possible; then the dashboard and MCP must only be reachable
  through a VPN (e.g. Tailscale or WireGuard) — nothing public on the internet.

## E.2 Stack

| Concern | Choice |
|---|---|
| Monorepo | Cargo workspace |
| Dashboard | Rust + Axum + Askama (server-side HTML) |
| Database | SQLite (local file) with SQLx and embedded migrations |
| Queue | SQLite (jobs table with checkpoints) |
| Validation | serde + JSON Schema (AI output schemas) |
| Worker | Rust + Tokio (audit orchestrator) |
| MCP | Rust MCP SDK (`rmcp`), stdio — phase 5 ✅ (ADR-0030) |
| Scanners | official Docker images pinned by digest (Docker or rootless Podman) |
| HTTPS proxy | none locally; Caddy only on the VPN interface if deployed to a VPS |
| Authentication | single owner, strong password + TOTP, short session (see Q-005 for local mode) |

The exact layout of the repository is in [../ARCHITECTURE.md](../ARCHITECTURE.md) (it
replaces the original tree of §E.3).

## E.4 Main tables

Created incrementally by migrations in `crates/db/migrations/` as each phase needs them.

| Table | Columns (planned) | Phase |
|---|---|---|
| `apps` | slug, name, repo_url, branch, stack, sensitive_data, lab_url, created_at | base |
| `snapshots` | id, app, commit_hash, captured_at | base |
| `audits` | id, app, kind, status, commit_hash, previous_audit_id, created_at, started_at, completed_at, budget, usage, summary | base |
| `events` | id, kind, app, audit_id, occurred_at, actor, target, data (JSON), model, prompt_version, references_event_id — **no UPDATE or DELETE (SQLite triggers)** | base |
| `llm_usage` | id, audit_id, role, model, input_tokens, output_tokens, reasoning_tokens, cost_usd, duration_ms, occurred_at — append-only | 0 ✅ |
| `maps` | id, map_id, app, audit_id, commit_hash, content (JSON), created_at — append-only | 3 ✅ |
| `hypotheses` | hypothesis_id, audit_id, fingerprint, state, finding_id, content (JSON with the verification and its evidence) | 4 ✅ |
| `findings` | finding_id, app, number, state, severity, fingerprint, first/last audit, content (JSON live record) | 4 ✅ |
| `hunt_rounds`, `audit_reports` | audit_id, round / report, content (JSON) — append-only | 4 ✅ |
| `tasks` | task_id, finding_id, app, audit_id, number, state, priority, claimed_by, branch, fix_commit, created_at, content (JSON) — never deleted | 5 ✅ |
| `verifications` | audit_id (the REVERIFICATION audit), task_id, result, created_at, decided_at, content (JSON) — decided once | 5 ✅ |
| `agents` | name, token_hash (never the token, R16), registered_at, revoked_at — never deleted (ADR-0030) | 5 ✅ |
| `decisions` | id, kind, target, justification, expires_at, occurred_at | 4 |
| `scanner_results` | id, audit_id, scanner, result_id, rule_id, message, file, start_line, end_line, reported_severity, cwe — append-only | 2 ✅ |

## E.5 Safe execution of scanners

- Every scanner runs in a disposable container: `--rm --network=none --read-only
  --cap-drop=ALL --security-opt=no-new-privileges --memory=2g --cpus=2 --pids-limit=256`,
  code mounted read-only at `/src`, output at `/out`. Single source:
  `opentatu_scanners::ISOLATION_FLAGS`.
- Databases that need the internet (Semgrep rules, Trivy/OSV vulnerability databases) are
  updated in a separate step, with network, and used offline during analysis.
- Disable the tools' telemetry.
- Implementation notes (ADR-0023): a `--tmpfs=/tmp` and `HOME=/tmp` (read-only root),
  `--user=<uid>:<gid>` of the data directory's owner; refresh runs
  (`opentatu scanners update`) have network and never mount the code; offline data in
  `<data>/scanner-cache/<tool>/`; images, versions and rulesets in `config/scanners.toml`.
- Record the version and digest of each image in `audit.json`.
- Access to the container engine is a high privilege on the host: prefer rootless Podman
  locally; never run OpenTatu on the same machine as production applications.

## E.6 MCP tools (for fixing agents)

| Tool | Result |
|---|---|
| `opentatu_list_apps()` | registered apps |
| `opentatu_memory(app)` | current `MEMORY.md` |
| `opentatu_list_tasks(app, state)` | tasks |
| `opentatu_get_task(task_id)` | full task |
| `opentatu_claim_task(task_id, agent)` | `TASK_CLAIMED` |
| `opentatu_submit_fix(task_id, branch, commit, summary)` | `FIX_SUBMITTED` and triggers a REVERIFICATION |
| `opentatu_verification_status(task_id)` | verification status |
| `opentatu_history(app, limit)` | timeline |

Rules: one token per agent; every call becomes exactly one event (read-only tools:
`MCP_TOOL_CALLED`, Q-007); no tool closes a finding or changes severity.

Implementation (ADR-0030): `opentatu mcp` is spawned by the agent's MCP client with
`OPENTATU_MCP_TOKEN` (created by `opentatu agent add <name>`, stored only as a hash); the
token identifies the agent, so `agent` in `opentatu_claim_task` is optional and must match.
`opentatu_submit_fix` takes the full commit hash; the REVERIFICATION audit that it queues
runs before every other queued audit and is the only way a finding becomes CLOSED (R06).

## E.7 Roadmap

Moved to [../ROADMAP.md](../ROADMAP.md) (with status tracking).

## E.8 Security of OpenTatu itself

- It stores code, keys and the list of open flaws of all the owner's applications: it is
  the most sensitive system the owner has.
- Private network access only (loopback locally; VPN if remote), TOTP, secrets outside the
  repository, encrypted backups of SQLite and `data/`.
- SSH keys (phase 9) with minimal scope and rotation.
- OpenTatu must audit itself as one more registered application, from phase 4 on.

How these are implemented: [../SECURITY.md](../SECURITY.md).
