//! Registered applications (spec §A.4; R01: only registered apps owned by the owner are
//! audited). Serialized as `app.json` (spec §B.4).

use serde::{Deserialize, Serialize};

use crate::{Error, Slug, Timestamp};

/// Registration input. Validate with [`NewApp::validate`] before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewApp {
    /// Unique slug.
    pub slug: Slug,
    /// Display name.
    pub name: String,
    /// Git repository: URL (`https://…`, `ssh://…`, `git@host:…`) or local path.
    pub repo_url: String,
    /// Main branch audited by default.
    pub branch: String,
    /// Technologies, e.g. `["Next.js", "Postgres"]`.
    pub stack: Vec<String>,
    /// Sensitive data kinds handled by the app (raise severity by one level, §C.8).
    pub sensitive_data: Vec<String>,
    /// Isolated lab URL for dynamic tests (R10), if any.
    pub lab_url: Option<String>,
}

impl NewApp {
    /// Rejects empty fields and values that could be read as command-line options or
    /// unsafe git transports when passed to `git` (R12 hardening).
    pub fn validate(&self) -> Result<(), Error> {
        let invalid = |message: String| Err(Error::InvalidApp(message));
        if self.name.trim().is_empty() {
            return invalid("name is empty".into());
        }
        let repo = self.repo_url.trim();
        if repo.is_empty() || repo.starts_with('-') || repo.contains("ext::") || repo.contains('\n')
        {
            return invalid(format!(
                "repo_url {:?} must be a git URL or path (not empty, no leading '-', no ext:: transport)",
                self.repo_url
            ));
        }
        if !is_valid_branch(&self.branch) {
            return invalid(format!(
                "branch {:?} is not a valid branch name (no spaces, '..', leading '-' or control characters)",
                self.branch
            ));
        }
        if let Some(lab) = &self.lab_url
            && !(lab.starts_with("http://") || lab.starts_with("https://"))
        {
            return invalid(format!(
                "lab_url {lab:?} must start with http:// or https://"
            ));
        }
        Ok(())
    }
}

/// Conservative subset of git's branch-name rules.
fn is_valid_branch(branch: &str) -> bool {
    !branch.is_empty()
        && !branch.starts_with('-')
        && !branch.starts_with('/')
        && !branch.ends_with('/')
        && !branch.ends_with(".lock")
        && !branch.contains("..")
        && !branch.contains("@{")
        && branch
            .chars()
            .all(|c| !c.is_control() && !" ~^:?*[\\".contains(c))
}

/// A registered app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    /// Profile fields as registered.
    #[serde(flatten)]
    pub profile: NewApp,
    /// Registration time (from the clock, R03).
    pub created_at: Timestamp,
}

impl App {
    /// The app's slug.
    pub fn slug(&self) -> &Slug {
        &self.profile.slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_app() -> NewApp {
        NewApp {
            slug: Slug::new("abastevo").unwrap(),
            name: "Abastevo".into(),
            repo_url: "git@github.com:owner/abastevo.git".into(),
            branch: "main".into(),
            stack: vec!["Next.js".into()],
            sensitive_data: vec!["dados pessoais".into()],
            lab_url: None,
        }
    }

    #[test]
    fn valid_profile_passes() {
        assert!(new_app().validate().is_ok());
        let mut local = new_app();
        local.repo_url = "/home/owner/code/abastevo".into();
        local.branch = "release/2026-11".into();
        assert!(local.validate().is_ok());
    }

    #[test]
    fn r12_rejects_values_that_could_inject_git_options() {
        for repo in ["", "--upload-pack=touch /tmp/x", "ext::sh -c id"] {
            let mut app = new_app();
            app.repo_url = repo.into();
            assert!(app.validate().is_err(), "{repo:?}");
        }
        for branch in ["", "-x", "a b", "a..b", "x.lock", "refs/@{1}"] {
            let mut app = new_app();
            app.branch = branch.into();
            assert!(app.validate().is_err(), "{branch:?}");
        }
    }

    #[test]
    fn app_json_is_flat() {
        let app = App {
            profile: new_app(),
            created_at: "2026-11-12T09:00:00-03:00".parse().unwrap(),
        };
        let json = serde_json::to_value(&app).unwrap();
        assert_eq!(json["slug"], "abastevo");
        assert_eq!(json["created_at"], "2026-11-12T09:00:00-03:00");
        assert_eq!(serde_json::from_value::<App>(json).unwrap(), app);
    }
}
