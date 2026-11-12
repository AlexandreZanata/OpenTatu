//! AI roles of the internal investigator agent (spec §C.1–§C.6).
//!
//! Each role is a separate model call that always starts with the shared base prompt
//! (`prompts/base.md`). The prompt file of each role is `prompts/<role>.md`.

crate::text_enum! {
    /// An AI role of the investigator. Not to be confused with the auth roles of an
    /// audited app, which are plain strings inside the security map.
    pub enum AiRole {
        /// Builds and updates the security map; does not hunt vulnerabilities (§C.1).
        Cartographer => "cartographer",
        /// Generates vulnerability hypotheses and triages scanner results (§C.2).
        Hunter => "hunter",
        /// Proves or refutes each hypothesis; in doubt, "inconclusive" (§C.3).
        Verifier => "verifier",
        /// Tries to refute confirmed findings and computes final severity (§C.4).
        Skeptic => "skeptic",
        /// Turns upheld findings into executable fix tasks (§C.5).
        Planner => "planner",
        /// Writes the audit report from system-computed data (§C.6).
        Reporter => "reporter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_round_trip_as_lowercase_text() {
        assert_eq!(AiRole::ALL.len(), 6);
        for role in AiRole::ALL {
            assert_eq!(role.as_str().parse::<AiRole>().unwrap(), *role);
            assert_eq!(role.as_str(), role.as_str().to_lowercase());
        }
    }
}
