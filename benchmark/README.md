# Benchmark (phase 6)

Measures OpenTatu against intentionally vulnerable applications (e.g. OWASP Juice Shop,
WebGoat), run only in the isolated environment.

Planned layout:

| Path | Contents |
|---|---|
| `apps/` | references (repository URL + pinned commit) to the vulnerable apps |
| `expected/` | known flaws per app, in a format comparable by fingerprint/CWE/location |
| `results/` | scoreboards: found, missed, false positives — per prompt version and per model |

Done when: a scoreboard exists per prompt version and per model (see
[../docs/ROADMAP.md](../docs/ROADMAP.md)).
