//! Commit changes and approved-fix patch identity (phase 7, ADR-0032/0033, R02).

use serde::{Deserialize, Serialize};

use crate::CommitHash;

/// One changed path in a commit diff; a rename carries its previous path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    /// Path at the new commit (old path for a deletion).
    pub path: String,
    /// Old path for a rename.
    pub previous_path: Option<String>,
    /// Git status: A, M, D, R, T, etc.; rename scores are retained.
    pub status: String,
}

/// Patch identity recorded before an approved fix branch can disappear (ADR-0032).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixIdentity {
    /// Merge base of the configured branch and the approved commit.
    pub base: CommitHash,
    /// Configured branch head when approval was recorded.
    pub branch_head: CommitHash,
    /// Stable nonempty patch IDs of each nonempty fix commit (first-parent for merges).
    pub commits: Vec<String>,
    /// Stable patch ID of the entire fix diff (squash detection).
    pub whole: Option<String>,
}

/// Explicit scope recorded in an incremental report; unchanged work is not new coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncrementalScope {
    /// Exact base commit of the previous final non-reverification audit.
    pub base_commit: CommitHash,
    /// Changed paths, including deletions and both sides of renames.
    pub changed_files: Vec<String>,
    /// Entry-point IDs actually examined in hunter rounds of this audit.
    pub examined_entry_points: Vec<String>,
    /// Map entry points not examined; they are not claimed safe by this audit.
    pub out_of_scope: Vec<String>,
}

impl FixIdentity {
    /// How the approved patch reached a rewritten branch, if it did. Empty fixes never
    /// match vacuously. A squash matches one whole-diff patch; a rebase matches all commits.
    pub fn matched_by(&self, branch_patches: &[String]) -> Option<&'static str> {
        let mut available = std::collections::BTreeMap::new();
        for patch in branch_patches {
            *available.entry(patch.as_str()).or_insert(0usize) += 1;
        }
        let matched = !self.commits.is_empty()
            && self.commits.iter().all(|patch| {
                let Some(count) = available.get_mut(patch.as_str()) else {
                    return false;
                };
                if *count == 0 {
                    return false;
                }
                *count -= 1;
                true
            });
        if matched {
            Some("patch_id_rebase")
        } else if self
            .whole
            .as_ref()
            .is_some_and(|p| branch_patches.contains(p))
        {
            Some("patch_id_squash")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_matching_requires_the_whole_squash_or_every_fix_commit() {
        let hash = "1111111111111111111111111111111111111111".parse().unwrap();
        let mut fix = FixIdentity {
            base: hash,
            branch_head: "2222222222222222222222222222222222222222".parse().unwrap(),
            commits: vec!["a".into(), "b".into()],
            whole: Some("whole".into()),
        };
        assert_eq!(fix.matched_by(&["whole".into()]), Some("patch_id_squash"));
        assert_eq!(
            fix.matched_by(&["b".into(), "a".into()]),
            Some("patch_id_rebase")
        );
        assert_eq!(fix.matched_by(&["a".into()]), None);
        fix.commits = vec!["a".into(), "a".into()];
        assert_eq!(
            fix.matched_by(&["a".into()]),
            None,
            "each commit needs an equivalent"
        );
        assert_eq!(
            fix.matched_by(&["a".into(), "a".into()]),
            Some("patch_id_rebase")
        );
        fix.commits.clear();
        fix.whole = None;
        assert_eq!(fix.matched_by(&[]), None);
    }
}
