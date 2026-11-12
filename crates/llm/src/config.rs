//! Typed contents of `config/models.toml` (spec §D.2).

use std::collections::BTreeMap;
use std::path::Path;

use opentatu_core::{AiRole, AuditKind};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Default HTTP timeout: reasoning models can think for minutes on large inputs.
const DEFAULT_TIMEOUT_SECONDS: u64 = 300;

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
    /// Base URL confirmed in `docs/opencode-go.md` (HTTPS).
    pub base_url: String,
    /// Name of the environment variable holding the API key (never the key itself).
    pub api_key_env: String,
    /// HTTP timeout per call, in seconds.
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// The only models OpenTatu may call (empty = no restriction). The provider does not
    /// enforce key restrictions server-side, so OpenTatu does (docs/opencode-go.md).
    #[serde(default)]
    pub allowed_models: Vec<String>,
    /// Models whose provider trains on prompts/completions. Never assigned to AI roles
    /// (they receive application code); usable only for diagnostics such as `llm ping`.
    #[serde(default)]
    pub models_that_train_on_prompts: Vec<String>,
}

fn default_timeout_seconds() -> u64 {
    DEFAULT_TIMEOUT_SECONDS
}

impl ProviderConfig {
    /// Whether OpenTatu may call `model` at all.
    pub fn is_allowed(&self, model: &str) -> bool {
        self.allowed_models.is_empty() || self.allowed_models.iter().any(|m| m == model)
    }

    /// Whether the provider trains on prompts sent to `model`.
    pub fn trains_on_prompts(&self, model: &str) -> bool {
        self.models_that_train_on_prompts.iter().any(|m| m == model)
    }
}

/// Model settings of one AI role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    /// Exact model id from `docs/opencode-go.md`.
    pub model: String,
    /// Sampling temperature (0.0–2.0).
    pub temperature: f32,
    /// Owner consent to send this role's prompts to a model that trains on them
    /// (`[provider] models_that_train_on_prompts`). Default `false`: such models are refused.
    #[serde(default)]
    pub accept_prompt_training: bool,
}

impl RoleConfig {
    /// Whether this role may call `self.model` given the provider's training list.
    pub fn may_use_model(&self, provider: &ProviderConfig) -> bool {
        self.accept_prompt_training || !provider.trains_on_prompts(&self.model)
    }
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

/// Limits of a single audit. `0` means "not defined" (reported by `validate`).
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

    /// Distinct models assigned to AI roles, sorted.
    pub fn role_models(&self) -> Vec<&str> {
        let mut models: Vec<&str> = self.roles.values().map(|r| r.model.as_str()).collect();
        models.sort_unstable();
        models.dedup();
        models
    }

    /// Uses `model` for `role` (a benchmark variant, `opentatu benchmark run --role-model`).
    /// The role keeps its temperature and consent; call [`Self::validate`] afterwards — the
    /// allowlist and the training guard still apply.
    pub fn set_role_model(&mut self, role: AiRole, model: &str) -> Result<(), Error> {
        let config = self
            .roles
            .get_mut(&role)
            .ok_or(Error::RoleNotConfigured(role))?;
        config.model = model.trim().to_owned();
        Ok(())
    }

    /// Replaces some limits of one audit kind for one run (benchmark: a large app must not
    /// stop before the skeptic). `None` keeps the configured value.
    pub fn set_audit_budget(
        &mut self,
        kind: AuditKind,
        max_tokens: Option<u64>,
        max_minutes: Option<u32>,
    ) -> Result<(), Error> {
        let budget =
            self.budget.per_audit.get_mut(&kind).ok_or_else(|| {
                Error::InvalidConfig(format!("budget.per_audit.{kind} is missing"))
            })?;
        if let Some(tokens) = max_tokens {
            budget.max_tokens = tokens;
        }
        if let Some(minutes) = max_minutes {
            budget.max_minutes = minutes;
        }
        Ok(())
    }

    /// Problems that prevent real model calls. Empty means ready.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let provider = &self.provider;
        if !provider.base_url.starts_with("https://") {
            problems
                .push("provider.base_url must be an https:// URL (see docs/opencode-go.md)".into());
        }
        if provider.api_key_env.trim().is_empty() {
            problems.push("provider.api_key_env is empty".into());
        }
        if provider.timeout_seconds == 0 {
            problems.push("provider.timeout_seconds must be greater than 0".into());
        }
        for role in AiRole::ALL {
            let Some(config) = self.roles.get(role) else {
                problems.push(format!("roles.{role} is missing"));
                continue;
            };
            if config.model.trim().is_empty() {
                problems.push(format!("roles.{role}.model is empty"));
            } else if !provider.is_allowed(&config.model) {
                problems.push(format!(
                    "roles.{role}.model `{}` is not in allowed_models",
                    config.model
                ));
            } else if !config.may_use_model(provider) {
                problems.push(format!(
                    "roles.{role}.model `{}` trains on prompts; set accept_prompt_training = true \
                     only with the owner's consent (roles receive application data)",
                    config.model
                ));
            }
            if !(0.0..=2.0).contains(&config.temperature) {
                problems.push(format!("roles.{role}.temperature must be within 0.0..=2.0"));
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

    /// The committed config must always parse, cover every role and be ready for calls.
    const COMMITTED: &str = include_str!("../../../config/models.toml");

    fn committed() -> ModelsConfig {
        ModelsConfig::from_toml(COMMITTED).unwrap()
    }

    #[test]
    fn committed_config_parses_and_covers_every_role() {
        let config = committed();
        for role in AiRole::ALL {
            assert!(
                config.roles.contains_key(role),
                "config/models.toml lacks roles.{role}"
            );
        }
        assert_eq!(config.provider.api_key_env, "OPENCODE_GO_API_KEY");
    }

    #[test]
    fn role_overrides_keep_the_allowlist_and_the_training_guard() {
        let mut config = committed();
        config
            .set_role_model(AiRole::Skeptic, "deepseek-v4.1-flash")
            .unwrap();
        assert!(config.validate().is_empty());
        config
            .set_role_model(AiRole::Hunter, "muse-spark-1.3-contributor")
            .unwrap();
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("trains on prompts")),
            "no consent for the hunter"
        );
        config.set_role_model(AiRole::Hunter, "gpt-x").unwrap();
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("allowed_models"))
        );
    }

    #[test]
    fn budget_overrides_change_only_the_given_limits() {
        let mut config = committed();
        let before = config.budget.per_audit[&AuditKind::Full];
        config
            .set_audit_budget(AuditKind::Full, Some(10_000_000), None)
            .unwrap();
        let after = config.budget.per_audit[&AuditKind::Full];
        assert_eq!(after.max_tokens, 10_000_000);
        assert_eq!(
            (after.max_calls, after.max_minutes),
            (before.max_calls, before.max_minutes)
        );
        config
            .set_audit_budget(AuditKind::Full, Some(0), None)
            .unwrap();
        assert!(!config.validate().is_empty(), "0 is still refused");
    }

    #[test]
    fn committed_config_is_ready_for_calls() {
        let problems = committed().validate();
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn roles_cannot_use_models_outside_the_allowlist() {
        let mut config = committed();
        config.roles.get_mut(&AiRole::Hunter).unwrap().model = "glm-5.3".into();
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("not in allowed_models"))
        );
    }

    #[test]
    fn training_models_need_explicit_owner_consent() {
        let mut config = committed();
        let training = config.provider.models_that_train_on_prompts[0].clone();
        let hunter = config.roles.get_mut(&AiRole::Hunter).unwrap();
        hunter.model = training;
        hunter.accept_prompt_training = false;
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("trains on prompts"))
        );
        config
            .roles
            .get_mut(&AiRole::Hunter)
            .unwrap()
            .accept_prompt_training = true;
        assert!(config.validate().is_empty(), "{:?}", config.validate());
    }

    #[test]
    fn q011_roles_follow_the_owner_choice() {
        let config = committed();
        let training = &config.provider.models_that_train_on_prompts;
        for role in [
            AiRole::Cartographer,
            AiRole::Hunter,
            AiRole::Verifier,
            AiRole::Planner,
        ] {
            assert!(
                !training.contains(&config.roles[&role].model),
                "{role} must not train"
            );
        }
        assert_ne!(
            config.roles[&AiRole::Skeptic].model,
            config.roles[&AiRole::Verifier].model
        );
    }

    #[test]
    fn plain_http_is_rejected() {
        let mut config = committed();
        config.provider.base_url = "http://insecure.test/v1".into();
        assert!(config.validate().iter().any(|p| p.contains("https://")));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let text = COMMITTED.replace("[provider]", "[provider]\nunexpected = 1");
        assert!(ModelsConfig::from_toml(&text).is_err());
    }
}
