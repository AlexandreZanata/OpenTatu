# Engine acceptance and independent source follow-up

The real Abastevo audit ended **PARTIAL**: seven hypotheses, zero confirmed,
one dismissed and six inconclusive. It produced **zero native findings and zero
native fixing tasks**. This is an incomplete static audit, not evidence that
Abastevo has no vulnerabilities.

The curated [engine-acceptance.json](engine-acceptance.json) preserves the native
audit record, hypotheses, verification text, coverage, limitations and owner-facing
report. The original pt-BR report remains the `report_md` JSON string. English
source follow-up is stored separately under `source_bound_review`; it does not
rewrite native verdicts or create findings from manual observations.

## Exact identities and recorded usage

- Target: [Abastevo](https://github.com/AlexandreZanata/Abastevo), commit
  `018f2df18c8b029ec3e665fc98ba7318e6f076bc`.
- Audit: `AUD-ABASTEVO-20261114-1501`, kind `FULL`, status `PARTIAL`.
- OpenTatu source commit: `7df5899cdb3844ac127e96c3041da66cc8925721`, reviewed in
  [OpenTatu PR #1](https://github.com/AlexandreZanata/OpenTatu/pull/1).
- Compiled engine binary **SHA-256**, distinct from either Git commit:
  `76e8742f210141463265b16ec9b540b0fd48a9c8d13f35d70d9bf8071e274ea0`.
- Native start: `2026-11-14T18:19:08-03:00`; completion:
  `2026-11-14T18:43:15-03:00`; duration: 1,447 seconds.
- Recorded model usage: 12 calls, 971,356 input tokens, 141,826 output tokens,
  **US$ 0.12783331** of provider-reported allowance consumption for this audit.
  This subtotal is not the entire experiment campaign's consumption.
- All roles that ran used `deepseek-v4.1-flash`. Recorded prompts:
  `base@1.1.0`, `cartographer@2.2.0`, `hunter@2.3.0`, `verifier@1.4.0`,
  `reporter@2.1.0`. No skeptic or planner output is invented for this run.

The worker used the same durable campaign ledger and US$ 20 aggregate ceiling
as the other experiments. Only public runtime limits appear in the curated
export; the private ledger, binary path, application configuration and logs
are not included. Native input artifact SHA-256 values are preserved under
`source_artifact_sha256` for reconciliation.

The native ASVS summary records 118 applicable requirements: 27 verified,
zero failed and 91 not tested (22% by its integer summary). These are the
system's static-audit coverage flags, not executed dynamic-test results or
application-wide security certification. The original context/entry-point
omissions and other limitations remain in the JSON.

## Source follow-up on every recorded hypothesis

Each item below is independent inspection of the same pinned source after
reconciliation with the final native artifacts. Source dispositions are
reviewer assessments, not retroactive engine decisions. Existing target tests
were read where relevant and were **not executed**.

### H001 — email-shape validation and the absent SMTP sink

Native result: **INCONCLUSIVE**. The narrow validator does accept internal
control characters and extra at-signs, but the claimed outgoing header injection
requires a mail-header sink absent from the pinned composition. The API constructs
the memory outbox and enables it only for memory mode in
[`main.go:472`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/cmd/api/main.go#L472).
[`SendCode:50`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/mail/memory.go#L50)
appends a struct to a bounded ring; it constructs no mail headers and makes no
network request. Configuration accepts only off or memory delivery at
[`config.go:610`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/platform/config/config.go#L610).
Source follow-up: **UNSUPPORTED_OR_MITIGATED** for the current SMTP-injection
claim. Validator hardening is a separate topic and is not a demonstrated mail
exploit. The native verifier explicitly lacked the adapter/composition context.

### H002 — request identifiers are sanitized before the admin handler

Native result: **INCONCLUSIVE**. The complete router is wrapped by `requestID`
in [`httpserver.go:115`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/platform/httpserver/httpserver.go#L115).
The middleware replaces unsafe identifiers and overwrites the request header
before dispatch at
[`httpserver.go:202`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/platform/httpserver/httpserver.go#L202);
`safeRequestID` restricts length and characters at line 301. The API uses this
server composition. JSON error encoding and the admin UI's text rendering add
separate output controls; detailed anchors are retained in the JSON follow-up.
Source follow-up: **UNSUPPORTED_OR_MITIGATED** for the alleged unsanitized
header log poisoning/XSS. This does not assert safety of every audit/log field.

### H003 — forwarded TLS assertions remain a conditional trust concern

Native result: **INCONCLUSIVE**. The handler's
[`TLS check:45`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/payments/adapters/http/asaas_webhook.go#L45)
does accept the forwarded scheme without binding that assertion to a trusted
peer. Configured credentials, a valid webhook token and the matching merchant
remain independent admission gates. The
[`production Compose template:67`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/infra/compose.prod.yml#L67)
publishes no API host port, and
[`Caddy:68`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/infra/caddy/Caddyfile#L68)
replaces the forwarded scheme. Captured staging NodePort templates do not prove
current reachability or firewall state. Source follow-up:
**INCONCLUSIVE_NEEDS_MORE_SOURCE**. A conditional header-trust concern is not
proof of real token capture or authenticated financial event injection.

### H004 — claim ownership is checked in application code and SQL

Native result: **DISMISSED**. The earlier application check compares the locked
claim owner with the session-derived account at
[`claims.go:127`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/contributor/application/claims.go#L127).
The concrete write independently requires the same account, pending state and
unexpired claim at
[`contributor.sql:40`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/db/queries/contributor/contributor.sql#L40).
The adapter and generated query preserve those parameters and predicates.
Source follow-up: **UNSUPPORTED_OR_MITIGATED**. This supports the dismissal and
adds the update-time SQL controls missing from the model's context; it is not
an executed cross-account test.

### H005 — intentional aggregate admission is a policy/capacity tradeoff

Native result: **INCONCLUSIVE**. Aggregate rolling ceilings exist in both memory
and PostgreSQL, including the atomic admission logic at
[`pgstore.go:197`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/pgstore.go#L197).
The [`policy constants:25`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/domain/policy.go#L25)
are deliberate controls against unbounded durable growth, while key signup is
intentionally anonymous. Source supports possible consumption of shared
capacity; it does not establish an admission-policy bypass or its operational
availability impact. Source follow-up: **INCONCLUSIVE_NEEDS_MORE_SOURCE**.
Removing or raising these limits is not an approved correction.

### H006 — device authorization occurs downstream of the HTTP assignment

Native result: **INCONCLUSIVE**. The concrete store calls its device gate before
processing at
[`commands.go:103`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/cloudsync/adapters/commands.go#L103).
[`device.go:74`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/cloudsync/adapters/device.go#L74)
binds all principal fields into
[`LockDevice:47`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/db/queries/cloudsync/records.sql#L47):
device, account, family, contributor, key and origin, with revocation/tombstone
exclusion and a shared row lock. Reads and mutations also scope records to the
account. Related pull/snapshot/manifest/import/restore paths retain this gate.
Source follow-up: **UNSUPPORTED_OR_MITIGATED** for the proposed cross-account or
revoked-device bypass. The native verifier lacked the concrete store and SQL;
its original inconclusive result remains unchanged.

### H007 — development-storage traversal needs downstream proof

Native result: **INCONCLUSIVE**. The
[`proxy prefix check:33`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/platform/httpapi/development_storage.go#L33)
does not normalize dot segments. The proxy nevertheless supplies no signing
credentials and downstream private S3 signature validation remains the
authentication boundary. Composition gates the fixed development bucket and
preview configuration at
[`main.go:1557`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/cmd/api/main.go#L1557);
production rejects the preview flags at
[`config.go:680`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/platform/config/config.go#L680).
An arbitrary signature-prefixed header does not prove authorized S3 access.
Actual routing, normalization and valid-capability retargeting require a
synthetic end-to-end demonstration. Source follow-up:
**INCONCLUSIVE_NEEDS_MORE_SOURCE**, with development/owned-staging scope. This
is a different component from the manual official-source candidate.

## Predeclared probes and context limitations

All four probes in [REVIEW_GUIDE.md](REVIEW_GUIDE.md) are **NOT_EXERCISED**:
IP-form quota canonicalization, provider-link owner derivation, JWKS redirect
refusal, and the manual official-source candidate `AB-STATIC-001`. Related
ownership, quota-capacity or development-proxy hypotheses do not exercise those
specific cases. No Abastevo recall, precision or completeness score is assigned.

The native verifier explicitly reported missing concrete adapters, composition
and SQL for several hypotheses. Independent source follow-up supplied some of
those omitted controls. Improving retrieval of injected adapters, caller wiring
and authoritative queries is a recorded next refinement opportunity. These
observations do not establish general model accuracy or a causal improvement
from any individual engine change.

## Fixing-agent handoff and validation boundary

The curated [engine task manifest](engine-tasks/MANIFEST.json) contains an empty
task list and `native_manifest_present: false`: no native planner task existed
to export. No fixing task is invented for an inconclusive hypothesis.

The independent [FIX_TASK.md](FIX_TASK.md) and
[regression-tests.patch](regression-tests.patch) still provide the manually
authored `AB-STATIC-001` reproduction/correction handoff and ten proposed cases.
Their target tests and proposed fix remain unexecuted and unverified. This
candidate did not become an engine-confirmed finding in this run.

Any model proposal retained in JSON is untrusted data. Before applying a proposed
fix, inspect its exact source/preconditions and independent review, then reproduce
it with fictitious data in a disposable local fixture under target `AGENTS.md`.
Ordinary code imports, dependency setup and synthetic test preparation are within
the handoff. Preserve existing controls. Do not contact production, live government
sources or external provider APIs. Do not use production data or real credentials,
run live operational ingest jobs, or perform production migrations/deployments.
Delivery follows maintained `dev` to protected `main`; publication neither applies
a fix nor closes a finding. No target program, target test or dynamic lab was
executed by this review, and no production/upstream service was contacted.
