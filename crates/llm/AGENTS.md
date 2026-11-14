# opentatu-llm — model provider abstraction

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 0 ✅ (OpenCode Go
client, router, guards, retries, accounting port)

## Responsibility

- **Does:** the single entry point `Llm::generate(context, role, messages, schema)` (§D.2),
  per-role routing from `config/models.toml`, the model allowlist and the guard against models
  that train on prompts, retries of rate limits and transport failures (never of timeouts),
  usage recording (`UsageRecorder`), and provider
  clients behind `LlmClient` (`OpenCodeGoClient`).
- **Does not:** know prompt texts or output schemas (that is `opentatu-investigator`), read
  environment variables (the binary passes the key in), log request contents.

## Files

| File | Contents |
|---|---|
| `src/router.rs` | `Llm`: `generate`, `ping`, allowlist, training guard, retries, recording |
| `src/opencode_go.rs` | `OpenCodeGoClient` (Chat Completions, Bearer, User-Agent, `x-opencode-session`) |
| `src/context.rs` | `CallContext` (audit + provider session id) |
| `src/retry.rs` | `RetryPolicy` (capped exponential backoff for `RateLimited` and `Transport`) |
| `src/usage.rs` | `UsageRecorder` port (implemented by `opentatu_worker::ServicesUsageRecorder`) |
| `src/campaign.rs`, `src/campaign_tests.rs` | Optional `CampaignBudget`: exclusively locked append-only billing ledger; durable full-context reservations before every transport attempt, finite outputs, actual-charge settlement and conservative unknown outcomes |
| `src/client.rs` | `LlmClient` trait, `Message`, `MessageRole`, `ModelRequest`, `ModelResponse` |
| `src/config.rs` | `ModelsConfig` and parts; `validate()`; `set_role_model`, `set_audit_budget` (benchmark variants — still validated); test that `config/models.toml` parses |
| `src/fake.rs` | `FakeClient` — the only test double for model calls |
| `src/error.rs` | `Error` (`RateLimited`, `Authentication`, ...) |

Per-audit budget enforcement and `llm_usage.jsonl` belong to the worker. The monthly
plan share is configured and reported; campaign admission adds its own durable aggregate
ceiling rather than claiming a global monthly reservation across data directories.

Campaign runs explicitly add `Llm::with_campaign_budget`. Reuse the exact same ledger path
and unchanged spending cap/model/endpoint/input ceiling/prices across all runs. The guard uses researched `ModelsConfig::campaign`
ceilings; every retry reserves separately. Missing, malformed, timed-out and interrupted
responses retain the full reservation; model/token/price violations halt the campaign.
The ledger contains only billing metadata and remains private runtime data. Regular usage
recording and per-audit limits still apply independently.

Provider responses with an existing choice and empty/null/whitespace content still carry
usage metadata. Preserve that metadata through `Llm` accounting before the investigator
rejects or retries the role answer. Malformed provider envelopes and missing charge data
remain uncertain; older reservations are never retroactively refunded. Diagnostic `ping`
requires a JSON object after accounting; it does not accept blank or malformed content.

The current output ceiling is 131,072 tokens, reserving US$0.457287 per attempt at peak
prices. Older 32,768-token attempts retain their original US$0.339322 reservation until
settled. An increase appends a synced `output_limit_raised` record; replay validates
historical reservations against the ceiling active for their own attempt. Decreases and
all fixed-policy changes fail closed. The aggregate spending cap never changes.

`RoleConfig::max_output_tokens` is optional and includes reasoning. `Llm::generate` uses
it directly; omitted settings preserve the provider default, and zero is rejected by
configuration validation. The campaign guard clamps it to its financial ceiling without
reducing the full per-attempt reservation. These experimental generation settings change
neither timeout classification nor retries.

## Local rules

- Model ids, base URL and limits come only from `config/models.toml`, filled from
  [../../docs/opencode-go.md](../../docs/opencode-go.md) — never from memory.
- R16: the API key is a value passed in by the binary; never log it, never put it in errors,
  events, reports or scanner environments.
- Errors must not contain request contents (they may hold analyzed code).
- Every model call goes through `Llm` so guards, retries and accounting live in one place;
  never call an `LlmClient` directly.
- OpenCode Go rejects calls without `x-opencode-session`: always pass a `CallContext`
  (`for_audit` for role calls, `diagnostic` for checks).

## Tests

Use `FakeClient`; never call a real provider in tests. `cargo test -p opentatu-llm`.
