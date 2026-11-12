# Recipe: finish a roadmap phase

1. Verify the phase's "done when" criterion in [../ROADMAP.md](../ROADMAP.md) by actually
   running it (commands, dashboard, outputs). Do not mark it done from reasoning alone.
2. `cargo xtask check` passes on a clean checkout.
3. Update docs: ROADMAP status (✅) and "what exists" notes, [../COMPONENTS.md](../COMPONENTS.md),
   crate `AGENTS.md` files, ADRs for decisions taken, [../OPEN-QUESTIONS.md](../OPEN-QUESTIONS.md).
4. Stop and report to the owner, in this structure:

   ```markdown
   ## Phase N — <name>: report
   **Done:** what was built (bullets, with crates/files).
   **How to test:** exact commands and what you should see.
   **Done criterion:** the roadmap criterion and the evidence it was met.
   **Decisions:** ADR ids (one line each).
   **Open questions:** ids that need the owner's answer.
   **Next phase:** what it will do and what it needs from the owner.
   ```

5. Wait for the owner's approval before starting the next phase.
