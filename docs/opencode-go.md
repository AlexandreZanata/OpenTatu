# OpenCode Go — research record (spec §D.1)

> **Status: NOT STARTED.** This is the first task of phase 0. Until every item below is
> confirmed from the official documentation, `config/models.toml` keeps its placeholders
> and `opentatu llm ping` refuses to run. Never fill these from memory.

Consultation date and time: _not yet_ (record with offset, e.g. `2026-11-12T10:00:00-03:00`)

Consulted by: _agent/model name_

## Pages read

| URL | Read at | Notes |
|---|---|---|
| _pending_ | | |

## Findings

| # | Item (spec §D.1) | Result | Source |
|---|---|---|---|
| 1 | API base URL; OpenAI-compatible? (chat completions or other) | not confirmed | |
| 2 | Authentication format (header, token prefix) | not confirmed | |
| 3 | Models in the plan; exact id of DeepSeek Flash; other models for the skeptic | not confirmed | |
| 4 | Context window and output limit per model used | not confirmed | |
| 5 | Structured JSON output, tool calling, streaming | not confirmed | |
| 6 | Plan usage limits per period; error codes on limit (e.g. 429); rate-limit headers | not confirmed | |
| 7 | **Terms of use: is API use for personal automation allowed?** If not: STOP and tell the owner | not confirmed | |
| 8 | Data retention and data-use policy (application code will be sent) | not confirmed | |

## Ambiguities and gaps

- _List anything unclear or missing, marked "not confirmed", and copy blocking items to
  [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md)._

## Resulting configuration

Once confirmed, fill `config/models.toml` (base URL, model ids, budgets) and reference the
rows above in the commit message.
