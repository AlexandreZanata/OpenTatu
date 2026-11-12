//! Placeholder for commands planned in later roadmap phases.

/// Fails with a pointer to the roadmap phase that implements `command`.
pub fn not_implemented(command: &str, phase: u8) -> anyhow::Result<()> {
    anyhow::bail!("`{command}` is not implemented yet (roadmap phase {phase}, see docs/ROADMAP.md)")
}
