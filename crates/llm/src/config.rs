//! Typed contents of `config/models.toml` (spec §D.2).

use std::collections::BTreeMap;
use std::path::Path;

use opentatu_core::{AiRole, AuditKind};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Root of `config/models.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelsConfig {
    /// Provider connection settings.
    pub provider: ProviderConfig,
    /// Model and temperature per AI role.
    pub roles: BTreeMap<AiRole, RoleConfig>,
    /// Usage limits.
    pub budget: BudgetConfig,
}

/// Provider connection settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// Provider name, e.g. `opencode-go`.
    pub name: String,
    /// Base URL confirmed in `docs/opencode-go.md` (empty until phase 0 research).
    pub base_url: String,
    /// Name of the environment variable holding the API key (never the key itself).
    pub api_key_env: String,
}

/// Model settings of one AI role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    /// Exact model id (empty until phase 0 research).
    pub model: String,
    /// Sampling temperature (0.0–2.0).
    pub temperature: f32,
}

/// Usage limits (spec §A.2, §D.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Maximum share of the plan's limit OpenTatu may use (spec: 0.60).
    pub max_plan_share: f32,
    /// Per-audit limits by audit kind.
    pub per_audit: BTreeMap<AuditKind, AuditBudget>,
}

/// Limits of a single audit. `0` means "not defined yet" (reported by `validate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditBudget {
    /// Total tokens (input + output).
    pub max_tokens: u64,
    /// Model calls.
    pub max_calls: u32,
    /// Wall-clock minutes.
    pub max_minutes: u32,
}

impl ModelsConfig {
    /// Parses the TOML text of `config/models.toml`.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        toml::from_str(text).map_err(|error| Error::InvalidConfig(error.to_string()))
    }

    /// Reads and parses a models config file.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::ReadConfig {
            path: path.to_owned(),
            source,
        })?;
        Self::from_toml(&text)
    }

    /// Problems that prevent real model calls (missing roles, placeholders not yet filled
    /// by the phase 0 research, out-of-range values). Empty means ready.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.provider.base_url.trim().is_empty() {
            problems.push("provider.base_url is empty (fill it from docs/opencode-go.md)".into());
        }
        if self.provider.api_key_env.trim().is_empty() {
            problems.push("provider.api_key_env is empty".into());
        }
        for role in AiRole::ALL {
            match self.roles.get(role) {
                None => problems.push(format!("roles.{role} is missing")),
                Some(config) if config.model.trim().is_empty() => {
                    problems.push(format!("roles.{role}.model is empty"));
                }
                Some(config) if !(0.0..=2.0).contains(&config.temperature) => {
                    problems.push(format!("roles.{role}.temperature must be within 0.0..=2.0"));
                }
                Some(_) => {}
            }
        }
        if !(self.budget.max_plan_share > 0.0 && self.budget.max_plan_share <= 1.0) {
            problems.push("budget.max_plan_share must be within (0, 1]".into());
        }
        for kind in [
            AuditKind::Full,
            AuditKind::Incremental,
            AuditKind::Reverification,
        ] {
            match self.budget.per_audit.get(&kind) {
                None => problems.push(format!("budget.per_audit.{kind} is missing")),
                Some(limits) if limits.max_tokens == 0 || limits.max_calls == 0 => {
                    problems.push(format!("budget.per_audit.{kind} has undefined (0) limits"));
                }
                Some(_) => {}
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed config must always parse and cover every role, even before the
    /// phase 0 research fills the placeholders.
    const COMMITTED: &str = include_str!("../../../config/models.toml");

    #[test]
    fn committed_config_parses_and_covers_every_role() {
        let config = ModelsConfig::from_toml(COMMITTED).unwrap();
        for role in AiRole::ALL {
            assert!(
                config.roles.contains_key(role),
                "config/models.toml lacks roles.{role}"
            );
        }
        assert_eq!(config.provider.api_key_env, "OPENCODE_GO_API_KEY");
    }

    #[test]
    fn validate_reports_placeholders() {
        let config = ModelsConfig::from_toml(COMMITTED).unwrap();
        let mut ready = config.clone();
        ready.provider.base_url = "https://provider.test/v1".into();
        for role in ready.roles.values_mut() {
            role.model = "model-x".into();
        }
        for limits in ready.budget.per_audit.values_mut() {
            limits.max_tokens = 1_000;
            limits.max_calls = 10;
        }
        assert!(ready.validate().is_empty(), "{:?}", ready.validate());
        if config.provider.base_url.is_empty() {
            assert!(!config.validate().is_empty());
        }
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let text = COMMITTED.replace("[provider]", "[provider]\nunexpected = 1");
        assert!(ModelsConfig::from_toml(&text).is_err());
    }
}
