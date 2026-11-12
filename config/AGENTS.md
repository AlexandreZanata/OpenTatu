# config — runtime configuration and static assets

| Path | Contents | Status |
|---|---|---|
| `models.toml` | provider, model per AI role, budgets (§D.2) — parsed by `opentatu_llm::ModelsConfig` | placeholders until phase 0 |
| `asvs/` | OWASP ASVS requirements as JSON | phase 3 |
| `semgrep/` | Semgrep rules downloaded ahead of time (used offline) | phase 2 |

## Rules

- Never put secrets here: `api_key_env` holds the **name** of the variable.
- Model ids, URLs and limits only from [../docs/opencode-go.md](../docs/opencode-go.md); keep
  the `PENDING(phase-0)` markers until then.
- TOML only (ADR-0008). Unknown keys are rejected; a test keeps `models.toml` valid.
- Third-party assets (ASVS, rules) keep their upstream license and a note of version and
  download date.
