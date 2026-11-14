# Validation status and reproduction instructions

Target: Abastevo commit `018f2df18c8b029ec3e665fc98ba7318e6f076bc`.

**Completed:** static source inspection; comparison with the existing literal
allowlist-refusal tests; preparation of the attached pure regression-test patch.
The patch also passed `git apply --check` against the exact extracted source, and
its added test file passed `gofmt -d` without changes (10 synthetic table cases).
These artifact checks do not execute the target tests or prove the candidate.

**Not executed:** the attached Go tests, Abastevo compilation, dependencies,
loopback transport tests, production requests, upstream-source requests, or
dynamic exploitation. No test pass/fail result is claimed for the target.
The expected RED behavior below is a prediction from the inspected code, not a
recorded execution result. No fixes are included or applied.

## Attachment

[`regression-tests.patch`](regression-tests.patch) adds
`backend/internal/modules/official/adapters/source/opentatu_canonicalization_test.go`.
It calls the existing `Match` and `DefaultAllowlist` APIs with synthetic URLs.
It never calls `Fetch` or makes network requests. The original upstream URL
patterns appear only as inert strings matching the existing allowlist fixtures.

## Work for the fixing agent

1. Read Abastevo's root `AGENTS.md` and current delivery instructions. Start
   from the pinned source or record why a newer maintained `dev` revision is
   being used; preserve both source identities in the evidence.
2. In an authorized disposable local checkout, check and apply the attachment:

   ```bash
   git apply --check /path/to/regression-tests.patch
   git apply /path/to/regression-tests.patch
   cd backend
   go test ./internal/modules/official/adapters/source -run TestOpenTatuAllowlistCanonicalization -count=1
   ```

3. Record actual failing subtests before changing implementation. The encoded
   dot-extension and encoded traversal cases are expected to fail against the
   pinned implementation. If they do not fail, investigate the discrepancy;
   do not manufacture a failure or claim the candidate is proven.
4. Implement the bounded correction described in [FIX_TASK.md](FIX_TASK.md).
   Repeat the pure test, the existing source-package tests, and the repository's
   affected format/static checks. Preserve every original control.
5. Implement UC-04 only in an explicitly authorized disposable local fixture.
   Record whether the outbound resource discrepancy is actually reproduced.
6. Publish actual command results, source/fix commits, and scope limitations.
   Follow the target's protected delivery workflow. Do not deploy the change.

These instructions are a handoff for a later authorized agent. They do not
authorize OpenTatu to build or run an analyzed repository outside its existing
isolation and owner-approval rules. No dependency, helper, or production fixture
is added to OpenTatu by attaching this patch.
