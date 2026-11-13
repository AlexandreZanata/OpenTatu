# Offline dynamic lab (phase 8)

ADR-0034 implements R10 with a disposable local x86-64 QEMU VM. The owner authorizes
an audit, exact commit and complete profile/controller digest. The verifier selects an
existing case ID, receives its executed result through `<lab>`, and evaluates that result
against the source. It cannot supply programs, request headers, URLs or executable tests.
Only a successful authorized control plus disclosure to the other fictitious identity can
establish `REPRODUCED`. An authorized success plus 403/404 without the fixture marker gives
`NOT_REPRODUCED`; unavailable controls, redirects and other responses are inconclusive.

## Host prerequisites

Linux x86-64, QEMU (`qemu-system-x86_64`, `qemu-img`), `genisoimage`, `cloud-localds`, and
space for the prepared image, private input ISO and disposable overlay. TCG is always used;
KVM is not required. OpenTatu remains the only application executable. The optional tools
and prepared guest image are infrastructure for this phase, outside the repository.

The runner rejects a QCOW2 image with a backing file or a virtual size above 4 GiB. Each
launch has no NIC, monitor, host share or forwarded port; its input and seed disks are
read-only. Host helper environments are cleared. Whole-VM deadlines include boot, offline
build, HTTP and ZAP, and are capped again by the audit's remaining time. One VM executes at
a time per worker. Settings allow 512–4096 MiB, 1–2 CPUs, 30–1800 seconds and 1–8 attempts
per audit, including unsuccessful/interrupted attempts. These are bounds, not performance
claims. Application work uses a 1536 MiB guest tmpfs; ZAP has a 512 MiB Java heap.

## Prepare an immutable guest image

Do this separately from an investigation. Preparation can download public OS/tool assets;
no application source, owner configuration, API keys, real database or production disk may
be present. Use verified upstream releases/checksums and record the image's final SHA-256.

1. Prepare a standalone Ubuntu x86-64 cloud-init/NoCloud QCOW2 image, at most 4 GiB virtual
   size. Install `mount`, `setpriv`, CA certificates, and the offline application compiler
   and dependency cache. Dependencies must come from the app's public pinned manifests,
   with their integrity checks; never import an owner's home directory or credential store.
2. Install the pinned ZAP release and a compatible JRE under `/opt/jre`. Do not enable
   telemetry, update jobs or network-dependent build steps. For Go, provide the toolchain
   at `/usr/local/go` and a read-only module cache at `/opt/opentatu-gomod`. An optional
   warmed `/opt/opentatu-gocache` is copied into the application's private workspace.
3. Create distinct uid/gid 2001 (application) and 2002 (scanner), without privileged groups
   or credentials. Write exactly `opentatu-offline-lab-v1` to `/etc/opentatu-lab-image`.
4. Disable cloud networking and `systemd-networkd-wait-online`; remove stale network
   configuration and preparation credentials. Keep cloud-init enabled for a fresh local
   NoCloud instance. Shut down cleanly. Flatten any preparation overlay with `qemu-img
   convert -O qcow2`, check `qemu-img info --output=json`, then `sha256sum` the final image.
5. Remove the preparation network. Execution supplies no NIC at all. The controller also
   checks root uid, QEMU DMI identity and that `/sys/class/net` contains only loopback,
   before reading any manifest or starting a process. Calling `lab guest` on the host fails.

The host creates the NoCloud seed and mounts its private input under a root-only parent.
The same digest-checked OpenTatu binary runs the guest controller. Application/build and
ZAP processes have separate uid/gid, no supplementary groups/capabilities, no-new-privileges,
cleared environments and discarded output. Only the root controller writes the dedicated
serial receipt. Source copies and overlay writes disappear after the run; the base image
is unchanged. Keep the prepared image patched and trusted: virtualization is the execution
boundary, not a claim that arbitrary guest kernels are safe.

## Configure and approve

Copy [lab.example.toml](../config/lab.example.toml) to an owner-controlled `*.local.toml`
file, with private permissions. The example is a schema illustration, not an enabled lab
or a production application recipe. Adapt build/run argv, offline caches, source prefixes,
fixture environment, two identities and exact GET cases. Only fictitious values belong
there. Never load profiles from the analyzed repository. Reserve OPENCODE_* and OPENTATU_*
for the host; fixture configuration refuses those names. Do not reuse real credentials.

```bash
cargo build -p opentatu --release
./target/release/opentatu --lab-config /absolute/path/lab.local.toml lab profile --app demo
./target/release/opentatu --lab-config /absolute/path/lab.local.toml audit create --app demo --approve-lab
./target/release/opentatu --lab-config /absolute/path/lab.local.toml worker --once
```

The policy output is non-secret. Approval records an owner decision atomically with audit
creation. To approve an already queued audit, including a fixing-agent submission:

```bash
./target/release/opentatu --lab-config /absolute/path/lab.local.toml lab approve --audit AUD-DEMO-20261112-0900
```

Approve before a worker claims it. Each audit/commit needs its own approval, including a
fix commit; neither MCP nor the model can grant it. Rebuilding the controller or editing
any profile invalidates its digest and requires fresh approval. Do not replace the runner,
image or profile while an audit is running. `--lab-url` is legacy descriptive metadata; it
never enables requests to that URL. A configured lab or existing lab URL makes decisive
verification require matching dynamic proof; absent approval/unsupported cases stay
inconclusive. Static audits without a lab remain usable and explicitly report no dynamic
coverage. This release tests cross-identity read access for CWE-284/639/862 only; it does not validate arbitrary
injection, mutation or business-workflow claims. Register additional owner-reviewed cases
when the source/fixture supports them; CWE/file matching prevents unrelated proof reuse.

## Evidence and coverage

Attempts have transactional `LAB_TEST_STARTED`/`LAB_TEST_COMPLETED` events and a decided-once
`lab_runs` projection (migration 0009). Dates come from the host Clock. Receipts bind audit,
hypothesis, commit, case, attempt, nonce, profile, image, source and controller. The host
checks correlation/provenance and recomputes the outcome. Source hashing is length-framed
and includes ordered paths and contents. Only bounded text files enter the guest: no
symlinks, secret files, ignored dependency trees, binary files or files above the existing
read limit. Exclusions are limitations, never a claim of complete application coverage.

HTTP uses guest loopback, no proxy/redirect following, a three-second request timeout and
64 KiB response bound. Bodies are reduced to status, byte count, digest and fixture-marker
presence; raw responses and credentials are never persisted or sent to models. Model
`test_result`/`lab_request_response` evidence is discarded; the worker adds only an actual
system receipt. The persistence gate independently requires its matching completed row.

ZAP runs `requestor` for the same two GETs, waits for passive analysis and emits a minimized
traditional JSON report. No spider, active scan or model-selected target is enabled. ZAP
alerts use the existing scanner format and do not establish a finding by themselves. A
missing/mismatched ZAP version, truncated/oversized report or failed job remains untested.
Results appear on the audit dashboard, in `lab_runs.json`, `audit.json` and a system-owned
report section. A failed lab does not close a finding. Passing fix reverification still
means FIX_APPROVED; configured-branch merge detection alone closes it (R06, ADR-0032).

## Verification

Normal `cargo xtask check` uses `FakeLab`, `FakeClient`, temporary repos, in-memory SQLite
and FixedClock. It covers positive controls, source/approval mismatch, attempt caps,
transaction rollback, immutable history, host refusal, isolation argv, redaction, fenced
follow-up review and fix approval versus closure.

Use a release controller build for QEMU/TCG; a debug build adds substantial hashing and
copy overhead. Prewarm public dependencies offline in the prepared image using the same
compiler flags (including CGO_ENABLED) as the fixture build; no app source is needed for
that preparation. Keep acceptance files beside the image in persistent ignored storage.

The opt-in actual VM tests use a local Abastevo clone with an explicitly planted fictitious
GET fixture; they do not claim AI discovery of a previously unknown production flaw. Keep
`config/lab-acceptance.local.toml` outside git, with profile `abastevo`; its image's parent
must contain a local `abastevo` clone at the intended commit. The case handler is
`backend/cmd/api/phase8_lab.go`, fixture path `/v1/lab/accounts/owner-b`, identities
`owner-b`/`stranger-a` via `X-Lab-Actor`, marker `LAB_ONLY_ACCOUNT_B_726`. The handler permits
both fixture identities but initially omits their ownership comparison. Mount it after
router middleware registration. Build/run the actual API only inside the VM; readiness is
`/health/live`, with development, loopback HTTP and discovery disabled in fixture env. No
real Postgres is required for this lazy-pool liveness/fixture path. Source selects `backend`.

```bash
cargo test -p opentatu-worker --test lab_qemu r10_actual_abastevo_vm_reproduces -- --ignored --nocapture
# In the local clone only: require actor == account before returning the fixture; commit.
cargo test -p opentatu-worker --test lab_qemu r10_actual_abastevo_vm_refutes -- --ignored --nocapture
```

A minimal local-only vulnerable fixture is:

```go
package main

import (
    "net/http"
    "github.com/go-chi/chi/v5"
)

func phase8Account(w http.ResponseWriter, r *http.Request) {
    actor := r.Header.Get("X-Lab-Actor")
    if actor != "owner-b" && actor != "stranger-a" { http.Error(w, "unauthorized", 401); return }
    account := chi.URLParam(r, "accountID")
    if account != "owner-b" { http.NotFound(w, r); return }
    w.Header().Set("Content-Type", "application/json")
    _, _ = w.Write([]byte(`{"fixture":"LAB_ONLY_ACCOUNT_B_726"}`))
}
```

For the fixed commit, insert `if actor != account { http.Error(w, "forbidden", 403); return }`
before the response. Register `router.Get("/v1/lab/accounts/{accountID}", phase8Account)`
after middleware registration, then commit only in the disposable local clone. The Rust
acceptance tests execute the two identity requests against the actual API inside QEMU.

Each test runs the actual build, API, two identity probes and ZAP in a new offline VM and
checks the stored result. Minimized receipts, events and finding exports are retained under
a private `acceptance-*` directory beside the image, using an injected SystemClock and
explicit PARTIAL status for this known-case-only acceptance. The positive test creates a finding only with its stored receipt
and cited source. Never run all ignored tests together: they expect different commits.
Never push the fixture or the local fixes to Abastevo.

## Upstream references and prepared-image provenance

The boundary follows [QEMU invocation](https://www.qemu.org/docs/master/system/invocation.html)
and local [cloud-init NoCloud](https://docs.cloud-init.io/en/latest/reference/datasources/nocloud.html).
ZAP jobs/options follow its [requestor documentation](https://www.zaproxy.org/docs/desktop/addons/automation-framework/job-requestor/)
and [command-line documentation](https://www.zaproxy.org/docs/desktop/cmdline/).

The acceptance image was prepared from Ubuntu Noble cloud-image bytes with SHA-256
`04e8f2461a16a44e78a370284471f29616cdd9baf1893df25807b71f985b6c94`
(verified against upstream checksum metadata), plus official ZAP 2.17.0 Linux archive
`efe799aaa3627db683b43f00c9c210aea0b75c00cc8f0a0f0434d12bb3ddde5a`.
The resulting standalone image SHA-256 is
`8edce947095ec0b4fb1d156b3617d7af869f216f98350daadd1cafc0ffd873bf`.
The final 2026-11-13 image includes a 219 MiB public Go build cache prepared with
CGO_ENABLED=0, verified against the pinned public dependency manifests. The fixture's
TMPDIR/GOTMPDIR/HOME point into its bounded private workspace. A release-controller
acceptance reproduced the planted flaw in 8m38s and refuted it after the ownership check
in 9m00s (authorized 200 throughout, other identity 200 -> 403); both passive ZAP runs
completed. The immutable image digest remained unchanged.
Images, source copies, local recipes, fictitious credentials and raw logs are not repository
artifacts. New images require a new digest and owner approval.
