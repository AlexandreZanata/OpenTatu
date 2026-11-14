# Review and regression cases

All cases target Abastevo commit `018f2df18c8b029ec3e665fc98ba7318e6f076bc`.
Use synthetic inputs and local fixtures. The cases are review acceptance criteria,
not an authoritative known-flaw catalog or a benchmark scoring denominator.

## UC-01: allow a documented official download

Call `source.Match(source.DefaultAllowlist(), url)` with the documented station
workbook path from the existing `TestDefaultAllowlistMatchesDocumentedOrigins`.
Require success with `KindStationDetail`. Preserve the existing listing and
weekly-summary positive cases. No network request is necessary.

## UC-02: refuse a disguised disallowed file

Under the approved price-listing prefix, compare `pwned.exe` with `pwned%2eexe`
and `pwned%2Eexe`. The encoded form must not convert a forbidden file into an
approved listing. The attached pure regression test expects refusal.

## UC-03: refuse encoded traversal beyond an approved resource prefix

Use the approved download prefix, percent-encoded dot segments, and an otherwise
valid `revendas_lpc_*.xlsx` filename. Include lowercase, uppercase, mixed literal/
encoded dots, and encoded path separators. Require refusal before egress.
Never send these inputs to the real source host.

## UC-04: establish the actual outbound resource in a synthetic fixture

After UC-02/03 reproduce locally, adapt the existing loopback TLS fetcher fixture
in `source/fetch_test.go`. The fixture should record the original request target
and decoded resource interpretation, and serve different synthetic markers for
approved and out-of-prefix resources. Require no request for refused inputs.
Test direct fetches and redirects. Test-only loopback/TLS allowances must remain
inside the fixture and must not change production defaults.

This case has not been implemented or executed in this bundle. Its result is
needed before claiming a demonstrated end-to-end egress boundary violation.

## UC-05: avoid a quota-canonicalization false positive

Compare IPv4 and IPv4-mapped IPv6 inputs to `domain.HashIPSubject` using the same
synthetic day key and secret. Require equal subjects. Reject zone identifiers.
Trace callers through this function before reporting textual-IP quota bypasses.
Reuse the existing quota tests instead of creating another hashing mechanism.

## UC-06: avoid a provider-link ownership false positive

Trace `Handler.linkProvider` through `ValidateAccess` to `LinkProvider`. Reuse
the existing injected-service transport tests with two synthetic accounts.
An invalid or revoked session must prevent the mutation; a valid account-A
session must pass account A as the owner. A client-supplied account identifier
must not redirect the mutation to account B. Inspect strict body decoding as
part of this test instead of assuming unknown fields are accepted.

## UC-07: avoid a JWKS-redirect false positive

Reuse the existing JWKS transport fixture. Return a redirect from the synthetic
configured issuer document to a second local endpoint and require that the
second endpoint receives zero requests. Repeat with a caller-supplied HTTP
client. The implementation currently overrides its redirect policy.

## UC-08: measure engine evidence quality honestly

For a separately recorded OpenTatu audit, preserve the exact source commit,
model/prompt/configuration identity, usage, audit status, limitations, hypotheses,
verdicts, and task manifest. Check whether cross-file controls from UC-05/06/07
were visible to the verifier and whether AB-STATIC-001 was independently found.
Human review of these cases is supplementary acceptance evidence. Do not call
the number of cases recall or classify every other finding as a false positive.
