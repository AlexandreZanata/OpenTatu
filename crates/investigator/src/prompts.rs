//! Prompt catalog: the files of `prompts/`, embedded at compile time.
//!
//! Each file starts with a front matter block:
//!
//! ```text
//! ---
//! name: verifier
//! version: 1.0.0
//! spec: C.3
//! ---
//! ```
//!
//! `name` must equal the file stem (the [`AiRole`] text for role prompts) and `version` is
//! `MAJOR.MINOR.PATCH`. Any change to a prompt bumps its version (docs/recipes/change-prompt.md);
//! the `name@version` tag is recorded with every AI-produced event and report (R17).

use opentatu_core::AiRole;

use crate::Error;

/// Every embedded prompt: `(file name, contents)`. The ONLY list of prompt files; a test
/// fails if `prompts/` holds a prompt that is not registered here.
const SOURCES: &[(&str, &str)] = &[
    ("base.md", include_str!("../../../prompts/base.md")),
    (
        "cartographer.md",
        include_str!("../../../prompts/cartographer.md"),
    ),
    ("hunter.md", include_str!("../../../prompts/hunter.md")),
    ("verifier.md", include_str!("../../../prompts/verifier.md")),
    ("skeptic.md", include_str!("../../../prompts/skeptic.md")),
    ("planner.md", include_str!("../../../prompts/planner.md")),
    ("reporter.md", include_str!("../../../prompts/reporter.md")),
];

/// Every `{{variable}}` a prompt may use, with its meaning. Values are always produced by
/// the system (R03: dates come from the clock; the AI never invents them).
pub const VARIABLES: &[(&str, &str)] = &[
    (
        "owner",
        "Owner's name, from the OPENTATU_OWNER environment variable (Q-009)",
    ),
    ("role", "AI role text, e.g. `verifier`"),
    ("audit_id", "Audit ID, e.g. AUD-ABASTEVO-20261129-1432"),
    ("audit_kind", "FULL, INCREMENTAL, REVERIFICATION or PR"),
    ("app_name", "App display name"),
    ("app_slug", "App slug"),
    ("commit", "Full commit hash under analysis (R02)"),
    ("started_display", "Audit start, owner-facing pt-BR display"),
    ("started_iso", "Audit start, ISO 8601 with offset"),
    (
        "previous_audit_id",
        "Previous completed audit ID, or `none`",
    ),
    (
        "previous_display",
        "Previous audit start, pt-BR display, or `none`",
    ),
    (
        "remaining_budget",
        "Remaining budget summary (tokens, calls, minutes, percent)",
    ),
    (
        "memory",
        "Fenced <memory> block with the generated MEMORY.md",
    ),
    (
        "lab_available",
        "`yes` or `no`: whether the isolated dynamic lab exists",
    ),
    (
        "lab_url",
        "Isolated lab URL, or `none` (production is always out of scope)",
    ),
    ("finished_display", "Audit end, pt-BR display"),
    ("finished_iso", "Audit end, ISO 8601 with offset"),
    ("duration", "Audit duration, human readable"),
    ("models", "Model id per role"),
    ("prompt_versions", "name@version of every prompt used"),
    ("scanners", "Scanner versions and image digests"),
    ("usage", "Tokens and calls consumed"),
];

/// A versioned prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// Name from the front matter (file stem).
    pub name: String,
    /// Semantic version from the front matter.
    pub version: String,
    /// Spec section the prompt implements, e.g. `C.3`.
    pub spec: String,
    /// Text after the front matter.
    pub body: &'static str,
}

impl Prompt {
    /// `name@version`, recorded with AI-produced events and reports (R17).
    pub fn version_tag(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

/// Shared base prompt (spec §C.0), prepended to every role prompt.
pub fn base() -> Result<Prompt, Error> {
    named("base")
}

/// Prompt of `role` (spec §C.1–§C.6).
pub fn for_role(role: AiRole) -> Result<Prompt, Error> {
    named(role.as_str())
}

/// System prompt template of `role`: base + role body, plus their version tags.
pub fn system_template(role: AiRole) -> Result<(String, Vec<String>), Error> {
    let base = base()?;
    let role_prompt = for_role(role)?;
    let template = format!(
        "{}\n\n{}",
        base.body.trim_end(),
        role_prompt.body.trim_end()
    );
    Ok((
        template,
        vec![base.version_tag(), role_prompt.version_tag()],
    ))
}

/// Every embedded prompt, parsed.
pub fn all() -> Result<Vec<Prompt>, Error> {
    SOURCES
        .iter()
        .map(|(file, source)| parse(file, source))
        .collect()
}

fn named(name: &str) -> Result<Prompt, Error> {
    let file = format!("{name}.md");
    let (file, source) = SOURCES
        .iter()
        .find(|(candidate, _)| *candidate == file)
        .ok_or_else(|| Error::MissingPrompt(name.to_owned()))?;
    parse(file, source)
}

fn parse(file: &'static str, source: &'static str) -> Result<Prompt, Error> {
    let malformed = |problem: &str| Error::MalformedPrompt {
        file,
        problem: problem.to_owned(),
    };
    let rest = source
        .strip_prefix("---\n")
        .ok_or_else(|| malformed("must start with `---`"))?;
    let (header, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| malformed("front matter is not closed with `---`"))?;

    let field = |key: &str| {
        header
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, value)| value.trim().to_owned())
            .ok_or_else(|| malformed(&format!("missing `{key}` in front matter")))
    };
    let name = field("name")?;
    let version = field("version")?;
    let spec = field("spec")?;

    if file.strip_suffix(".md") != Some(name.as_str()) {
        return Err(malformed("`name` must equal the file name without `.md`"));
    }
    let semver = version.split('.').collect::<Vec<_>>();
    if semver.len() != 3
        || !semver
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(malformed("`version` must be MAJOR.MINOR.PATCH"));
    }
    Ok(Prompt {
        name,
        version,
        spec,
        body: body.trim_start_matches('\n'),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::render::placeholders;

    #[test]
    fn every_prompt_has_valid_front_matter() {
        let prompts = all().unwrap();
        assert_eq!(prompts.len(), SOURCES.len());
        assert!(prompts.iter().all(|p| !p.body.trim().is_empty()));
    }

    #[test]
    fn every_role_has_a_prompt() {
        for role in AiRole::ALL {
            let prompt = for_role(*role).unwrap();
            assert_eq!(prompt.name, role.as_str());
        }
    }

    #[test]
    fn prompts_only_use_known_variables() {
        let known: BTreeSet<&str> = VARIABLES.iter().map(|(name, _)| *name).collect();
        for (file, source) in SOURCES {
            for name in placeholders(source) {
                assert!(
                    known.contains(name),
                    "prompts/{file} uses unknown variable {{{{{name}}}}}"
                );
            }
        }
    }

    #[test]
    fn every_prompt_file_is_registered() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
        let registered: BTreeSet<&str> = SOURCES.iter().map(|(file, _)| *file).collect();
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let is_doc = matches!(name.as_str(), "AGENTS.md" | "CLAUDE.md" | "README.md");
            if name.ends_with(".md") && !is_doc {
                assert!(
                    registered.contains(name.as_str()),
                    "prompts/{name} is not in SOURCES"
                );
            }
        }
    }

    #[test]
    fn system_template_joins_base_and_role() {
        let (template, tags) = system_template(AiRole::Verifier).unwrap();
        assert!(template.contains("{{audit_id}}"));
        assert_eq!(tags, ["base@1.1.0", "verifier@1.0.0"]);
    }
}
