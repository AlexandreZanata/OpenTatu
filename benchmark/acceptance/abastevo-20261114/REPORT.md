# Abastevo static review — 2026-11-14

This publication bundle records a bounded, human-directed static review of
[Abastevo](https://github.com/AlexandreZanata/Abastevo) at commit
[`018f2df18c8b029ec3e665fc98ba7318e6f076bc`](https://github.com/AlexandreZanata/Abastevo/commit/018f2df18c8b029ec3e665fc98ba7318e6f076bc).
The owner explicitly authorized publishing audit material for this application.
Only public source and synthetic examples are included.

This is **not an OpenTatu-confirmed finding, an executed penetration test, or
benchmark ground truth**. The review candidate below is awaiting the regression
tests in [TESTS.md](TESTS.md) and the independent proof gates of OpenTatu.
Abastevo has no authoritative known-flaw catalog in this review; no recall,
precision, or completeness score is assigned.

## Candidate AB-STATIC-001: encoded paths bypass source-resource classification

Status: **static candidate; runtime impact unverified**. Suggested classification
for independent review: CWE-180, canonicalization before validation. No final
severity has been assigned.

The official-source allowlist authorizes a host, a path prefix, and, for downloads,
a filename pattern. In
[`backend/internal/modules/official/adapters/source/source.go:75`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/official/adapters/source/source.go#L75),
`Match` normalizes `u.EscapedPath()`. Percent-encoded dots remain encoded during
that normalization. The extension classification on line 79, prefix check on
line 96, and filename checks on lines 99–106 consequently inspect an encoded
representation. The original URL is then passed to the HTTP client in
[`fetch.go:228`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/official/adapters/source/fetch.go#L228).
The redirect check on line 167 reuses the same `Match` function.

Two related cases need regression coverage:

- A resource ending in `pwned%2eexe` under an approved listing prefix has no
  literal dot in the inspected basename. It can be classified as a listing
  although decoding yields the otherwise-refused `pwned.exe` resource.
- Percent-encoded `..` segments under the approved download prefix survive
  `path.Clean` and retain an allowed final filename. A server that decodes and
  normalizes those segments can interpret a resource outside the approved path.

These observations establish a representation mismatch in the code. They do not
establish how the real upstream server interprets those URLs or whether an
attacker can supply an accepted source link in an active deployment. A controlled
synthetic origin must demonstrate the complete path before asserting operational
exploitation. No requests were sent to an upstream government service.

The remaining origin and IP restrictions still matter: this candidate does not
demonstrate arbitrary-host SSRF, private-IP access, credential exposure, remote
code execution, or malicious workbook execution. Downloads remain subject to
existing resource bounds and downstream parsing rules.

The existing
[`source_test.go`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/official/adapters/source/source_test.go)
covers literal traversal and wrong filenames. The attached patch extends those
same APIs with encoded variants. Its execution status is explicitly unexecuted.
The proposed correction and acceptance gates are in [FIX_TASK.md](FIX_TASK.md).

## Controls that should prevent false positives

- **Quota identity:**
  [`identity/domain/quota.go:70`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/identity/domain/quota.go#L70)
  parses addresses and hashes `addr.Unmap().String()` on line 82. Different
  representations of the same IPv4 address do not establish a quota bypass.
- **Account ownership:**
  [`account/adapters/http/http.go:377`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/http/http.go#L377)
  derives the provider-link account from a validated session before the mutation
  on line 382. A caller-supplied session identifier alone is not evidence of IDOR.
- **JWKS redirects:**
  [`account/adapters/oidc/jwks.go:163`](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/backend/internal/modules/account/adapters/oidc/jwks.go#L163)
  clones the HTTP client and refuses redirects, including for an injected client.
  Default Go redirect behavior does not establish an SSRF finding here.

These are scoped static observations, not application-wide security guarantees.
[USE_CASES.md](USE_CASES.md) describes the corresponding review and test cases.

## Scope and provenance

The review inspected selected backend authorization and egress paths and relevant
tests. It did not run Abastevo, install its dependencies, contact production,
inspect production data, or test an upstream service. It does not cover every
route, private organization boundary, dependency, deployment, or client.
Provider-funded OpenTatu acceptance runs, if later added, need separate artifacts
with their actual model, prompt versions, usage, and outcomes. They must not
retroactively change the origin of this static candidate.

Abastevo is created by **Alexandre Zanata (@AlexandreZanata)**:
[creator](https://github.com/AlexandreZanata),
[original repository](https://github.com/AlexandreZanata/Abastevo). Its pinned
[Abastevo Visible Attribution License 1.0](https://github.com/AlexandreZanata/Abastevo/blob/018f2df18c8b029ec3e665fc98ba7318e6f076bc/LICENSE)
is a custom license; this bundle does not describe it as MIT or change its terms.
The patch contains newly authored tests, not copied target implementations.
