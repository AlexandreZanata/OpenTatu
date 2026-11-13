# opentatu-scanners — static containers and offline dynamic lab

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 2 ✅ (Semgrep,
Gitleaks, Trivy fs, OSV-Scanner)

## Responsibility

- **Does:** run Semgrep, Gitleaks, Trivy (fs) and OSV-Scanner offline in disposable
  containers, refresh their rules/databases in separate networked runs, and normalize their
  raw output to `opentatu_core::ScannerResult` (ADR-0023).
- **Does not:** decide what is a vulnerability (the hunter triages), touch the database
  or receive provider/production application API keys (R16). Host inventory uses only
  the explicitly configured dedicated SSH identity file, never an administrative key.

## Files

| File | Contents |
|---|---|
| `src/lib.rs` | module list and re-exports |
| `src/isolation.rs` | `ISOLATION_FLAGS` (§E.5, single list), `ContainerRun::command_line`, `ContainerEngine`, `Network`, `Mount`, `SOURCE_TARGET` |
| `src/config.rs` | `ScannersConfig`, `EngineConfig`, `ToolConfig` — `config/scanners.toml`, validated |
| `src/suite.rs` | `ScannerSuite` port; `Scanners` (`run`, `update`, `ready`, `cache_dir`) |
| `src/engine.rs` | `command`, `execute`, `lab_execute`, `guest_child` — the shared process boundary (R12) |
| `src/adapters/mod.rs` | `Adapter` trait, `adapter(kind)`, helpers `clean_message`, `relative`, `first_cwe`, `parse_json`, `base_run` |
| `src/adapters/{semgrep,gitleaks,trivy,osv}.rs` | one adapter per tool: analysis run, update runs, readiness, parser |
| `src/testing.rs` | `FakeSuite`, `FakeOutcome` (feature `test-support`) |
| `src/error.rs` | `Error` |
| `tests/fixtures/<tool>.json` | trimmed real outputs (no real secrets) used by the parser tests |
| `tests/containers.rs` | `#[ignore]`d test with real containers (`--ignored` to run) |

## Local rules

- The only crate allowed to start scanner/lab processes (R12): `engine::command` is the
  single construction site. Static execution, bounded host helpers and capability-gated
  guest children use it; the one clippy exception stays there with its rule comment.
- Never build engine arguments by hand: `ContainerRun::command_line()` only. Analysis runs
  are `Network::None`; a `Network::Default` run that mounts `SOURCE_TARGET` is refused.
- Images pinned by digest in `config/scanners.toml`; the version there is reported in
  `audit.json` (R17). Tool arguments live only in the tool's adapter.
- Secrets never leave a tool unredacted (R15): Gitleaks runs with `--redact` and its
  normalized message never uses `Secret`/`Match`.
- Telemetry off; containers run as the data directory's owner with `HOME=/tmp`.
- Messages are collapsed and capped (`clean_message`); paths are repository-relative.

## How to extend

[../../docs/recipes/add-scanner.md](../../docs/recipes/add-scanner.md)

## Tests

Parser tests over `tests/fixtures/`, command-line invariants in `isolation.rs`, config
validation over the committed `config/scanners.toml`. Real containers:
`cargo test -p opentatu-scanners --test containers -- --ignored --nocapture`.

Phase 8 (ADR-0034): `lab_config.rs` validates explicit owner profiles and redacts fixture
values; `lab.rs` implements `LabSuite` / `QemuLab`, fixed NIC-less QEMU argv, pinned image
and controller, bounded serial receipts. `lab_guest.rs` owns the host-refusing guest
capability, private source/build workspace, separate uid/gid and bounded GET controls.
`lab_zap.rs` owns the passive requestor/wait/report plan and minimized parser. The shared
`engine::command` is the sole process-construction site; guest application commands require
the offline guest capability. `testing::FakeLab` reuses the same policy/receipt port.
Static containers iterate `ScannerKind::STATIC`; ZAP cannot use their adapters. See
[../../docs/LAB.md](../../docs/LAB.md) for the actual VM acceptance and image requirements.

Phase 9 (ADR-0035): `host_config.rs` validates explicit SSH profiles and private credentials;
`host.rs` implements `HostSuite`/`SshHosts`, pinned trust, fixed argv and strict receipts;
`host_collect.rs` implements the unprivileged forced collector. `engine::bounded_execute`
reuses the existing process constructor for lab helpers and inventory, with cleared
environments and capped output/deadlines. `testing::FakeHosts` reuses the same port. No
general SSH command API or raw diagnostics are exposed. The only real credential consumed
by SSH is its dedicated owner-private identity file; the provider API key is never passed.
The opt-in local fixture lives in `tests/host-fixture/`; see `docs/HOST_INSPECTION.md`.
