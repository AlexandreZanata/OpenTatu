# Fixing-agent handoff: official-source path canonicalization

Task ID: `AB-STATIC-001`. Status: **candidate awaiting reproduction**.
Source: Abastevo `018f2df18c8b029ec3e665fc98ba7318e6f076bc`.
Source analysis: [REPORT.md](REPORT.md). Test status: [TESTS.md](TESTS.md).

## Objective

Ensure official-source URL authorization and the HTTP request refer to the same
resource interpretation. Percent encoding must not hide a forbidden file type
or traversal beyond an approved resource prefix. Reproduce the candidate before
deciding whether implementation changes are justified.

## Existing components to extend

- `backend/internal/modules/official/adapters/source/source.go`: `Match`,
  `Entry`, and `DefaultAllowlist`.
- The same package's `source_test.go` and `fetch_test.go` fixtures.
- `fetch.go`: its existing direct-request and redirect calls to `Match`.

Keep one allowlist implementation. Do not introduce another URL-policy helper
with a divergent interpretation in the HTTP caller. Avoid unrelated refactors.

## Suggested implementation direction

Choose and document one consistent policy for ambiguous encodings: refuse
encoded path separators and traversal representations, or canonicalize the
decoded path before prefix and filename checks and enforce consistency with
the actual emitted request. Account for encoded dots, separators, backslashes,
and double encoding where the upstream may decode more than once. Reject
ambiguity rather than silently broadening the permitted source set.

Do not assume that cleaning an escaped path removes encoded `..`. Preserve
exact host/scheme/port authorization, query/fragment/userinfo refusal, download
filename restrictions, IP pinning, response bounds, timeouts, and redirect
validation. Preserve documented ordinary listing/download URLs.

## Required evidence

- Apply [regression-tests.patch](regression-tests.patch) in the intended target
  package and record actual RED results at the pinned or explicitly recorded
  maintained revision. Investigate any case that does not reproduce.
- Record GREEN results after the correction, plus the package's existing tests
  and affected compilation/format/static checks required by the target.
- Reuse the local TLS fixture for UC-04 in [USE_CASES.md](USE_CASES.md). Show
  which resource is received and that refused inputs do not trigger egress.
- Include positive controls proving ordinary approved resources still work.
- State the exact threat preconditions and remaining uncertainty. Do not label
  a same-origin path-policy defect as arbitrary-host or private-IP SSRF.

## Execution and delivery constraints

Use public source and fictitious data only. Do not contact production, `gov.br`,
real users, storage, or external providers. Do not run source acquisition,
imports, migrations, ingestion jobs, deployment scripts, or real credentials.
Tests requiring local runtime execution need their own explicitly authorized
disposable environment. Do not weaken a safety check to make the tests pass.

Read and follow Abastevo's current `AGENTS.md` and delivery policy. The inspected
policy uses maintained `dev` and a protected `dev` → `main` pull request;
do not push directly to `main`, merge without the required checks, or deploy.
This bundle supplies review artifacts and regression tests, not a completed fix.
Any OpenTatu finding state change remains subject to independent reverification.

Retain Abastevo's license and attribution: original system **Abastevo**, creator
**Alexandre Zanata (@AlexandreZanata)**,
[creator profile](https://github.com/AlexandreZanata),
[repository](https://github.com/AlexandreZanata/Abastevo).
