# prompts — AI role prompts (normative, versioned)

One file per prompt: `base.md` (shared, §C.0) and one per `AiRole` (`cartographer`,
`hunter`, `verifier`, `skeptic`, `planner`, `reporter`). They are embedded into the binary by
`opentatu-investigator` (`SOURCES` in `crates/investigator/src/prompts.rs`).

## Rules

- Front matter is mandatory: `name` (= file name), `version` (semver), `spec` (section).
- **Any change bumps `version`** ([../docs/recipes/change-prompt.md](../docs/recipes/change-prompt.md)).
- Instructions and JSON keys in English; ask for free-text values in pt-BR. The reporter's
  section headings are owner-facing output and stay pt-BR.
- Variables `{{name}}` only from `opentatu_investigator::prompts::VARIABLES`; values come
  from the system, never from the model (dates from the clock, R03).
- Input blocks use the tag names of `opentatu_investigator::Block`; the content of those
  blocks is untrusted (R11) and is inserted only through `fence()`.
- Keep the shared output fields: `role`, `audit_id`, `gaps`, `not_analyzed`,
  `suspicious_instructions`.
- These files are runtime content: they are excluded from the vocabulary check because of
  their pt-BR output headings. This `AGENTS.md` is not.
