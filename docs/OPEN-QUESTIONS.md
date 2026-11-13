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
Q-012 were answered by the owner the same day (ADR-0022); Q-013 was also answered.
Q-030 (phase 5) and Q-031 (phase 6) were answered the same day.
Sentinel planning adds Q-014 onward below. Answers selecting a planning direction do not
authorize implementation, deployment or a production action.

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

---

## Q-014 — Sentinel deployment target
- **Status:** answered (2026-11-12) · **Blocks:** nothing yet
- **Context:** the expansion brief proposes a small VPS; current security guidance separates
  OpenTatu from production applications. See the [master plan](sentinel/SENTINEL_MASTER_PLAN.md).
- **Answer:** a separate small Sentinel VPS accessed through a VPN (owner). Actual sizing,
  credentials, service setup and installation are not authorized by this planning choice.

## Q-015 — Separate expiring telemetry from permanent incident history
- **Status:** answered (2026-11-12) · **Blocks:** nothing yet
- **Context:** R04's permanent history conflicts with retaining every raw telemetry payload
  indefinitely. A change to the data boundary needs an ADR, not weaker triggers.
- **Answer:** propose separate retention rules and an ADR: permanent minimized incident
  history, expiring raw telemetry and sensitive evidence (owner). Exact rules are Q-018;
  proposed ADR-0027 is not yet an implemented amendment to R04.

## Q-016 — First useful Sentinel release
- **Status:** answered (2026-11-12) · **Blocks:** nothing yet
- **Context:** the custom Android app and defensive execution could delay useful monitoring.
- **Answer:** prioritize read-only monitoring and Android alerts through an existing
  transport; deliver the custom Kotlin Multiplatform application later (owner).

## Q-017 — Expansion entry gate and original phase 10 mapping
- **Status:** open · **Blocks:** Sentinel S0 kickoff
- **Context:** “initial phases” does not identify an exact completion gate; original phases
  8–10 require explicit requests. Planning S phases must not silently advance them.
- **Options:** accept Audit phases 0–7 first and approve Sentinel S0 separately (recommended);
  or explicitly select a different gate and the relationship to optional phase 10.
- **Working interpretation:** the plan uses 0–7 as a conservative prerequisite; no expansion
  implementation or original optional phase is started.

## Q-018 — Retention classes and exact durations
- **Status:** open · **Blocks:** Sentinel S1 retention and activation
- **Context:** Q-015 approves a proposal direction, not payload/evidence durations, replay
  horizon, backup deletion, legal holds or evidence needed for a retained conclusion.
- **Options:** approve a field-level minimized ledger plus timed telemetry/evidence classes
  and backup expiry; or keep additional evidence under an explicit restricted policy.
- **Working interpretation:** document the classes and expiration events only; no real data
  collection or deletion policy is enabled. Preserve all current append-only triggers.

## Q-019 — Android transport, fallback and external watchdog
- **Status:** open · **Blocks:** Sentinel S3
- **Context:** Q-016 selects an existing transport approach, not a service. Android device
  services, VPN reachability, notification metadata and independent outage detection matter.
- **Options:** authenticated existing Android transport such as ntfy, tested on the actual
  device, plus an approved independent critical channel; or another standard service.
- **Working interpretation:** compare candidates with synthetic payloads; no account,
  subscription, external message, watchdog or automation is created during planning.

## Q-020 — Resource envelope, traffic, thresholds and numeric caps
- **Status:** open · **Blocks:** Sentinel S1–S3 load acceptance
- **Context:** actual VPS sizing, event volume, disk headroom, source lag, rule thresholds,
  retention and alert timing targets are unknown. The plan labels numerical examples as
  engineering hypotheses, not measured capacity or production requirements.
- **Options:** approve the proposed synthetic benchmark profile and adjust from measurements;
  or supply a different hardware/workload/cost envelope before acceptance.
- **Working interpretation:** use synthetic fixtures only; reject unbounded resource growth.

## Q-021 — Source credentials, remote owner access and recovery
- **Status:** open · **Blocks:** Sentinel S1 authenticated ingress; S7 mobile exposure
- **Context:** current dashboard authentication is absent; VPN alone is insufficient.
  Q-005 requires password + TOTP + short sessions before non-loopback owner access.
- **Options:** scoped revocable source tokens over VPN/TLS and owner password/TOTP sessions
  with paired devices and offline recovery (recommended design); or a separately reviewed
  alternative with an ADR explicitly superseding any conflicting existing requirement.
- **Working interpretation:** keep current loopback-only behavior; no remote bind until
  relevant authentication, rotation, revocation and recovery tests pass.

## Q-022 — Runtime telemetry and provider eligibility
- **Status:** open · **Blocks:** Sentinel S5 AI calls; S8 provider selection
- **Context:** OpenCode Go research covers coding-audit usage; Audit skeptic/reporter
  training consent does not authorize runtime incidents or new roles/data classes.
- **Options:** use only purpose-approved non-training runtime providers after current terms,
  retention and region review; or explicitly approve additional purpose-scoped disclosure.
- **Working interpretation:** no runtime data is sent to a model. The deterministic MVP
  works without AI; exact model IDs/URLs/limits remain in reviewed research and config.

## Q-023 — Infrastructure modules and execution boundaries
- **Status:** open · **Blocks:** Sentinel S1 IO layout, S4 sandbox, S10 executor
- **Context:** new collection/notification/response IO must respect layers. The Factory and
  optional CLI runtimes cannot inherit arbitrary process access from scanners/snapshots.
- **Options:** approve narrow infrastructure ports/crates and checked edges via an ADR;
  extend an existing owner only where its responsibility genuinely matches. Preserve one
  product executable and distinct MCP profiles.
- **Working interpretation:** proposed module maps only; no new crate, edge, process
  exception, general shell or production credential is added by planning.

## Q-024 — Optional pseudonymous origin correlation
- **Status:** open · **Blocks:** nothing yet; only a future subject-correlation capability
- **Context:** Abastevo avoids raw IP, precise location and private identifiers. Exporting
  existing quota hashes for a new purpose is not automatically authorized.
- **Options:** omit origin identifiers and use bounded aggregate signals (recommended MVP);
  or approve a separate keyed, rotating, short-lived pseudonym purpose and privacy review.
- **Working interpretation:** omit `subject_reference` and raw identifiers from the MVP.

## Q-025 — Defensive allowlist, exact approval and owner override
- **Status:** open · **Blocks:** Sentinel S9 simulation policy; S10 real response
- **Context:** playbooks need target scope, TTL, rollback, protected management access,
  reauthentication and recovery. R20 owner override needs a precise relation to hard
  capability limits. Credential revocation can be irreversible.
- **Options:** simulation first, then one explicitly approved reversible playbook with
  single-use exact-action authorization; preauthorized automation only under a later policy.
- **Working interpretation:** no production executor or automatic block is enabled;
  irreversible operations remain deferred and no chat statement creates a permission.

## Q-026 — Incident classification and sufficient compromise evidence
- **Status:** open · **Blocks:** Sentinel S2 classification policy; S5 verified conclusions
- **Context:** the Audit vulnerability severity matrix does not define operational incident
  confidence. High volume, an LLM conclusion or scanner output cannot prove compromise.
- **Options:** approve rule-specific triage priority and evidence/reviewer standards with
  separate lifecycle/classification/confidence; require owner confirmation for compromise.
- **Working interpretation:** preserve suspicious/operational/unknown distinctions; no
  detector is permitted to invent proof or close an Audit finding.

## Q-027 — Android devices, distribution, assets and iOS preparation
- **Status:** open · **Blocks:** Sentinel S7 release
- **Context:** supported devices/SDKs, availability of platform push services, distribution,
  brand assets, secure cache and eventual Mac/iOS build ownership are not established.
- **Options:** Android-first signed test distribution on the owner's device with pinned
  compatible KMP/Compose tooling; shared platform ports and iOS delivery later.
- **Working interpretation:** the mobile plan proposes modules/wireframes; it does not claim
  assets or a working app exist. No Android features are added to Abastevo.

## Q-028 — Shared AI budgets and Audit versus Sentinel priorities
- **Status:** open · **Blocks:** Sentinel S5 model admission; S8 fallback expansion
- **Context:** existing per-audit accounting lacks concurrent reservations, and the parsed
  60% shared plan share has no identified runtime enforcement in the inspected baseline.
- **Options:** preserve Audit reverification priority, allocate explicit incident/app/month
  caps and reserve before dispatch; or approve an explicit revised priority policy.
- **Working interpretation:** numerical turn/tool caps are proposals; no unbounded runtime
  workload is admitted and retries/fallbacks cannot acquire a separate budget.

## Q-029 — Abastevo deployment and source-access confirmation
- **Status:** open · **Blocks:** Sentinel S1 bridge installation/activation
- **Context:** source at inspected commit contains Compose/Caddy and dated staging evidence
  for k3s/nginx; neither establishes today's deployment. Private metrics may be loopback
  inside a pod/container; generic HTTP logs lack typed auth/abuse outcomes.
- **Options:** owner confirms current topology and exact read-only observation exports;
  approve optional narrow emitters in a separate Abastevo task only where necessary.
- **Working interpretation:** source-shape synthetic fixtures and a disposable lab only;
  no production logs, admin session, DB credential, Docker socket or kubeconfig access.

---

## Q-030 — A fix approved on a branch that is never merged
- **Status:** answered (2026-11-12) · **Blocks:** phase 7 (implementation)
- **Context:** R06 closes a finding when OpenTatu approves the REVERIFICATION of the
  submitted commit, which usually lives on a branch. If that branch is never merged, the
  next FULL audit of the app's branch finds the same code and R09 marks the CLOSED finding
  as REGRESSION with raised priority. Reverifications already tell the two cases apart by
  git ancestry (ADR-0030): a closed finding's code in a commit that does not contain its
  fix is not a regression of that commit. (Q-014–Q-029 are used by the Sentinel drafts.)
- **Options:** (a) keep R09 literal in FULL audits — "regression" until the fix is merged;
  (b) use the ancestry rule in FULL audits too and report "fix approved but not merged"
  (finding reopened without the priority raise); (c) close a finding only when its fix
  commit reaches the app's branch ("approved, awaiting merge" until then).
- **Answer:** (c) — separate "approved" from "closed". An approved reverification moves the
  finding to a new state, "fix approved, awaiting merge"; the finding becomes CLOSED only
  when the fix reaches the app's branch (`main`). Implemented in phase 7 (ADR-0032); until
  then, the phase 4–5 behavior stays (owner).

---

## Q-031 — FULL audit budget for large applications
- **Status:** answered (2026-11-12) · **Blocks:** nothing
- **Context:** the FULL budget in `config/models.toml` (3M tokens, 300 calls, 90 min) was a
  provisional value to tune with real usage. On OWASP Juice Shop (benchmark run 1, about
  1,100 files, 60 hypotheses) it ran out during verification: 16 hypotheses were never
  verified and the skeptic, planner and reporter did not run (audit PARTIAL, R13), at
  US$0.40. The benchmark runs 2 and 3 raise it for one run only (`--max-tokens`).
- **Options:** (a) keep 3M and accept PARTIAL audits of large apps; (b) raise the FULL
  budget (e.g. 10M tokens / 180 min — about US$1–2 per large audit at list prices);
  (c) cap verification per audit (e.g. the 30 most severe hypotheses) so later steps always
  run, and report the rest as not verified.
- **Answer:** (a) — keep the FULL budget (3M tokens, 300 calls, 90 min). For now the test
  scope is Abastevo: the owner's application is the base of every real test, and it fits
  the budget (1.95M tokens, 28 calls). A larger application or a new test target reopens
  this question (owner).


## Q-032 — When and how a budget-stopped audit resumes
- **Status:** open · **Blocks:** nothing in phase 7
- **Context:** Q-002 defines resumable PAUSED versus final PARTIAL; the phase-7 request
  explicitly defers resumption. Current budget exhaustion remains a final PARTIAL audit.
- **Options:** a later explicit owner command with a renewed bounded budget and validated
  checkpoint; or a separately approved scheduling policy with concurrency/idempotency rules.
- **Working interpretation:** no automatic or manual resumption is implemented in phase 7.
  A new audit is distinct from resuming the stopped run. The owner must choose trigger,
  cumulative budget, retry semantics and handling of changed code/provider configuration
  before resumption is implemented.

## Q-033 — Initial dynamic-lab virtualization target
- **Status:** answered (2026-11-12) · **Blocks:** nothing
- **Answer:** a disposable local QEMU VM, using TCG when KVM is unavailable (owner).
  Application code runs only in that VM, with fictitious data and no network interface.

## Q-034 — Dynamic-test approval granularity
- **Status:** answered (2026-11-12) · **Blocks:** nothing
- **Answer:** owner approval per audit and exact commit, inside a restricted set of
  non-destructive HTTP probes (owner). The approval binds the complete owner-controlled
  lab-profile digest; changing the profile or commit requires a fresh approval.
