# Recipe: change a prompt

Prompts in `prompts/` are normative (spec Part C) and versioned (R17).

1. Edit `prompts/<name>.md`. Keep English instructions, English JSON keys, and the request
   for pt-BR free-text values. Keep the input tag names of `opentatu_investigator::Block`.
2. Bump `version` in the front matter (semver):
   - PATCH: wording that does not change behavior;
   - MINOR: new optional output key or new guidance;
   - MAJOR: removed/renamed output keys or changed semantics (the output types and
     validation must change in the same commit).
3. New `{{variable}}`? Register it with a description in
   `opentatu_investigator::prompts::VARIABLES` and make the orchestrator provide it.
   Values always come from the system (never from the model; dates from the `Clock`).
4. New prompt file? Register it in `SOURCES` (`crates/investigator/src/prompts.rs`); the
   test `every_prompt_file_is_registered` fails otherwise.
5. If the change affects a rule (severity matrix, evidence standard), update the matching
   section of [../spec/C-investigator.md](../spec/C-investigator.md).
6. From phase 6 on, run the benchmark and record the scoreboard for the new version.
7. `cargo xtask check`.
