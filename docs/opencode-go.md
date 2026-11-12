# OpenCode Go — research record (spec §D.1)

> **Status: DONE (phase 0).** Every item below was read from the official documentation
> and, where possible, confirmed against the live API with the owner's test key.
> `config/models.toml` is filled from this record. Re-check this page before changing
> models, URLs or limits — the provider changes them over time.

Consultation: 2026-11-12, between 06:10 and 06:30 (-03:00, horário de Brasília).

Consulted by: Claude Opus 5.5 (Claude Code), on the owner's request.

## Pages and endpoints read

| Source | What it gave |
|---|---|
| https://opencode.ai/docs/go/ | Plans, endpoints per API style, model list with prices and monthly limits, usage windows, where-can-I-use-it guidance, privacy and retention per model |
| https://opencode.ai/legal/terms-of-service (effective 2026-11-08) | General terms: internal use only, prohibited conduct, data use |
| `GET https://opencode.ai/inference/go/v1/models` (live, public) | Exact model ids, context/output limits, capabilities, API style per model |
| `POST https://opencode.ai/inference/go/openai/v1/chat/completions` (live, test key) | Auth format, required headers, response and usage format, error format |

## Findings

| # | Item (spec §D.1) | Result | Source |
|---|---|---|---|
| 1 | Base URL; OpenAI-compatible? | **Yes.** Chat Completions at `https://opencode.ai/inference/go/openai/v1/chat/completions` (base URL `https://opencode.ai/inference/go/openai/v1`). The Go page also lists Responses, Anthropic Messages and Google endpoints and says text models also work through the other API styles (translated). | docs/go, live call |
| 2 | Authentication | `Authorization: Bearer <key>` (keys look like `oc_sk_...`). `X-Api-Key` is also accepted by CORS headers (not used). **Also required:** `x-opencode-session` — calls without it fail with HTTP 400 `MissingSessionID`. The Go page asks direct clients to send a product `User-Agent` and a stable session id per conversation. | live call, docs/go "Where can I use it?" |
| 3 | Models; exact DeepSeek Flash id; alternative for the skeptic | DeepSeek Flash = **`deepseek-v4.1-flash`** (name "DeepSeek V4.1 Flash 1017", released 2026-10-17). Owner-allowed alternative: **`muse-spark-1.3-contributor`** (Meta). Other ids exist (30 in total) but the owner restricted the key to these two. | live `/v1/models` |
| 4 | Context and output limits | `deepseek-v4.1-flash`: context 1,000,000, output 384,000. `muse-spark-1.3-contributor`: context 1,048,576, output 131,072. | live `/v1/models` (`limit`) |
| 5 | Structured JSON, tools, streaming | Both: `structured_output: true`, tools supported, reasoning supported. JSON mode (`response_format: {"type": "json_object"}`) confirmed live for both. DeepSeek returns its reasoning in `reasoning_content`. Streaming: not documented for Go; not needed. Muse Spark is listed under the Responses API but works through Chat Completions (confirmed live). | live `/v1/models` (`capabilities`), live calls |
| 6 | Usage limits, limit errors, rate-limit headers | Limits are **monthly dollar amounts per model**: Go plan gives `deepseek-v4.1-flash` **$100/month** and `muse-spark-1.3-contributor` $60/month; each model also has a 5-hour window (20%) and a weekly window (50%). Every response reports the charge in `usage.opencode.allowance` (USD). No rate-limit headers observed; the error code when a limit is hit is **not documented** (client treats 429 as rate limit). With Console balance, Go may **fall back to pay-as-you-go** after the limits instead of blocking. | docs/go, live calls |
| 7 | **Terms: personal automation allowed?** | **Allowed with conditions.** The Go page says the plan is meant for OpenCode and other coding agents that send similar requests, documents SDKs for applications that call Go directly, monitors traffic for abuse, and asks direct clients to identify themselves (own `User-Agent`) and send `x-opencode-session`. The terms of service limit use to the customer's own internal use (OpenTatu is personal and internal). One prohibited-conduct clause covers programmatic extraction of data or output; in context (next to crawling and scraping) it targets scraping, not the API clients the Go docs invite — **flagged to the owner**. Nothing prohibits analyzing the security of one's own software. | docs/go, ToS |
| 8 | Retention and data use | `deepseek-v4.1-flash`: **not used for training**, 0-day retention under a zero-data-retention agreement that OpenCode **renews monthly — valid through 2026-11-30**; requires the workspace privacy region **Global**. `muse-spark-1.3-contributor`: **prompts and completions are used to train Meta models; not zero-data-retention**; requires enabling paid endpoints that train on request data in the privacy settings. ToS: content sent to third-party models follows those providers' retention policies. | docs/go (Privacy), ToS |

## Live checks (2026-11-12, test key)

- `deepseek-v4.1-flash` and `muse-spark-1.3-contributor` answered `{"pong": true}` in JSON
  mode via Chat Completions; `opentatu llm ping` recorded both calls in `llm_usage`
  (tokens, reasoning tokens, `allowance` cost ≈ $0.00005 each).
- Missing `x-opencode-session` → HTTP 400 `{"type":"error","error":{"type":"MissingSessionID",...}}`.
- Invalid key → HTTP 401 `{"error":{"type":"authentication_error","message":"Invalid credential"}}`.
- Unknown model → HTTP 400 `{"error":{"type":"server_error","message":"Upstream request failed: Model is unavailable."}}`.
- **The key is not restricted server-side:** one 5-token probe to `glm-5.3` succeeded.
  OpenTatu therefore enforces the owner's restriction itself (`[provider] allowed_models`;
  `opentatu llm ping --model glm-5.3` is refused without any network call).

## Decisions taken from this research

- Every AI role uses `deepseek-v4.1-flash` (no training, zero retention). Muse Spark trains
  on prompts, so it is listed in `models_that_train_on_prompts` and never assigned to a role
  — usable only for diagnostics. The skeptic therefore shares the verifier's model (Q-011).
- The client sends `User-Agent: OpenTatu/<version> (+repo URL)` and one stable
  `x-opencode-session` per conversation (`opentatu_llm::CallContext`).
- Budgets: OpenTatu may spend 60% of each model's monthly allowance ($60/month of DeepSeek);
  per-audit token/call limits are provisional (see `config/models.toml`).
- Accounting stores `usage.opencode.allowance` as `cost_usd` for the monthly share.

## Owner actions recommended

1. Keep the workspace privacy region on **Global** (required by DeepSeek) and re-check the
   DeepSeek zero-retention agreement after **2026-11-30** (renewed monthly).
2. Consider **disabling the paid-endpoints-that-train setting** if Muse Spark is not needed
   outside diagnostics; it allows prompts to be used for training.
3. Keep Console balance at zero (or disable Extra Usage) if you never want pay-as-you-go
   charges after the Go limits are reached.
4. Since the key was shared in a chat, rotate it if it ever needs to be more than a test key.

## Ambiguities and gaps

- Error code/body when a Go usage limit is reached: not documented, not observed (needs a
  real limit hit to confirm; the client maps 429 to `RateLimited`).
- Whether rate-limit headers exist: none observed.
- The ToS clause on programmatic extraction (item 7): interpreted as anti-scraping; the
  owner may want to confirm with OpenCode support.
