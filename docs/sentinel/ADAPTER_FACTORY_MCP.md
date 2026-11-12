# Adapter Factory MCP plan

Status: proposed for S4, after the read-only monitoring MVP. The Factory standardizes
adapter development by external coding agents; it is separate from runtime incident
investigation. It builds on phase 5 MCP infrastructure when that infrastructure exists.
Contracts: [adapter protocol](ADAPTER_PROTOCOL.md). Security:
[platform model](SENTINEL_SECURITY_MODEL.md).

## Profiles and authority

Keep one distributed Rust executable. Proposed entry points are `opentatu mcp --profile
adapter-factory` and `opentatu mcp --profile sentinel-investigation`; the existing phase 5
fixing profile remains separately scoped. These commands are future interfaces, not
available commands today. The product name `opentatu-adapter-factory` names a profile and
contract, not a mandatory second server binary.

Use separately launched sessions, credentials, grants and tool catalogs. A Factory token
cannot query live incident evidence or invoke a response; an Investigation token cannot
scaffold, install or activate adapters. Never run the Factory with production credentials
or mounts. Stdio is the first transport under ADR-0012. A remote MCP transport needs its
own accepted ADR and protocol authorization implementation.

The standard distinguishes HTTP authorization from stdio credential handling; HTTP
requires audience validation and must not pass upstream tokens through. Follow the
[MCP authorization specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
and [security guidance](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices)
when a remote profile is approved. No private agent protocol is proposed.

## Essential development tools

Tool names below are proposals. Each is a thin interface over a `Services` use case;
authorization checks and transitions are independent of model instructions.

- `opentatu_adapter_inspect_repository`: read an owner-registered snapshot by handle and
  exact commit. Returns bounded tree/stack hints/telemetry candidates and evidence refs.
  Stack detection and telemetry discovery are modes of this tool initially, avoiding
  three nearly identical implementations.
- `opentatu_adapter_list_templates`: list signed or locally trusted versioned templates,
  supported contract versions, capabilities and fixture coverage.
- `opentatu_adapter_get_contract`: return a supported contract/schema plus compatibility
  rules and synthetic example by version.
- `opentatu_adapter_scaffold`: materialize a selected trusted template in an approved
  isolated workspace. Requires a grant bound to snapshot, template digest and write scope.
  Its result includes the initial manifest and documentation, so separate manifest/docs
  generators are unnecessary in the MVP.
- `opentatu_adapter_validate`: pure/static validation of manifest, schema, grant requests,
  forbidden files and compatibility; returns typed problems and report digest. Permission
  checking is a mode, not a separate divergent policy engine.
- `opentatu_adapter_run_tests`: run an allowlisted fixture test profile through the
  isolated executor, with no network, secrets or production mounts. Returns structured
  test results and evidence references, with bounded stdout/stderr after masking.
- `opentatu_adapter_package`: create a reviewable content-addressed artifact from a
  validated workspace; include provenance, manifest, docs, tests and permission diff.
- `opentatu_adapter_status`: inspect development/package/installation status in scope.

Deferred aliases: `detect_stack`, `discover_telemetry`, `create_manifest`,
`check_permissions`, `generate_docs` until they justify independent contracts. MVP
installation uses the owner-operated local interface with an exact package approval.
`opentatu_adapter_install` is deferred; if added, it consumes a single-use installation
grant and only stages the artifact. Activation is always a separate capability and gate.

## Shared tool envelope and concrete schemas

Server authenticates `principal_id`; clients cannot set it or their effective grants.
Inputs carry explicit app, immutable resource handles and idempotency where relevant.
No arbitrary host path, shell command, SQL, URL, network destination or environment map is
accepted. Resolve handles against the session's authorized registry.

Example input schema for validation (illustrative v1; generate normative schemas from the
same domain contracts rather than maintaining copies):

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["application_id", "workspace_id", "contract_version"],
  "properties": {
    "application_id": {"type": "string", "minLength": 1, "maxLength": 64},
    "workspace_id": {"type": "string", "minLength": 1, "maxLength": 128},
    "contract_version": {"type": "string", "enum": ["1.0"]},
    "mode": {"type": "string", "enum": ["all", "permissions", "contract"]}
  }
}
```

Example scaffold command:

```json
{
  "application_id": "demo",
  "snapshot_id": "registered-snapshot-handle",
  "template_id": "json-event-source",
  "template_digest": "sha256:<validated-template-digest>",
  "workspace_id": "approved-isolated-workspace",
  "development_approval_id": "single-use-scaffold-grant",
  "idempotency_key": "synthetic-command-1"
}
```

Example result:

```json
{
  "tool_call_id": "server-assigned-call-id",
  "status": "COMPLETED",
  "resource_id": "package-or-workspace-handle",
  "report_digest": "sha256:<computed-report-digest>",
  "evidence_references": ["registered-test-report"],
  "problems": [],
  "next_allowed_steps": ["package"]
}
```

Errors have stable codes (`UNAUTHORIZED`, `OUT_OF_SCOPE`, `INVALID_INPUT`,
`APPROVAL_REQUIRED`, `STALE_APPROVAL`, `LIMIT_EXCEEDED`, `ISOLATION_UNAVAILABLE`,
`CONTRACT_UNSUPPORTED`, `TEST_FAILED`) and masked explanations. Schema bounds and policy
grants both apply; a syntactically valid tool call is not an authorized call.

Every invocation has a correlation ID, request digest, server times, principal, app,
profile, outcome and execution limits. Preserve Q-007: a domain transition records its
domain event; a read-only call records `MCP_TOOL_CALLED`, exactly one canonical event per
call. Multi-step work emits separate task/run events rather than duplicate call events.
Errors and denied calls receive a sanitized call event as well.

## Approved workflow

1. Owner registers repository scope and authorizes reading one snapshot. Private-repo
   credentials stay in the owner's source-access mechanism, never in manifests or prompts.
   Arbitrary URL fetching is prohibited; prevent SSRF, redirect and git-option injection.
2. Inspection produces a capability proposal with concrete file references, minimized
   data fields, grant requests and expected application changes. Repository documentation
   is untrusted evidence, including files that claim to be agent instructions.
3. Owner approves development scope before scaffold or writing. External coding agents
   may implement in their own isolated checkout; this does not give internal security
   roles disk, shell or network access under R12.
4. Validate static contracts, then run only the approved sandbox test profile. Never run
   repository install scripts, hooks or build scripts on the OpenTatu host. Dependency
   preparation, if needed, is an independently approved networked stage without source
   secrets; fixture test execution is offline.
5. Package a diff, manifest, provenance, license inventory, tests and rollback instructions.
   Owner sees exactly which repository and files would change. A package is inert.
6. Owner separately approves installation tied to artifact digest and effective grants.
   Installation provisions a disabled adapter and scoped credential; activation verifies
   source reachability and starts a controlled synthetic test.
7. Record health and protocol version. A failed activation revokes new credentials,
   disables the new installation and leaves the prior compatible version available.

## Isolation and execution boundary

Reuse existing hardened snapshots for repository reads and scanner isolation construction
for controlled test containers. The existing scanner API is not automatically a safe
general build runner: at S0 decide a typed extension versus a dedicated executor crate,
record dependency edges in an accepted ADR, and retain all existing isolation tests.
No `Command::new` exception is added here. A future executor has allowlisted profiles,
digest-pinned images, read-only source, dedicated bounded output, no host credentials,
no Docker socket, non-root user, quotas and timeout/kill cleanup. Symlinks, path traversal,
archive extraction and output references are independently validated.

If safe execution cannot be supported within the current boundary, the Factory still
delivers static contracts and a package for an external isolated agent; it reports
`NOT_EXECUTED` for sandbox tests and cannot mark the adapter installable. Absence of a
test runner must never be reported as successful validation.

## Portability proof and acceptance

S4 creates a synthetic adapter from a different stack, with no Abastevo naming or rules
in the collector/detector. It emits the same event contract, passes the same conformance
suite and produces an incident through existing rules. S11 repeats this against a second
real owner-authorized app. Both are required: a fixture proves contract portability; a
real app tests the discovery and integration assumptions.

Security tests reject prompt injection, cross-app workspace access, unauthorized writes,
changed digests, approval replay, privilege requests, path traversal, malicious manifests,
untrusted scripts and attempts to query production. Cancellation and server restart cannot
leave unbounded containers or silently advance an approval. Every failure is reviewable
without disclosing secrets.
