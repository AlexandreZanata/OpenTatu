# AI provider and execution architecture plan

Status: proposed. S5 reuses the existing provider only after runtime-data eligibility
review; S8 adds a second provider. Exact model IDs, URLs, limits and terms must come from
reviewed provider research, never this plan's memory or illustrative examples. Existing
configuration authority: [OpenCode Go research](../opencode-go.md).

## Four independent responsibilities

1. **Model provider:** implements `LlmClient::complete`, translating typed requests and
   normalized usage/errors. Current implemented provider: OpenCode Go.
2. **Agent runtime provider:** hosts an agent loop and proposes tools/results. The first
   runtime is OpenTatu's bounded role loop. Codex CLI or Claude Code CLI would be optional
   external runtimes, subject to their actual API, automation terms and isolation policy.
3. **Tool execution runtime:** authorizes and performs a typed operation within explicit
   IO and capability bounds; it never trusts provider claims as authority.
4. **Orchestrator:** deterministic task admission, budget reservation, routing, leases,
   cancellation and state transitions. It remains OpenTatu-owned when providers change.

Identity/history/grants belong to the agent definition and task, not to any model session.
A provider switch creates a configuration/version event, retaining the same identity and
authorized memory. Permissions are never inferred from a model's capabilities.

## Actual foundation and extensions

Reuse `Llm`, `LlmClient`, `FakeClient`, `UsageRecorder`, `CallContext`, strict role schemas,
`run_role`, retry policy, `Secret`, training guard and allowlist. Current `ModelsConfig`
contains one provider and six fixed Audit roles. `CallContext` links an Audit, not an
incident/task. Current API has no native tool calling or provider registry. The per-audit
`BudgetGuard` checks observed completed usage, without in-flight reservations. The
configured monthly `max_plan_share` is not a proven enforced runtime cap in this baseline.

Extend the existing path with provider IDs, model capability metadata, a purpose/data
classification policy and task/incident/agent context. Keep backward-compatible Audit
configuration parsing and existing model assignments. Add a discriminated workload context
or optional links, with validation against contradictory IDs. Extend existing `llm_usage`
accounting via a new migration; do not create another spend ledger with divergent totals.
Provider adapters return normalized usage while preserving provider-reported provenance.

## Routing policy and configuration

Proposed per-agent binding:

```toml
# Illustrative configuration; placeholders are invalid until researched and authorized.
[agent_bindings.investigator]
provider_policy = "approved-private-runtime-data"
model_policy = "structured-analysis"
budget_policy = "bounded-incident"
fallback_policy = "same-or-stricter-data-policy"
```

Keep provider endpoint/model facts in the existing models configuration and research
record; use references in team configuration. Resolve only an explicitly registered model
with required context, structured-output and tool/runtime capabilities. Reject unknown
capabilities rather than assuming every “compatible” API behaves identically. Default
deny new models, data classes and remote destinations. The composition root reads secrets;
config contains credential references, never keys. The ZaturoBot bot/app may present
only approved bindings through a future authenticated service contract; it never receives
OpenTatu's provider key or overrides purpose, scope, provider eligibility or budgets.

Under ADR-0038, OpenTatu remains the service and ZaturoBot owns client presentation.
The bot's own model/task system is not an OpenTatu provider grant or a replacement for
OpenTatu's orchestrator/accounting. Existing ZaturoBot credentials cannot select arbitrary
models, send incident data to a different provider or invoke response tools. Q-040 must
define the narrow integration principal and trusted owner context; see the
[integration plan](ZATUROBOT_INTEGRATION_PLAN.md). No runtime telemetry is shared with a
bot-side model merely because it arrived through a conversation.

The inspected ZaturoBot model/tool path is scoped to bounded public-coding diagnostics.
Keep OpenTatu management data outside that path by default. A sanitized service view is
still disclosure to an additional system and requires an approved field policy; ordinary
chat routing must not turn it into a model prompt. ZaturoBot coding-provider consent and
OpenTatu Audit-role consent each remain insufficient for runtime incident material.

Routing order: purpose/data eligibility -> model allowlist -> capability/context bounds ->
shared budget reservation -> provider circuit state -> incident priority -> dispatch.
Fallback must meet the same or stricter privacy and capability rules; it cannot silently
choose a training-enabled model or spend outside the incident envelope. One failing
provider must not be multiplied into unbounded retries across other providers.

## Durable budgets and failure handling

Add atomic reservations through services/db before dispatch, covering call count,
configured maximum output tokens, estimated input and known tariff bounds. Reconcile
actual usage afterward; a crash leaves a conservative reservation until reconciled or
expired by explicit policy. Unknown cost is `unknown`, never zero. Disable monetary
routing without a reviewed price basis; retain hard call/token/time caps. Track tokens,
reported/estimated cost, retries, latency, errors, role, provider/model, prompt/schema
versions and task lineage without logging content or private reasoning.

Enforce incident/app/month/global limits and the owner's existing shared OpenCode Go plan
share. Budget allocation between Audit reverification and urgent Sentinel investigations
is an owner decision; preserve Audit priorities until changed explicitly. Provider-side
limit windows and prices remain provider research facts, independent from local quotas.
No plan asserts that a provider accepts permanent security monitoring under a personal
coding subscription.

Use capped retries for transient failure only, per-provider rate limiting and a circuit
breaker with deterministic half-open probes driven by actual work. No paid probe while
idle. Cancellation/deadline prevents another turn or tool. Degraded mode uses deterministic
incident summaries and notification templates; pending investigations remain visible.

## Progressive provider support and release gates

- S5: existing OpenCode Go adapter, only if updated terms/privacy review authorizes the
  runtime workload. If eligibility is unresolved, ship read-only tools and no AI calls.
- S8: one owner-selected second API provider from DeepSeek, OpenAI, Anthropic or another
  documented API. Research exact contract, auth, errors, capabilities, billing, retention,
  training use, region and automation terms before implementing it. This is a roadmap
  option list, not a recommendation that any particular subscription grants API access.
- Later: Ollama or equivalent on separately sized hardware; local models are not assumed
  feasible on the small Sentinel VPS. Treat configured local endpoints as trusted network
  targets with no arbitrary URL access. Keep resource isolation from monitoring.
- Later: external agent CLI integration only after explicit executor ADR, current terms
  verification and isolation proof. Do not add shell access to an internal AI role or
  reuse a consumer login as an unrestricted API credential.

The existing owner's consent to training-enabled Audit skeptic/reporter models in
ADR-0022 does not authorize runtime telemetry. Classify incident data and secure explicit
purpose-scoped consent before sending any of it. Add provider research documents only
when that provider is selected; keep IDs/limits/URLs in those reviewed sources and config.

## Acceptance and ownership

Scenario F disables every provider while ingest/detect/notify remain operational. Scenario
J changes a configured provider without changing agent identity/history/grants. Contract
tests use local mock endpoints and the existing fake; validate output truncation, malformed
JSON, unsupported tools, rate limits, missing cost, cancellation and partial failure.
Concurrent reservation tests prove shared caps cannot be over-admitted; retries/fallbacks
must consume the same caps. Test privacy-denied fallback without a network call. Never
use a real provider key in CI, fixtures, artifacts or history.
