# Open questions

Ambiguities and decisions that belong to the owner. **Do not resolve them on your own.**
When the code needs an answer before the owner gives one, implement the *working
interpretation* (if listed), keep it easy to change, and mention the question in your phase
report. When the owner answers: record the answer here, mark the status `answered`, and add
an ADR if it shapes the architecture.

Template:

```markdown
## Q-NNN — Title
- **Status:** open | answered (YYYY-MM-DD) · **Blocks:** phase N | nothing yet
- **Context:** what is ambiguous (spec references).
- **Options:** ...
- **Working interpretation:** what the code does meanwhile (if anything).
- **Answer:** (owner)
```

---

## Q-001 — Which timestamp stamps the audit ID?
- **Status:** open · **Blocks:** phase 1
- **Context:** §B.2 builds `AUD-<SLUG>-<YYYYMMDD>-<HHMM>`; §C.7 step 1 generates the ID at
  creation, but the §B.5 example shows `created_at` 14:31:47 with ID `...-1432`, which matches
  `started_at` (14:32:00).
- **Options:** (a) `created_at` — the ID exists before the audit starts, even if queued;
  (b) `started_at` — matches the example, but queued audits would have no ID.
- **Working interpretation:** (a). `AuditId::new` takes the instant as a parameter, so the
  choice lives in the use case.
- **Answer:**

## Q-002 — Meaning of PAUSED versus PARTIAL
- **Status:** open · **Blocks:** phase 1 (status model), phase 4 (budget)
- **Context:** R13 says the audit "pauses with a checkpoint and status PARTIAL"; §C.7 and §D.2
  emit `AUDIT_PAUSED` on budget exhaustion or persistent rate limits; `AUDIT_PARTIAL` also
  exists.
- **Options:** (a) PAUSED = stopped with a checkpoint and resumable; PARTIAL = finished
  without full coverage (e.g. owner gives up resuming, or the period budget ends); (b) a single
  PARTIAL status that can be resumed.
- **Working interpretation:** (a) — `AuditStatus` has both.
- **Answer:**

## Q-003 — Does a DOWNGRADED finding get a fix task?
- **Status:** open · **Blocks:** phase 5
- **Context:** the §A.6 diagram only connects UPHELD → TASK_CREATED.
- **Options:** (a) yes, DOWNGRADED → TASK_CREATED (it is still a real finding);
  (b) no, downgraded findings wait for an owner decision.
- **Working interpretation:** (a) — allowed by `FindingState::allowed_transitions`.
- **Answer:**

## Q-004 — Where does a REGRESSION go next?
- **Status:** open · **Blocks:** phase 5
- **Context:** §A.6 says a regression "reopens with priority raised" without naming the state.
- **Options:** (a) REGRESSION → TASK_CREATED (new task, priority +1); (b) REGRESSION →
  CONFIRMED (re-verify first).
- **Working interpretation:** (a).
- **Answer:**

## Q-005 — Dashboard authentication in local mode
- **Status:** open · **Blocks:** any non-loopback deployment
- **Context:** §E.2 requires a strong password + TOTP + short sessions, designed for a VPS
  behind a VPN. ADR-0002 makes loopback the default.
- **Options:** (a) password + TOTP always, even on loopback; (b) on loopback, rely on the
  `Host` allowlist, strict headers and CSRF tokens for forms, and require password + TOTP
  only when binding elsewhere; (c) a local session token printed by the CLI.
- **Working interpretation:** loopback only; non-loopback binds are refused until this is
  answered.
- **Answer:**

## Q-006 — License
- **Status:** open · **Blocks:** nothing technical
- **Context:** §A.1 says the first version is open source and the GitHub repository is public,
  but no license was chosen (without one, all rights are reserved).
- **Options:** MIT OR Apache-2.0 (Rust convention); AGPL-3.0 (keeps hosted derivatives open);
  other.
- **Answer:**

## Q-007 — Events for read-only MCP calls
- **Status:** open · **Blocks:** phase 5
- **Context:** §E.6 says "every call becomes an event", but §B.6 has no event kind for
  read-only calls (`opentatu_list_apps`, `opentatu_get_task`...) and some are not tied to an
  app.
- **Options:** (a) add an `MCP_CALL` event kind with optional app; (b) record only state-changing
  calls as events and read-only calls in a separate access log.
- **Answer:**

## Q-008 — Shape of `gaps`, `not_analyzed` and `suspicious_instructions` items
- **Status:** open · **Blocks:** phase 3 (output schemas)
- **Context:** the base prompt asks for file and line in suspicious instructions but never
  defines item shapes.
- **Options:** strings; or objects such as `{ "file", "line", "excerpt", "reason" }` for
  suspicious instructions and strings for the other two.
- **Answer:**

## Q-009 — Where does the owner's name (prompt variable `owner`) come from?
- **Status:** open · **Blocks:** phase 3
- **Options:** a `[owner] name` entry in a new `config/opentatu.toml`; an environment
  variable; the dashboard settings page.
- **Answer:**

## Q-010 — OpenCode Go terms of use and data retention
- **Status:** open · **Blocks:** phase 0
- **Context:** §D.1 items 7 and 8 must be confirmed from the official documentation before any
  model call; if automation is not allowed, work stops.
- **Answer:** to be filled from [opencode-go.md](opencode-go.md).
