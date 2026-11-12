# Part D — OpenCode Go integration (the MVP's API key)

## D.1 Mandatory research before writing any AI client code

The key used in the MVP belongs to OpenCode's **Go** plan. Before implementing the model
client, research and read the OFFICIAL, CURRENT OpenCode documentation about the Go plan
(official site opencode.ai and its documentation section). Do not use URLs, model names,
limits or formats from memory: everything must be confirmed in the documentation in force
on the date of the consultation.

Confirm and record:

1. API base URL and whether it is OpenAI-compatible (chat completions or another format).
2. Authentication format (header, token prefix).
3. Models available in the plan and the EXACT identifier of **DeepSeek Flash**. Also check
   which other models exist, so the skeptic can use a different model.
4. Context window and output limit of each model used.
5. Support for: structured JSON output, tool calling, streaming.
6. Plan usage limits (per period), error codes when the limit is hit (e.g. 429) and rate
   limit headers, if any.
7. Terms of use: whether the plan allows API use for personal automation such as OpenTatu.
   **IF THE DOCUMENTATION SAYS IT IS NOT ALLOWED, STOP AND TELL THE OWNER BEFORE CONTINUING.**
8. Retention and data-use policy (the applications' code will be sent to the provider).

Record everything in [../opencode-go.md](../opencode-go.md) with: date and time of the
consultation; exact links of the pages read; summarized relevant excerpts; ambiguous or
missing points marked "not confirmed".

## D.2 Client implementation rules

- Key in the environment variable `OPENCODE_GO_API_KEY`. Never in code, commits, logs,
  reports, events or scanners (R16). Read only in `apps/opentatu/src/config.rs`.
- Base URL and model IDs in configuration — [`config/models.toml`](../../config/models.toml)
  (TOML, ADR-0008) — never hardcoded. Structure: `[provider]` (name, base_url,
  api_key_env), `[roles.<role>]` (model, temperature: cartographer 0.1, hunter 0.4,
  verifier 0.0, skeptic 0.2 with a different model if available, planner 0.1, reporter
  0.3), `[budget]` (`max_plan_share = 0.60`; per-audit limits for FULL 90 min,
  INCREMENTAL 30 min, REVERIFICATION 15 min; tokens and calls to be defined).
- Abstraction layer (`opentatu-llm`): a single interface `generate(role, messages, schema)`
  to switch provider or model without touching the roles. It must allow plugging in a local
  model in the future for more sensitive applications.
- Account tokens and calls of every request in `llm_usage.jsonl` and in the `llm_usage`
  table (never the key, never the code sent).
- On a rate-limit error: exponential backoff with a ceiling; if it persists, pause the audit
  with a checkpoint (`AUDIT_PAUSED`).
- Validate every response with a schema; invalid JSON = at most 2 new attempts, then record
  the step failure.
- Command `cargo run -p opentatu -- llm ping` makes a minimal call and prints model, latency
  and tokens. This is the done criterion of the integration (phase 0).
