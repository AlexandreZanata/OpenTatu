# config — runtime configuration and static assets

| Path | Contents | Status |
|---|---|---|
| `models.toml` | provider, model per AI role, budgets (§D.2) — parsed by `opentatu_llm::ModelsConfig` | ✅ phase 0 |
| `scanners.toml` | container engine, timeout, image pinned by digest + version per scanner, Semgrep ruleset, OSV ecosystems — parsed by `opentatu_scanners::ScannersConfig` (ADR-0023) | ✅ phase 2 |
| `asvs/` | OWASP ASVS 5.0.0 requirements (official flat JSON, CC BY-SA 4.0), embedded by `opentatu_core::AsvsCatalog` (ADR-0024) | ✅ phase 3 |

Scanner rules and vulnerability databases are **not** stored here: `opentatu scanners update`
downloads them into `<data>/scanner-cache/<tool>/` on the owner's machine (third-party
licenses, ~2 GB, refreshed often).

## Rules

- Never put secrets here: `api_key_env` holds the **name** of the variable.
- Model ids, URLs and limits only from [../docs/opencode-go.md](../docs/opencode-go.md); keep
  the `PENDING(phase-0)` markers until then.
- TOML only (ADR-0008). Unknown keys are rejected; tests keep `models.toml` and
  `scanners.toml` valid.
- Scanner images change only together with their digest and version (R17).
- Third-party assets (ASVS, rules) keep their upstream license and a note of version and
  download date.
