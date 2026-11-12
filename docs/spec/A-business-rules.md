# Part A — Business logic (personal use)

## A.1 Purpose

OpenTatu exists so that each of the owner's personal applications has:

1. Deep security investigation, on demand and periodically.
2. A complete, dated and auditable history of everything found, fixed, accepted or dismissed.
3. A fix trail that other AI agents can execute.
4. A knowledge base (the "security map") that will later feed monitoring and defense.

It is not a product. There is a single owner, no multi-user and no billing. The first
version is open source; the running instance stays private and personal. Turning it into a
service is only considered if the project grows.

## A.2 Value and cost model (personal version)

Value produced:
- Less time spent on manual security review.
- Flaws found before deployment, not after.
- Memory across audits: the system knows what it saw, what was fixed and what broke again.
- Standardized, verified fixes — not just "the agent said it fixed it".

Costs:
- OpenCode Go subscription (usage limits per period; see Part D).
- The machine that runs OpenTatu (local-first by default — ADR-0002).
- Time maintaining prompts, scanners and rules.

Economic rules:
- Every audit has its own budget: tokens, model calls, maximum run time.
- OpenTatu's monthly budget uses at most **60%** of the plan limit, because the same key is
  also used for the owner's programming work.
- Consumption priority when the budget is tight:
  reverifications > incremental audits > full audits.

Value metrics (shown on the dashboard):
- Confirmed findings per audit.
- Mean time between confirmation and verified fix (MTTR).
- False positive rate (dismissed / total hypotheses).
- Requirement coverage (OWASP ASVS) per application.
- Cost per confirmed finding (tokens and time).
- Regressions detected.

## A.3 Actors

**Owner** — registers applications, approves dynamic tests, accepts risks, marks false
positives, and may override any decision (with a recorded justification).

**Investigator agent (internal)** — the set of AI roles described in Part C. Uses only the
tools OpenTatu offers. Has no shell.

**Fixing agents (external, via MCP)** — Claude Code, OpenCode, Codex, Cursor, etc. They take
tasks, fix on a branch and submit. They never close findings.

**Deterministic executors** — scanners in isolated containers (Semgrep, Gitleaks, Trivy,
OSV-Scanner; later ZAP in the dynamic lab).

## A.4 Entities

| Entity | Meaning |
|---|---|
| App | Registered system (slug, repository, stack, sensitive data, lab environment) |
| Snapshot | Exact state of the analyzed code (full commit hash) |
| Audit | One investigation run, with ID, kind, start, end, status, budget and cost |
| Security map | Security model of the app (versioned at every audit) |
| Requirement | OWASP ASVS item applicable to the app |
| Hypothesis | Vulnerability suspicion, not yet proven |
| Finding | Hypothesis confirmed with evidence |
| Evidence | Proof supporting a finding |
| Task | Unit of fix work for a fixing agent |
| Verification | OpenTatu's check after a fix |
| Decision | Owner act: accepted risk, false positive, override |
| Event | Immutable record of anything that happened |

## A.5 Audit kinds

- **FULL** — rebuilds the whole map and investigates every entry point. Use: first audit,
  monthly, before large releases.
- **INCREMENTAL** — investigates only what changed since the last completed audit (diff
  between commits) and the components that depend on it. Use: on every push to the main
  branch, or daily.
- **REVERIFICATION** — confirms a submitted fix resolved the finding without regressions.
  Use: whenever a fixing agent submits a fix.
- **PR** (future phase) — investigates a pull request before merge.

## A.6 Finding lifecycle

```text
HYPOTHESIS
   │  (verifier)
   ├──> DISMISSED      (false positive, with a reason)
   ├──> INCONCLUSIVE   (kept under observation; re-evaluated in the next audit)
   └──> CONFIRMED
           │  (skeptic)
           ├──> DOWNGRADED   (severity reduced, with a reason)
           └──> UPHELD
                  │  (planner)
                  └──> TASK_CREATED
                         │  (fixing agent)
                         └──> FIX_SUBMITTED
                                │  (OpenTatu, reverification)
                                ├──> FIX_REJECTED ──> back to TASK_CREATED
                                └──> CLOSED
```

Side states:
- **RISK_ACCEPTED** — owner decision, with justification and expiry date. When it expires,
  the finding goes back to CONFIRMED.
- **REGRESSION** — a CLOSED finding whose fingerprint reappears. Reopens with priority
  raised.

The executable state machine is `opentatu_core::FindingState`. DOWNGRADED and REGRESSION
both lead to a new task (Q-003, Q-004, answered in [../OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md)).

## A.7 Business rules (never break)

- **R01** Only registered applications owned by the owner are audited.
- **R02** Every audit references an exact commit (full hash), never "the current branch".
- **R03** Every audit gets a unique ID and records start and end in ISO 8601 with time zone.
  Dates and times ALWAYS come from the system clock; the AI never generates dates.
- **R04** The history is append-only. Nothing is deleted; record corrections are new events
  that reference the previous one.
- **R05** A hypothesis only becomes a finding with evidence meeting the minimum standard
  (§C.9).
- **R06** A finding is only CLOSED after a reverification approved by OpenTatu. A fixing
  agent's statement closes nothing.
- **R07** An accepted risk requires a justification and an expiry date.
- **R08** A false positive is recorded with reason and fingerprint; it is not reopened while
  the related code does not change.
- **R09** The fingerprint of a CLOSED finding that reappears = REGRESSION, with priority
  raised by one level.
- **R10** No invasive tests in production. Dynamic tests only in the isolated lab, with
  fictitious data.
- **R11** Analyzed code is untrusted data. Instructions inside it (comments, strings, docs)
  are ignored and recorded as a `SUSPICIOUS_INSTRUCTION_DETECTED` event.
- **R12** AI agents have no shell and no direct network or disk access; they only use tools
  exposed by OpenTatu.
- **R13** When the budget runs out, the audit pauses with a checkpoint and status PARTIAL.
  It never pretends to be complete. (PAUSED is the resumable stop and PARTIAL the final
  incomplete state — Q-002, answered; `opentatu_core::AuditStatus`.)
- **R14** Coverage is always reported. "Not tested" is never treated as "safe".
- **R15** Secrets found in code are masked in reports (e.g. `sk_live_****9f2a`).
- **R16** The OpenCode Go API key never goes to the scanners, logs, reports or history.
- **R17** Every report records: models used, prompt versions, scanner versions, cost and
  duration.
- **R18** Severity is computed with a fixed matrix (§C.8), not by impression.
- **R19** Every approved fix produces, when possible, a permanent regression test in the
  application.
- **R20** The owner may override any decision; the override becomes an `OWNER_DECISION`
  event with justification.

## A.8 Suggested cadence

- Incremental: on every push to the main branch (or once a day if there are new commits).
- Full: once a month per app and before important releases.
- Reverification: immediately, when a fix is submitted.
- Review of accepted risks: weekly (expiry alert).
