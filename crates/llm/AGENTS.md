# opentatu-llm — model provider abstraction

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: phase 0 ✅ (OpenCode Go
client, router, guards, retries, accounting port)

## Responsibility

- **Does:** the single entry point `Llm::generate(context, role, messages, schema)` (§D.2),
  per-role routing from `config/models.toml`, the model allowlist and the guard against models
  that train on prompts, rate-limit retries, usage recording (`UsageRecorder`), and provider
  clients behind `LlmClient` (`OpenCodeGoClient`).
- **Does not:** know prompt texts or output schemas (that is `opentatu-investigator`), read
  environment variables (the binary passes the key in), log request contents.

## Files

| File | Contents |
|---|---|
| `src/router.rs` | `Llm`: `generate`, `ping`, allowlist, training guard, retries, recording |
| `src/opencode_go.rs` | `OpenCodeGoClient` (Chat Completions, Bearer, User-Agent, `x-opencode-session`) |
| `src/context.rs` | `CallContext` (audit + provider session id) |
| `src/retry.rs` | `RetryPolicy` (capped exponential backoff) |
| `src/usage.rs` | `UsageRecorder` port (implemented by `opentatu_worker::ServicesUsageRecorder`) |
| `src/client.rs` | `LlmClient` trait, `Message`, `MessageRole`, `ModelRequest`, `ModelResponse` |
| `src/config.rs` | `ModelsConfig` and parts; `validate()`; test that `config/models.toml` parses |
| `src/fake.rs` | `FakeClient` — the only test double for model calls |
| `src/error.rs` | `Error` (`RateLimited`, `Authentication`, ...) |

Planned: per-audit budget enforcement and the 60% monthly share (phase 4, with the worker).
`llm_usage.jsonl` per audit is written by the worker (phase 3).

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
