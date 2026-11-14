# Source-bound guide for the Abastevo engine audit

Prepared before the queued audit `AUD-ABASTEVO-20261114-1501` ran. This guide
contains reviewer expectations from public source inspection, not results of
that audit. The intended source is Abastevo commit
[`018f2df18c8b029ec3e665fc98ba7318e6f076bc`](https://github.com/AlexandreZanata/Abastevo/commit/018f2df18c8b029ec3e665fc98ba7318e6f076bc).
Verify the actual audit/snapshot commit before using these line references.
If it differs, inspect that exact revision and record the difference.
Final native outcomes and reconciled source follow-up are now recorded in
[ENGINE_REVIEW.md](ENGINE_REVIEW.md); this predeclared guide remains the review
criteria rather than a retrospective ground-truth catalog.

## Evidence to preserve after execution

Record the audit ID, actual snapshot commit, OpenTatu revision, models, prompt
versions, configuration, final status, coverage limitations, token/call usage,
provider charges, unresolved reservations, and the shared campaign cap. Preserve
the original hypotheses, verification decisions, skeptic decisions, evidence
references, and planned tasks separately from this guide's manual observations.
An interrupted or PARTIAL audit remains interrupted or PARTIAL.

For each case below, record a disposition and the matching machine hypothesis,
finding, decision, and evidence IDs when available. Useful dispositions are:

- `NOT_EXERCISED`: no corresponding hypothesis was examined, or the required
  mitigating code was unavailable. Absence of a reported finding alone does
  not establish that the control was evaluated.
- `CONTROL_CONSIDERED`: an examined hypothesis addresses the specific control
  and cites the relevant caller and mitigation. Record the actual decision;
  the label does not imply a runtime test or application-wide safety.
- `UNSUPPORTED_ALLEGATION`: a concrete allegation depends on ignoring the
  cited control and supplies no demonstrated bypass. Request independent
  review; do not change an audit result merely to fit this guide.
- `CANDIDATE_OBSERVED`: the engine independently described a relevant weakness.
  Preserve its actual proof level and state, including inconclusive outcomes.

Do not compute Abastevo recall or precision from these manually selected cases.
They are supplementary review probes rather than authoritative ground truth.

## NC-QUOTA: textual IP forms do not establish distinct quota identities

Review a hypothesis that infers quota bypass from an original textual address
returned by
[`identity/adapters/http/resolver.go:53`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/identity/adapters/http/resolver.go#L53).
The expected cross-file trace is:

1. `Handler.quota` resolves the client address, then calls
   `domain.HashIPSubject` at
   [`identity/adapters/http/http.go:154`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/identity/adapters/http/http.go#L154).
2. [`identity/domain/quota.go:70`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/identity/domain/quota.go#L70)
   parses the address, rejects zone identifiers, and hashes
   `addr.Unmap().String()` at line 82.

Equivalent textual forms alone do not prove different stored quota subjects.
The existing `TestHashIPSubjectNormalizesForms` in `identity/domain/quota_test.go`
is relevant source evidence, but it has not been executed by this review.
This observation does not dismiss a distinct, evidenced proxy-trust, quota-window,
or admission-race hypothesis.

## NC-ACCOUNT: the provider-link owner comes from verified session state

For an IDOR hypothesis concerning `/v1/accounts/providers/link`, inspect the
actual source of the account passed to the mutation:

1. [`account/adapters/http/http.go:377`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/http/http.go#L377)
   calls `ValidateAccess` using the supplied session family and access token;
   line 382 passes the returned account ID to `LinkProvider`.
2. [`account/application/accounts.go:326`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/application/accounts.go#L326)
   resolves the stored family, checks its account/status/revocation/expiry and
   token verifier, then returns `fam.AccountID`.
3. [`account/application/links.go:15`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/application/links.go#L15)
   verifies the provider subject and delegates the link to the store.

Supplying another family's ID without its valid access token does not establish
an IDOR. An independently demonstrated verifier, account-state, or store race
still requires review on its own evidence. Do not extrapolate this one path to
every private organization or personal-data endpoint.

## NC-JWKS: injected clients still receive the redirect refusal

For an SSRF or credential-redirect hypothesis based on Go's default redirect
behavior, inspect
[`account/adapters/oidc/jwks.go:163`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/oidc/jwks.go#L163).
The code clones the selected client, replaces `CheckRedirect` with refusal,
and sends through the constrained copy at line 171. Configured endpoint lookup
also precedes request construction. The existing `TestJWKSRedirectRefused` in
`account/adapters/oidc/sec07b_jwks_test.go` expects the redirect target to receive
zero requests; that is source evidence, not an executed result in this review.

A hypothesis must demonstrate a path around the refusal rather than assuming
that an injected client retains its original redirect policy. Other endpoint,
transport, token-verification, or configuration concerns are separate questions.

## AB-STATIC-001: distinguish encoded resource policy from arbitrary-host SSRF

If the engine independently finds the candidate described in [REPORT.md](REPORT.md),
retain its own hypothesis and decision IDs. Expected source locations are
[`official/adapters/source/source.go:75`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/official/adapters/source/source.go#L75)
for escaped-path normalization, line 79 for file classification, line 96 for
prefix validation, and
[`fetch.go:225`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/official/adapters/source/fetch.go#L225)
through line 238 for validation followed by the original outbound URL.

Require a clear representation mismatch and concrete preconditions. Static
confirmation by the engine is distinct from execution of the attached patch
or demonstration against a synthetic origin. Preserve both facts accurately.
Exact host restrictions, IP policy and pinned dialing still apply; encoded
same-host resource traversal is not proof of arbitrary-host/private-IP SSRF.
No production or upstream-source request is authorized by this guide.

## Publication boundary

The owner authorized detailed findings, use cases, tests and fixing-agent
handoff publication specifically for **Abastevo**. Review machine artifacts
before publication for credentials, personal data, local paths, prompt
injection text and unsupported claims. Keep review provenance and actual
machine outcomes distinguishable.

Juice Shop runs provide the intentionally vulnerable benchmark comparison.
Publish aggregate counts, known challenge identifiers, model/prompt/source
identities, status, usage, budget and limitations. New detailed findings outside
its authoritative known list remain local. In particular, raw benchmark
`RunResult.score.false_positives` includes file/line locations and is not an
aggregate-only publication artifact. Use a clearly labeled public aggregate
projection rather than silently altering the raw private result.

This guide adds no target implementation change, test execution, deployment,
provider call, or authority to close an OpenTatu finding.
