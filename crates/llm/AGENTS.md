# opentatu-llm — model provider abstraction

Layer: **infrastructure** · May depend on: `opentatu-core` · Status: base ✅ (router, config,
fake); OpenCode Go client = phase 0

## Responsibility

- **Does:** the single entry point `Llm::generate(role, messages, schema)` (§D.2), per-role
  model routing from `config/models.toml`, provider clients behind `LlmClient`, and (phase 0)
  budget enforcement and `llm_usage` accounting.
- **Does not:** know prompt texts or output schemas (that is `opentatu-investigator`), read
  environment variables (the binary passes the key in), log request contents.

## Files

| File | Contents |
|---|---|
| `src/router.rs` | `Llm` (routing; accounting hooks added in phase 0) |
| `src/client.rs` | `LlmClient` trait, `Message`, `MessageRole`, `ModelRequest`, `ModelResponse` |
| `src/config.rs` | `ModelsConfig` and parts; `validate()`; test that `config/models.toml` parses |
| `src/fake.rs` | `FakeClient` — the only test double for model calls |
| `src/error.rs` | `Error` (`RateLimited`, `Authentication`, ...) |

Planned (phase 0, **after** `docs/opencode-go.md` is complete): `opencode_go.rs`
(`LlmClient` over HTTPS, mapping rate limits to `Error::RateLimited`), usage accounting,
exponential backoff with a ceiling.

## Local rules

- Model ids, base URL and limits come only from `config/models.toml`, filled from
  [../../docs/opencode-go.md](../../docs/opencode-go.md) — never from memory.
- R16: the API key is a value passed in by the binary; never log it, never put it in errors,
  events, reports or scanner environments.
- Errors must not contain request contents (they may hold analyzed code).
- Every model call goes through `Llm` so accounting and budget live in one place.

## Tests

Use `FakeClient`; never call a real provider in tests. `cargo test -p opentatu-llm`.
