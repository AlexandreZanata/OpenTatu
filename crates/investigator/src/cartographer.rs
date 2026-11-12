//! The cartographer (spec §C.1): output type and inputs. The system validates the output
//! against the commit and the ASVS catalog (`MapContent::sanitize`) before storing it.

use std::fmt::Write as _;

use opentatu_core::{AiRole, AsvsCatalog, AsvsLevel, MapContent};
use serde::Deserialize;

use crate::{Block, CommonFields, RoleOutput};

/// The cartographer's answer (`prompts/cartographer.md` 2.x).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CartographerOutput {
    /// Shared keys.
    #[serde(flatten)]
    pub common: CommonFields,
    /// The map, before validation.
    #[serde(flatten)]
    pub map: MapContent,
}

impl RoleOutput for CartographerOutput {
    const ROLE: AiRole = AiRole::Cartographer;

    fn common(&self) -> &CommonFields {
        &self.common
    }
}

/// What the cartographer receives (each entry becomes one fenced block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartographerInputs {
    /// `<structure>`: file tree.
    pub structure: String,
    /// `<code>`: selected, line-numbered files.
    pub code: String,
    /// `<config>`: configuration files.
    pub config: String,
    /// `<previous_map>`: the last map as JSON, if any.
    pub previous_map: Option<String>,
    /// `<diff>`: changes since the last audit (INCREMENTAL, phase 7).
    pub diff: Option<String>,
}

impl CartographerInputs {
    /// The fenced blocks, in prompt order, with the ASVS requirements up to `level`.
    pub fn blocks(self, catalog: &AsvsCatalog, level: AsvsLevel) -> Vec<(Block, String)> {
        let mut blocks = vec![
            (Block::Structure, self.structure),
            (Block::Code, self.code),
            (Block::Config, self.config),
            (Block::Asvs, asvs_listing(catalog, level)),
        ];
        if let Some(previous) = self.previous_map {
            blocks.push((Block::PreviousMap, previous));
        }
        if let Some(diff) = self.diff {
            blocks.push((Block::Diff, diff));
        }
        blocks
    }
}

/// One line per requirement up to `level`, grouped by chapter:
/// `V8.2.1 [L1] Verify that ...`.
pub fn asvs_listing(catalog: &AsvsCatalog, level: AsvsLevel) -> String {
    let mut out = String::new();
    let mut chapter = "";
    for requirement in catalog.up_to(level) {
        if requirement.chapter_id != chapter {
            chapter = &requirement.chapter_id;
            let _ = writeln!(
                out,
                "# {} {}",
                requirement.chapter_id, requirement.chapter_name
            );
        }
        let _ = writeln!(
            out,
            "{} [L{}] {}",
            requirement.id, requirement.level, requirement.description
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use opentatu_core::{AuditId, AuthenticationRequirement};
    use opentatu_llm::FakeClient;

    use super::*;
    use crate::run::tests::{llm, variables};
    use crate::{RoleCall, run_role};

    const ANSWER: &str = r#"{
      "role": "cartographer", "audit_id": "AUD-DEMO-20261112-0900",
      "components": [ { "name": "Servidor", "kind": "api", "files": ["src/main.rs"] } ],
      "entry_points": [ { "id": "EP-001", "method": "WS", "path": "match.join",
          "file": "src/ws.rs", "line": 40, "symbol": "on_join",
          "authentication": "unknown", "allowed_roles": [], "inputs": ["message"],
          "authorization_check": "nenhuma visível", "sensitive_data": [],
          "applicable_asvs": ["V4.4.1"] } ],
      "trust_boundaries": [], "sensitive_data": [],
      "authentication": { "mechanism": "desconhecido", "storage": "desconhecido",
          "expiry": "desconhecido", "renewal": "desconhecido", "logout": "desconhecido",
          "files": [] },
      "authorization": { "mechanism": "desconhecido", "checked_at": [],
          "entry_points_without_check": ["EP-001"] },
      "integrations": [],
      "configuration": { "cors": "desconhecido", "cookies": "desconhecido",
          "headers": "desconhecido", "debug_mode": "desconhecido",
          "error_handling": "desconhecido" },
      "infrastructure": { "base_image": "desconhecido", "container_user": "desconhecido",
          "ports": ["8080"], "environment_variables": ["DATABASE_URL"] },
      "critical_dependencies": [], "app_asvs": ["V13.1.1"],
      "changes_since_previous_map": [], "open_questions": [],
      "gaps": ["Sem o código do matchmaking."], "not_analyzed": [],
      "suspicious_instructions": []
    }"#;

    #[test]
    fn asvs_listing_covers_the_level_with_chapter_headers() {
        let catalog = AsvsCatalog::embedded().unwrap();
        let listing = asvs_listing(catalog, AsvsLevel::L1);
        assert!(listing.starts_with("# V1 Encoding and Sanitization\n"));
        assert_eq!(listing.lines().filter(|l| l.starts_with('V')).count(), 70);
        assert!(
            listing
                .lines()
                .filter(|l| l.starts_with('V'))
                .all(|l| l.contains(" [L1] "))
        );
    }

    #[tokio::test]
    async fn the_cartographer_answer_parses_into_a_map() {
        let fake = Arc::new(FakeClient::new());
        fake.push_answer(ANSWER);
        let audit: AuditId = "AUD-DEMO-20261112-0900".parse().unwrap();
        let catalog = AsvsCatalog::embedded().unwrap();
        let inputs = CartographerInputs {
            structure: "src/main.rs (1 KB)".into(),
            code: "=== src/main.rs ===".into(),
            config: String::new(),
            previous_map: None,
            diff: None,
        };
        let variables = variables();
        let call = RoleCall {
            audit_id: &audit,
            variables: &variables,
            inputs: inputs.blocks(catalog, AsvsLevel::L3),
            conversation: 1,
        };

        let answer = run_role::<CartographerOutput>(&llm(&fake), call)
            .await
            .unwrap();

        let map = &answer.output.map;
        assert_eq!(map.entry_points[0].method, "WS");
        assert_eq!(map.entry_points[0].line, Some(40));
        assert_eq!(
            map.entry_points[0].authentication,
            AuthenticationRequirement::Unknown
        );
        assert_eq!(answer.output.common.gaps.len(), 1);
        assert_eq!(answer.prompt_versions[1], "cartographer@2.1.0");
        let user = &fake.requests()[0].messages[1].content;
        for tag in ["<structure>", "<code>", "<config>", "<asvs>"] {
            assert!(user.contains(tag), "missing {tag}");
        }
        assert!(!user.contains("<previous_map>"), "no previous map yet");
    }
}
