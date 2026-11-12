# Security of OpenTatu itself

OpenTatu stores code, keys and the open flaws of every application of its owner: it is the
most sensitive system the owner has (spec §E.8). This document is the threat model and the
list of controls; the requirements are in the spec (R10–R16, §E.5, §E.8).

## Threat model (local-first)

| Threat | Control |
|---|---|
| Malicious website attacks the local dashboard (DNS rebinding, CSRF, clickjacking) | Loopback bind only; `Host` allowlist; CSP without scripts, `frame-ancestors 'none'`; `SameSite=Strict` cookies and CSRF tokens on every form (when forms arrive) |
| Other local users read data | Data directory created `0700`; database inside it |
| Analyzed code attacks the AI (prompt injection, R11) | Inputs only in fenced blocks (`fence`); base prompt principle 2; suspicious instructions recorded as events; AI has no tools beyond what OpenTatu exposes (R12) |
| Analyzed code attacks the host through scanners | Containers with `--network=none`, read-only root FS, no capabilities, `no-new-privileges`, memory/CPU/PID limits, code mounted read-only; images pinned by digest; rootless Podman recommended |
| Secret leakage (R15, R16) | API key only in the environment, read in one place, never logged or stored; secrets in findings masked with `mask_secret`; no prompt/code content in logs |
| History tampering (R04) | `events` table rejects UPDATE/DELETE with triggers; corrections are new events |
| Fixing agent closes or downgrades a finding | MCP tools cannot close findings or change severity (R06); only reverification closes |
| Dynamic tests hit production (R10) | Dynamic verification only against the lab URL, fictitious data; production out of scope in every prompt |
| Supply chain | Minimal dependencies, `default-features = false`, single declaration point, lockfile committed; CI on every push |
| Loss of history | Encrypted backups of `data/` (owner's responsibility; recipe planned) |

## Controls in code (where to look)

| Control | Location |
|---|---|
| Loopback-only config | `opentatu_dashboard::DashboardConfig::local` |
| `Host` allowlist, security headers, no-store | `crates/dashboard/src/security.rs` |
| Fencing untrusted content | `opentatu_investigator::fence` |
| Secret masking | `opentatu_core::mask_secret` |
| Container isolation | `opentatu_scanners::{ISOLATION_FLAGS, ContainerRun}` |
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
