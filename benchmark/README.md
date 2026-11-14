# Benchmark

How well OpenTatu finds known flaws (roadmap phase 6, ADR-0031). The scoreboard is
[SCOREBOARD.md](SCOREBOARD.md); each run is a file in `results/`.

| Path | Contents |
|---|---|
| `apps/<name>.toml` | One intentionally vulnerable app: repository, pinned commit, marker syntax, files left out and why, match tolerance |
| `expected/<name>.json` | Its known flaws, derived by `opentatu benchmark prepare` from the app's own markers — file paths and line numbers only |
| `results/<AUDIT>.json` | One run: models and prompt versions used, overrides, usage, what the verifier and the skeptic decided, and the score with every match |
| `SCOREBOARD.md` | Generated from `results/`: per run, per prompt version and model, per category |

## Running

```bash
cargo run -p opentatu -- scanners update            # once: offline scanner data
cargo run -p opentatu -- benchmark prepare juice-shop
cargo run -p opentatu -- benchmark run juice-shop   # real model calls (~30 min)
cargo run -p opentatu -- benchmark run juice-shop --max-tokens 10000000 --max-minutes 180
cargo run -p opentatu -- benchmark run juice-shop --role-model skeptic=deepseek-v4.1-flash
cargo run -p opentatu -- --data-dir <run data> benchmark rescore <AUDIT>  # after a scoring change
```

## Rules

- **Isolated only:** the app is cloned at a pinned commit (a moved tag is refused) and
  audited; it is never built, installed or run. Scanners run offline in their containers.
- **Ground truth from the app itself:** known flaws come only from the app's markers and its
  challenge catalog, never from a person or a model. Markers that point at the same lines
  are one flaw.
- **No answers in the audited copy:** marker comments are removed (lines kept, so line
  numbers match) and files that hold solutions are left out (listed in the app definition).
  Two biases remain and favor the tool: public apps may be known to the models from
  training, and Juice Shop's own runtime code names the challenge it detects next to many
  vulnerable lines (`challengeUtils.solveIf(challenges.…)`) — that is application logic,
  so it stays. Compare runs with each other, not with an absolute target.
- **Each run is a fresh app** (`bench-<name>-<n>`): no memory of earlier runs (R08, R09).
- **Scoring:** a reported finding (confirmed, not dismissed) on the same file within
  `match_lines` lines of a marked line finds that flaw. Reported findings outside the list
  are the false-positive candidates; Juice Shop marks only its coding challenges (33 flaws
  of about 118 challenges), so many of them are real flaws — review before counting them as
  false positives. Flaws matched only by unconfirmed or dismissed hypotheses are counted
  apart.
- **Budget:** a large app can exhaust the FULL budget before the skeptic runs (run 1:
  3M tokens during verification). `--max-tokens` / `--max-minutes` raise it for one run and
  are recorded in the result.
- After any prompt or model change, run the benchmark and commit the new result
  ([../docs/recipes/change-prompt.md](../docs/recipes/change-prompt.md)).

## Shared cost-capped campaigns

ADR-0044 adds optional `--campaign-budget <ledger.jsonl> --max-cost-usd <USD>` to
`benchmark run` and `worker`/`serve`. Both flags are required together. The ledger's
existing parent must be private (0700), and the ledger itself is private (0600).
Always reuse the **same absolute ledger path** for every run in a combined budget.
Its cap, provider and researched pricing cannot change on reopen. A larger configured
output ceiling is recorded as an append-only transition before dispatch; older charges
and unknown-attempt reservations remain intact. Decreasing the ceiling is refused.

For the owner-authorized 2026-11-14 campaign, every role uses the configured DeepSeek
model, including per-run `--role-model skeptic=deepseek-v4.1-flash` and
`--role-model reporter=deepseek-v4.1-flash`; normal default role assignments stay intact.
Use `--max-cost-usd 20` for the combined campaign. The gate reserves the maximum
documented input/output cost before each HTTP attempt, including retries. Successful
known charges release the unused reservation; failed, interrupted or malformed
attempts retain it. Exhaustion can therefore stop earlier than actual billed US$20.
Do not delete/reset the ledger or create another to continue the same campaign.

The JSONL ledger is private billing state; publish only reviewed aggregate cost and
attempt counts. Real-source Abastevo acceptance lives in
[`acceptance/abastevo-20261114/`](acceptance/abastevo-20261114/REPORT.md) and has no
invented known-flaw score. Its static review and proposed tests retain their own
provenance, separate from provider-funded engine results.

The owner limits detailed publication to Abastevo. New Juice Shop raw run results
therefore stay in the private campaign data directory; reviewed public projections in
[`campaigns/`](campaigns/README.md) retain counts, known-marker scores, model/prompt
versions, limits, timing and cost. They expose only the count of findings outside the
known catalog, never their locations or CWEs. These are labeled aggregate projections,
not substituted raw `RunResult` files. Historical committed results are unchanged.
