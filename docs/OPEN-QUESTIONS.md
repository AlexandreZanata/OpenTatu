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

Q-001 to Q-010 were answered on 2026-11-12: the owner delegated the decisions ("implement
them") and asked for the license; the recommended options were adopted (ADR-0019). Q-011 and
Q-012 were answered by the owner the same day (ADR-0022). No question is open.

---

## Q-001 — Which timestamp stamps the audit ID?
- **Status:** answered (2026-11-12)
- **Context:** §B.2 builds `AUD-<SLUG>-<YYYYMMDD>-<HHMM>`; §C.7 step 1 generates the ID at
  creation, but the original §B.5 example matched `started_at`.
- **Answer:** `created_at`. The ID exists from creation, even while the audit is queued.
  `AuditId::new` documents it; the §B.5 example was corrected.

## Q-002 — Meaning of PAUSED versus PARTIAL
- **Status:** answered (2026-11-12)
- **Context:** R13, §C.7 and §D.2 mention both a pause with checkpoint and a PARTIAL status.
- **Answer:** PAUSED = stopped with a checkpoint (budget exhausted, persistent rate limit) and
  resumable; PARTIAL = final, ended without full coverage, never reported as complete.
  Implemented as the audit status machine (`AuditStatus::allowed_transitions`,
  `AuditStatus::reached_by`).

## Q-003 — Does a DOWNGRADED finding get a fix task?
- **Status:** answered (2026-11-12)
- **Answer:** yes, DOWNGRADED → TASK_CREATED (it is still a real finding). Implemented in
  `FindingState::allowed_transitions`.

## Q-004 — Where does a REGRESSION go next?
- **Status:** answered (2026-11-12)
- **Answer:** REGRESSION → TASK_CREATED (a new task) with fix priority raised one level
  (`Severity::fix_priority(is_regression)`, R09); severity unchanged.

## Q-005 — Dashboard authentication in local mode
- **Status:** answered (2026-11-12)
- **Answer:** on loopback, no password: the `Host` allowlist (DNS rebinding), the `Origin`
  check on every state-changing request (CSRF), strict headers and, when forms arrive, CSRF
  tokens with `SameSite=Strict` cookies. Password + TOTP + short sessions become mandatory
  before the dashboard may bind outside loopback; until then non-loopback binds stay
  refused. Implemented: `security::check_origin` (+ `Referrer-Policy: same-origin`).

## Q-006 — License
- **Status:** answered (2026-11-12)
- **Answer:** MIT — use for any purpose (`LICENSE`, `license = "MIT"` in every crate).

## Q-007 — Events for read-only MCP calls
- **Status:** answered (2026-11-12)
- **Answer:** every MCP call produces exactly one event: state-changing tools record their
  domain event (`TASK_CLAIMED`, `FIX_SUBMITTED`); read-only tools record `MCP_TOOL_CALLED`
  (app optional), actor `agent:<id>`. Event kind added to `EventKind`. Read-only calls are
  left out of the `MEMORY.md` timeline (phase 1/5).

## Q-008 — Shape of `gaps`, `not_analyzed` and `suspicious_instructions` items
- **Status:** answered (2026-11-12)
- **Answer:** `gaps` and `not_analyzed` are lists of pt-BR sentences;
  `suspicious_instructions` items are `{ "file", "line", "excerpt", "reason" }` (line may be a
  range; excerpt short and masked). All five shared keys are required. Implemented as
  `opentatu_investigator::CommonFields` and stated in `prompts/base.md` 1.1.0.

## Q-009 — Where does the owner's name (prompt variable `owner`) come from?
- **Status:** answered (2026-11-12)
- **Answer:** the `OPENTATU_OWNER` environment variable (local `.env`), read by the binary
  when prompts are rendered (phase 3). Personal data stays out of the public repository.

## Q-010 — OpenCode Go terms of use and data retention
- **Status:** answered (2026-11-12)
- **Answer:** see [opencode-go.md](opencode-go.md): personal automation is allowed with
  conditions (product User-Agent, `x-opencode-session`, coding-agent-like traffic); DeepSeek
  V4.1 Flash does not train on prompts and has zero retention under a monthly-renewed
  agreement; Muse Spark trains on prompts.

---

## Q-011 — Should the skeptic use Muse Spark for model diversity?
- **Status:** answered (2026-11-12)
- **Context:** §C.4 asks for a model different from the verifier's "if possible". The key
  allows only `deepseek-v4.1-flash` and `muse-spark-1.3-contributor`, and Muse Spark is
  discounted in exchange for Meta training on prompts and completions (not zero retention).
  The skeptic receives findings with code excerpts of private applications.
- **Options:** (a) keep DeepSeek for every role (privacy first); (b) use Muse Spark for the
  skeptic only (diversity, but private code may be used for training); (c) allow another
  non-training model on the key.
- **Answer:** use both models, each where it is strongest (owner). After a comparison on
  synthetic code ([opencode-go.md](opencode-go.md), "Model selection"): DeepSeek V4.1 Flash
  for cartographer, hunter, verifier and planner; Muse Spark 1.3 for skeptic and reporter,
  with explicit consent per role (`accept_prompt_training = true`). Roles without consent
  still refuse training models.

## Q-012 — Pay-as-you-go fallback after the Go limits
- **Status:** answered (2026-11-12)
- **Context:** with Console balance, OpenCode Go may continue after the plan limits by
  charging pay-as-you-go instead of blocking. OpenTatu enforces its own 60% share once audits
  run (phase 4), but other uses of the key could still spend.
- **Options:** keep Console balance at zero / disable Extra Usage; or accept the fallback.
- **Answer:** the owner's Console balance is zero, so the plan blocks at its limits instead
  of charging pay-as-you-go.

## Q-013 — Which ASVS levels count as applicable requirements?
- **Status:** answered (2026-11-12)
- **Context:** coverage (R14) needs a set of applicable OWASP ASVS requirements; ASVS 5.0.0
  has 345 requirements in three levels (70 at level 1, 183 more at level 2, 92 more at
  level 3).
- **Options:** levels 1–2 (253 requirements); level 1 only (70); levels 1–3 (345).
- **Answer:** levels 1, 2 and 3 (owner) — `opentatu_core::APPLICABLE_LEVEL`. The
  cartographer associates the requirements that apply to each entry point and to the app as
  a whole; coverage counts those.

