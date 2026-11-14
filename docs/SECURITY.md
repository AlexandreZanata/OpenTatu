# Security of OpenTatu itself

OpenTatu stores code, keys and the open flaws of every application of its owner: it is the
most sensitive system the owner has (spec §E.8). This document is the threat model and the
list of controls; the requirements are in the spec (R10–R16, §E.5, §E.8).

## Threat model (local-first)

| Threat | Control |
|---|---|
| Malicious website attacks the local dashboard (DNS rebinding, CSRF, clickjacking) | Loopback bind only; `Host` allowlist; `Origin` check on every state-changing request; CSP without scripts, `frame-ancestors 'none'`; `Referrer-Policy: same-origin`; `SameSite=Strict` cookies and CSRF tokens on every form (when forms arrive) — Q-005 |
| Other local users read data | Data directory created `0700`; database inside it |
| Analyzed code attacks the AI (prompt injection, R11) | Inputs only in fenced blocks (`fence`); base prompt principle 2; suspicious instructions recorded as events; AI has no tools beyond what OpenTatu exposes (R12) |
| Analyzed code attacks the host through scanners | Containers with `--network=none`, read-only root FS (small `/tmp` tmpfs), no capabilities, `no-new-privileges`, memory/CPU/PID limits, non-root user (owner of the data directory), code mounted read-only; images pinned by digest; rootless Podman recommended (ADR-0023) |
| Code or results leak while refreshing scanner data | Refresh runs (`opentatu scanners update`) are separate containers with network that can never mount the application code (`ContainerRun::command_line` refuses); analysis runs are offline; scanner telemetry disabled |
| Secrets reach the model through context selection | `opentatu-context` never reads `.env`, private keys, keystores or `*.tfvars` (names only), never follows symlinks, and skips dependency/build directories; the map keeps environment variable names only |
| The AI declares a vulnerability or a severity without proof (R05, R18) | A "confirmed" verdict must cite an existing file and line of the commit or it becomes inconclusive; the skeptic only chooses the matrix level and the adjustment facts — the system computes the final severity; dismissed code is remembered by fingerprint and not reopened while unchanged (R08) |
| Runaway model spending inside one audit (R13) | `BudgetGuard` checks tokens, calls and minutes before every call and stops the AI steps when a limit is reached (`BUDGET_EXHAUSTED`) |
| The AI invents or is steered into a wrong map (R11) | Inputs fenced; the map is validated against the commit (`MapContent::sanitize` drops invented files, lines and ASVS ids, recorded as system notes); suspicious instructions become events; AI text is escaped in the dashboard |
| Secrets found by scanners are stored or shown | Gitleaks runs with `--redact` (raw output holds `REDACTED`); normalized results never use the matched text; the dashboard escapes every message |
| Secret leakage (R15, R16) | API key only in the environment (`.env`, git-ignored, `0600`), read in one place into `Secret` (prints `****`), never logged or stored; secrets in findings masked with `mask_secret`; no prompt/code content in logs or provider errors |
| Application code used to train third-party models | Models that train on prompts are listed in `models_that_train_on_prompts` and refused for AI roles; roles use DeepSeek V4.1 Flash (no training, zero retention renewed monthly — re-check after 2026-11-30) |
| Unexpected provider spending | `allowed_models` enforced locally (the key is not restricted server-side); per-audit budgets and the 60% monthly share from `llm_usage.cost_usd`; owner advised to disable the pay-as-you-go fallback (Q-012) |
| Malicious repository or URL attacks the host through git | Repository URLs/branches validated (no leading `-`, no `ext::`); git runs without hooks, prompts or `ext::` transport, with `--`/`--end-of-options` before user values; credentials redacted from errors; snapshots are never executed |
| History tampering (R04) | `events` table rejects UPDATE/DELETE with triggers; corrections are new events |
| Fixing agent closes or downgrades a finding | MCP tools cannot close findings or change severity (R06); only OpenTatu's REVERIFICATION of the exact submitted commit closes, and only when the verifier refutes the vulnerability citing lines of that commit and no closed finding's code came back (R09); the agent's summary goes to the verifier as untrusted data (`<fix>`, R11); a commit that cannot be fetched or verified is rejected (ADR-0030) |
| A fixing agent's token leaks or is guessed | One random 256-bit token per agent (`opentatu agent add`), shown once, stored only as a sha256 hash, passed through the MCP client's environment (`OPENTATU_MCP_TOKEN`), never echoed in errors or logs; `opentatu agent revoke` disables it; every call is an event with the agent as actor |
| A fixing agent impersonates another or acts outside its task | The token, not an argument, identifies the agent; only the agent holding a claimed task can submit its fix; dependencies must be done before a claim; reverifications are queued by the system, never by hand (`create_audit` refuses REVERIFICATION) |
| A benchmark app attacks the host | Benchmark apps are cloned at a pinned commit (a moved tag is refused) and only audited like any untrusted code: never built, installed or run; scanners stay offline in their containers; the prepared copy is written by OpenTatu and committed by the hardened git wrapper (ADR-0031) |
| Dynamic tests hit production (R10) | Dynamic verification only against the lab URL, fictitious data; production out of scope in every prompt |
| Supply chain | Minimal dependencies, `default-features = false`, single declaration point, lockfile committed; CI on every push |
| Loss of history | Encrypted backups of `data/` (owner's responsibility; recipe planned) |

## Controls in code (where to look)

| Control | Location |
|---|---|
| Loopback-only config | `opentatu_dashboard::DashboardConfig::local` |
| `Host` allowlist, `Origin` check, security headers, no-store | `crates/dashboard/src/security.rs` |
| Credential wrapper | `opentatu_core::Secret` |
| Model allowlist and training guard | `opentatu_llm::Llm` + `config/models.toml` |
| Fencing untrusted content | `opentatu_investigator::fence` |
| What the AI may see of a commit | `crates/context/src/files.rs` (`classify`, `IGNORED_DIRS`) |
| Validation of AI output against the commit | `opentatu_core::MapContent::sanitize`, `opentatu_investigator::run_role` |
| Secret masking | `opentatu_core::mask_secret` |
| Container isolation | `opentatu_scanners::{ISOLATION_FLAGS, ContainerRun}` |
| Scanner arguments (offline, redaction, telemetry off) | `crates/scanners/src/adapters/` |
| Hardened git | `crates/snapshots/src/git.rs` |
| Append-only events | `crates/db/migrations/0001_foundation.sql` triggers |
| Env reads / process spawning / clock reads banned elsewhere | `clippy.toml` |
| Data dir permissions | `apps/opentatu/src/config.rs` |

## Remote deployment (VPS)

Only after Q-005 is answered and owner authentication exists. Then: dedicated machine (never
the one running production apps), dashboard and MCP reachable only through a VPN (Tailscale
or WireGuard), TLS terminated on the VPN interface, password + TOTP, short sessions.

## Reporting a problem in OpenTatu

OpenTatu audits itself as a registered application from phase 4 on. Until then, record
security concerns as open questions or issues on the private instance.

## Incremental audit and watcher (phase 7)

The watcher performs authorized `git fetch` only, through the existing hardened snapshots
caller; it opens no listener and executes no application code. It resolves a configured
branch to an exact commit, then a transactional service guard prevents duplicate queued
audits. No FULL audit is automatically created. Set `--watch-minutes 0` to disable polling.

Changed names are NUL-delimited; literal pathspecs and `--end-of-options` separate data
from git options. Both rename sides are classified before requesting model-visible patch
bytes. Secret/symlink/oversized/ignored files are excluded and reported. Diff context has
an explicit byte budget and is fenced as untrusted data. Scanner isolation is unchanged.

Fix approval stores stable patch identities before branch deletion, does not close a
finding, and never marks unmerged code as a regression. Only configured-branch audits can
close an approved finding; the semantic fallback requires code absence and a refutation
with lines of the audited commit. Closure records the actual branch commit, including
rewritten history, for subsequent R09 checks. No fixing-agent tool gains closure authority.

## Dynamic lab (phase 8)

ADR-0034/Q-033/Q-034 require owner approval per audit/commit/profile, a prepared pinned
offline QEMU VM, fictitious data and restricted GET cases. Model/MCP output cannot grant
execution, choose a target or provide commands. No NIC, host share, monitor or forwarded
port exists; input is read-only and root disks are disposable. The same controller binary
checks guest identity before any input/process, with no dotenv/API key in the guest.
Application build/run and ZAP use separate unprivileged identities, no capabilities and
cleared environments. Fixed deadlines/output/response/resource caps bound execution.

The authorized control must work before disclosure/denial is meaningful. No raw bodies,
authorization headers or environment enter history/models; receipts are correlated to
the exact attempt/source/image/controller and outcomes recomputed. A persisted matching
receipt plus static evidence is required; model test claims and ZAP alerts cannot replace
proof. Missing consent, unavailable/unsupported tests, failures and mismatches remain
inconclusive/untested. Source exclusions and ZAP failures are limitations. Review
[LAB.md](LAB.md) for preparation, credentials, digest changes and the supported envelope.

## Read-only host inspection (phase 9)

Q-035/ADR-0035 require an explicit owner invocation and a closed OS/kernel/listening-port
query set. Both client and forced collector enforce typed exact requests; no arbitrary
commands, model/MCP host tools, privileged operations or application tests exist. Profiles
bind registered app, literal private/VPN endpoint, pinned Ed25519 server key, dedicated
private identity file and resource envelope. No TOFU, inherited SSH config/environment,
agent, proxy, port forwarding, PTY or connection multiplexing is used.

Root-managed server provisioning must enforce forced command, restricted authorized key,
non-root account with no sudo/supplementary groups, immutable binary/home/key configuration
and no forwarding/TTY/rc/environment injection. Client flags do not substitute for server
restrictions. The remote collector bypasses dotenv, refuses UID 0 and only emits bounded
validated fields. It never sources os-release or requests process data, logs, credentials
or arbitrary files. Server root/collector honesty is trusted, not remotely attested.

Inventory and all failures/interrupted attempts are dated and append-only. Addresses,
local credential paths and raw diagnostics never reach persisted results or models;
ports are coarse inventory rather than reachability/vulnerability proof. Partial or
unavailable collection never means safe. Production configuration/testing/installation
is not performed by implementation validation. Provisioning, rotation, revocation and
actual local SSH acceptance are in [HOST_INSPECTION.md](HOST_INSPECTION.md).

## CrowdSec boundary and temporary defense (phase 10)

Q-036/Q-037 and ADR-0036 authorize dedicated app-scoped LAPI correlation and one exact
owner-approved single-IP ban, never autonomous defense. Literal private endpoints,
verified HTTPS except loopback HTTP, no proxies/redirects/retries, bounded responses and
fixed operation paths contain network authority. Native machine and bouncer credentials
are dedicated external owner-private files; they are not provider credentials. Native
JWT permissions are broader than OpenTatu's capabilities: isolation, credential rotation,
trusted proxy/source-IP attribution and app-only bouncer scope require owner provisioning.

Finite exact targets, mandatory management/VPN/recovery exclusions and independent hard
private-address protections apply before writing, as do native allowlists/existing-decision
checks. Owner approval binds the full concrete spec and expires after 120 seconds; state
and approval commit before one POST. Native TTL is 1–300 seconds independent of controller
survival. API ACTIVE is not proof of packet enforcement. Rollback can delete only a
freshly verified owned ID, preserving foreign same-IP decisions. No range, shell, permanent
ban, bulk delete, standing grant, model or MCP action authority exists.

Ambiguous/interrupted creates remain auditable and are never retried. Missing receipts
without a proven ID remain UNKNOWN; owned cleanup can resume safely. Shrinking target
capabilities allows safe cleanup on the original trusted endpoint; changing its epoch
fails closed. Sensor/model timestamps cannot forge history or extend approval deadlines.
Keep minimized history permanently under R04, including necessary action IPs; raw logs,
headers, bodies, query values and sensor dates never enter persisted results. No production
tests/installations, raw evidence archive or Sentinel expansion is included. Provisioning,
rotation, propagation limits and actual disposable acceptance: [MONITORING.md](MONITORING.md).

## Pure Sentinel preflight and future ZaturoBot boundary

S0.1–S0.2 validate structure and exact caller-supplied configuration, not network identity
or globally current revocation. Source claims/time cannot expand manifest/grant scope.
Later authenticated services must load current trusted registry/grant state and apply
resource bounds before operation admission. No production credential or IO executor is
created by these contracts.

ADR-0038 assigns future bot/client presentation to ZaturoBot while OpenTatu retains policy,
history, budgets, exact approvals and effects. The external bot/app adds a confidentiality
and delegation boundary. Use approved sanitized projections, separate scoped integration
credentials and authenticated owner attribution; ordinary chat, client claims, task
completion or a model response cannot authorize effects. Runtime incident data must not
enter either project's model path merely because existing coding consent exists. Keep
loopback/Host/Origin guards until Q-021/Q-040 remote controls are implemented and tested.
No integration or deployment is enabled here; see the
[service plan](sentinel/ZATUROBOT_INTEGRATION_PLAN.md).
