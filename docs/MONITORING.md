# CrowdSec monitoring and temporary defense

Original roadmap phase 10, owner-authorized in Q-036/Q-037; design in ADR-0036.
One OpenTatu binary and SQLite database, explicit local configuration, no model calls,
external notifications, production installation or automatic new responses. Sentinel
S0–S11 and its deployment/retention/mobile gates remain separate.

## Provisioning and trust

The owner provisions an established CrowdSec sensor, a dedicated app-scoped Local API
(LAPI v1), and an existing remediation component (bouncer). OpenTatu does not install or
reconfigure them. Only the selected app's bouncers may consume this LAPI: sharing it with
SSH/VPN/management or other apps invalidates `dedicated_app_instance = true`. Use a private
network, firewall access to the controller, and a separate recovery path. Keep OpenTatu
separate from production applications as required by [SECURITY.md](SECURITY.md).

Use a dedicated registered/validated machine credential for alert reads and manual writes,
and a dedicated bouncer API key for decision reads. These native credentials are not
per-app/per-action capability tokens; compromise can exceed OpenTatu's own restrictions.
Do not reuse administrative or provider credentials. Inspect the sensor's own native
profiles: they can create automatic responses independently of OpenTatu. Disable unwanted
native automatic remediation for an owner-approval-only installation. OpenTatu neither
approves nor undoes independent native decisions.

Copy [monitor.example.toml](../config/monitor.example.toml) outside analyzed repositories
to an owner-controlled `monitor.local.toml`. Use a literal private IP root URL; HTTP is
accepted only on loopback, otherwise HTTPS certificate validation is mandatory. An optional
absolute `ca_file` adds a trusted PEM bundle; there is no insecure TLS/proxy/redirect mode.
Store credentials separately in an absolute-path, regular, non-symlink Linux file owned
by the running owner, mode 0600, at most 16 KiB, in a private directory. Its strict JSON
schema is:

```json
{"machine_id":"dedicated-machine","password":"<machine-password>","bouncer_key":"<read-key>"}
```

Do not paste actual values into command arguments, TOML, commits, logs or analyzed code.
`*.local.json`/`*.local.toml` and `data/` are ignored. Neither `.env`'s model key nor any
application credential is sent to LAPI. Responses/credentials are never logged verbatim.
Rotate/revoke the native credentials through CrowdSec administration; replace the private
JSON file. Changing non-secret capabilities or the credential path requires new rule
activation and fresh response proposals. Keep the same endpoint/epoch available for
cleanup of existing operations; removing an allowed target does not disable owned cleanup.

Set `instance` to a stable, owner-assigned database epoch. Rotate it after a LAPI reset,
restore or replacement: numeric alert/decision IDs can be reused. A changed endpoint/epoch
refuses old cleanup; restore the trusted original connection or inspect via native
administration. Never point an old epoch at a different database.

## Map-bound monitoring

Register the app and obtain its security map through the existing audit workflow. Choose
1–8 existing entry IDs and exact native scenario IDs. Only literal mapped HTTP routes and
supported methods are accepted; dynamic placeholders, wildcards, WebSocket/CLI entries
and arbitrary expressions fail explicitly. Sensor events must contain `http_path` and
`http_verb` metadata; many alerts lack these and cannot correlate. Ensure native metadata
describes the actual request and trusted source IP. An untrusted forwarding header must
not determine response targets. Inspect sensor/bouncer proxy configuration independently.

```bash
opentatu --monitor-config /private/monitor.local.toml monitor rules --app abastevo --profile abastevo-sensor
# Review the complete rule snapshot and copy its exact sha256 digest.
opentatu --monitor-config /private/monitor.local.toml monitor activate --app abastevo --profile abastevo-sensor --digest sha256:<reviewed-digest>
opentatu --monitor-config /private/monitor.local.toml monitor poll --app abastevo --profile abastevo-sensor
opentatu monitor history --app abastevo
opentatu monitor detections --app abastevo
```

Rules correlate existing alerts locally; they do not deploy CrowdSec parsers/scenarios or
expressions. A non-simulated alert must match all three: scenario, method, literal path.
Queries/fragments, headers, messages, bodies, logs, geodata and sensor dates are discarded.
Only matched normalized IPs, source IDs and rule/map provenance are permanently retained.
Dates come from the controller Clock. A detection is an operational suspicion, not proof
of compromise, a confirmed Finding or a claim that the deployed code equals the map commit.
No Finding/ASVS state changes, model context injection or MCP execution tools are added.

`monitor run` explicitly enables continuous polling for one app/profile, with
`--interval-seconds 10..3600` (default 30). Each cycle also reconciles already approved
responses; it never proposes/applies new bans. Ctrl-C exits. `serve`/`worker` do not start
this loop implicitly. A map or profile change invalidates collection/approval until the
owner reviews and activates new rules. Read-only dashboard cards show the latest five
polls, twenty detections and twenty response plans, with last-observed uncertainty.

Collection is deliberately bounded: one recent hour, maximum 100 new alerts overall,
native scenario-filtered pages of 100, detail lookups for new IDs, 32 requests/64 metadata
items per event, 1-MiB/5-second HTTP bounds, 30-second overall collection deadline. LAPI
has no offset pagination; a full page wholly newer than the cursor is SATURATED rather
than pretending to have consumed everything. Failed/interrupted polls retain their start
and never advance the durable cursor. First collection and gaps over an hour since the
last successful collection explicitly lack guaranteed coverage. This is not a durable
telemetry queue or complete historical backfill. Fix overload/metadata at the sensor;
review native alerts independently rather than clearing history to bypass the bound.

## Exact temporary response

Start with `allowed_ips = []` for monitoring only. The owner may configure at most 32
exact canonical public IP targets; no ranges. Configure every management/VPN/recovery
egress CIDR (IPv4 and IPv6) in `protected_networks`. Hard exclusions independently protect
private/loopback/CGNAT/link-local/unspecified/multicast targets and mapped-address aliases.
TEST-NET documentation addresses are permitted for disposable acceptance fixtures.
Native CrowdSec allowlists and existing decisions also cause pre-write refusal.

```bash
opentatu --monitor-config /private/monitor.local.toml response plan --app abastevo --profile abastevo-sensor --detection 1 --ttl-seconds 60
opentatu response show --app abastevo --plan 1
# Review target, evidence, map/profile/rules, native TTL, approval deadline and digest.
opentatu --monitor-config /private/monitor.local.toml response apply --app abastevo --plan 1 --digest sha256:<reviewed-digest> --reason "Owner justification for this exact temporary action"
opentatu --monitor-config /private/monitor.local.toml response reconcile --app abastevo --plan 1
opentatu --monitor-config /private/monitor.local.toml response rollback --app abastevo --plan 1
opentatu response history --app abastevo
```

Proposal/show perform no remote write. Each approval is single-use and must be consumed
within 120 seconds of proposal. It binds the complete immutable app/profile/map/rules/
detection/target/TTL specification and authorizes owned cleanup. Duration is 1–300 seconds;
there is no override that enlarges the capability. A fresh proposal/approval is necessary
after stale/rejected/used approval. One unresolved operation per target and a cap of 32
pending operations prevent overlapping approvals. No standing grant, automatic ban,
arbitrary URL/shell/action, range decision, bulk IP removal or permanent ban exists.

The approval event and APPLYING state commit before the single POST. The native payload
uses `Ip`, `ban`, `cscli` origin and `opentatu/<full-spec-sha256>` ownership marker. UTC
Clock timestamps follow the native manual-decision contract; local history still uses
Brasília time. Remaining TTL is recomputed immediately before submission and never exceeds
the approved duration. Native expiry works if OpenTatu exits. The expected local deadline
and actual sensor/bouncer timing are distinct; network processing and bouncer cache refresh
can delay packet-level observations. Verify deployment-specific propagation separately.

ACTIVE means an exact finite decision was observed in LAPI, not that a firewall dropped
packets. Verify the existing bouncer is healthy and consuming this dedicated instance.
Rollback freshly checks marker/target/origin/ID and deletes that ID only; it preserves
others' decisions for the same IP. REVERTED/EXPIRED requires confirmed absence of the
previously verified owned ID. A third-party ban can still block traffic after rollback.

After a timeout/interruption, inspect `response show` and run `reconcile`; never repeat
the create. APPLYING has a 30-second in-flight grace, then discovery uses the exact
ownership marker. UNKNOWN without a verified ID stays UNKNOWN even when currently absent:
a delayed write cannot be disproved by absence. Native TTL remains independent. Resolve
uncertainty with trusted sensor administration and preserve the record; this release does
not fabricate a terminal outcome or provide a history-clearing override. Known owned
cleanup resumes safely after interruption. Missing credentials/endpoint produce explicit
uncertainty and require operator repair, never a claim of successful rollback.

Minimized rules/polls/detections/specs/approvals/results and normalized action IPs are
permanent under R04. No raw telemetry archive is created. Full history is private owner
data; fixing-agent history exposes only existing event metadata, not event payloads.
Sentinel's proposed raw-evidence retention policy is not enacted by this phase.

## Disposable acceptance and reproducibility

Never run acceptance against production. On 2026-11-13 the actual fixture used:

- CrowdSec `v1.8.2-3d369d64`, API v1, official image
  `crowdsecurity/crowdsec@sha256:65b74942f7e7ed5c60fde4ac15fc4ba715904c398c8b16dd451f4907d90347d4`.
- Official OpenResty bouncer `v1.2.3`, image
  `crowdsecurity/openresty@sha256:6eeeba8741bab7466424bae420a155d3013461f13201dc224ac92d87693116b1`,
  live mode, one-second cache, ban→403. It served only its inert welcome page.
- Loopback-only published LAPI, disposable volume, synthetic machine/bouncer credentials,
  bounded resources, online API/agent disabled; separate internal Docker network
  `198.51.100.0/24`, sensor `.2`, bouncer `.3`, actual source client `.9`, protected `.10`.
  No spoofed forwarding header, real logs, application environment or production mount.
- Local Abastevo clone/snapshot in `data/phase8-validation/abastevo`, real literal route
  `GET /health/live` at `backend/cmd/api/main.go:231`, explicitly synthetic map/source audit
  PARTIAL and synthetic matching/nonmatching alerts. Its code was not executed or pushed,
  and no model investigation/discovery-quality claim is made.

For opt-in native receipt/expiry regression, provision the same isolated LAPI, write the
private `data/phase10-validation/monitor.local.toml` profile `lab-sensor`/app `abastevo`,
instance epoch, binding `EP-001`→`phase10/http-bf`, allowed `.9` and protected `.10` plus
management networks. Seed a non-remediating, non-simulated native Ip alert with
`http_path=/health/live` and `http_verb=GET` event metadata. Keep fake query/header secret
markers to exercise minimization. Then:

```bash
cargo test -p opentatu-worker --test monitoring r20_actual_abastevo_local_api_receipt_and_native_expiry -- --ignored --nocapture
```

The test rejects non-loopback endpoints, captures an exact Abastevo snapshot through the
existing hardened snapshot component, retains its private acceptance DB, activates the
exact rule digest, correlates, applies a five-second owner-approved decision, refuses
replay and observes native expiry after six seconds with no controller loop. It checks
no Finding was created and fake secrets/credentials did not enter history.

Separately use the CLI flow above against that acceptance DB with the official bouncer:
probe from the real `.9` client, approve a 60-second ban, probe 403 and protected `.10`
200, rollback and verify 200. Repeat with a foreign same-IP decision inserted after the
owned ban: owned rollback must preserve its ID and traffic stays 403 until fixture
administration removes that foreign ID. Finally apply five seconds, stop all controller
polling, verify 403→200 after native expiry/cache refresh and reconcile EXPIRED. All
these actual HTTP checks passed. Shut down only the named disposable containers, network
and volume afterwards. Synthetic credentials/raw diagnostics remain private ignored data.
An additional native allowlist check returned pre-write FAILED/CONFLICT with no new
alert or decision, confirming that owner approval does not bypass native protection.

Primary protocol references: [CrowdSec Local API schema](https://github.com/crowdsecurity/crowdsec/blob/v1.8.2/pkg/models/localapi_swagger.yaml),
[native manual decision client](https://github.com/crowdsecurity/crowdsec/blob/v1.8.2/cmd/crowdsec-cli/clidecision/decisions.go),
[official OpenResty bouncer](https://github.com/crowdsecurity/cs-openresty-bouncer/tree/v1.2.3).
