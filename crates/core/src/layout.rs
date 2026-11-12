//! The data directory layout (spec §B.4) — the ONLY place paths under the data directory
//! are built. Pure path arithmetic: nothing here touches the disk.
//!
//! ```text
//! <root>/opentatu.db                         SQLite database
//! <root>/apps/<slug>/app.json                app profile
//! <root>/apps/<slug>/MEMORY.md               working memory (generated)
//! <root>/apps/<slug>/OWNER-NOTES.md          owner's free notes (hand-edited, read into MEMORY.md)
//! <root>/apps/<slug>/history.jsonl           mirror of the app's events
//! <root>/apps/<slug>/map/current.json        latest security map
//! <root>/apps/<slug>/map/versions/<map>.json every map version
//! <root>/apps/<slug>/audits/<audit>/audit.json
//! <root>/apps/<slug>/audits/<audit>/coverage.json          ASVS × entry points matrix
//! <root>/apps/<slug>/audits/<audit>/llm_usage.jsonl        one line per model call
//! <root>/apps/<slug>/audits/<audit>/scanners/<tool>.json   raw scanner outputs
//! <root>/scanner-cache/<tool>/             offline rules and vulnerability databases
//! <root>/repos/<slug>.git                    bare mirror of the app's repository
//! <root>/snapshots/<slug>/<commit>/          checkout of one exact commit (R02)
//! ```

use std::path::{Path, PathBuf};

use crate::{AuditId, CommitHash, MapId, ScannerKind, Slug};

/// Paths of every artifact under the data directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLayout {
    root: PathBuf,
}

impl DataLayout {
    /// Layout rooted at the data directory (`OPENTATU_DATA_DIR`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The data directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// SQLite database file.
    pub fn database_file(&self) -> PathBuf {
        self.root.join("opentatu.db")
    }

    /// Directory of one app.
    pub fn app_dir(&self, slug: &Slug) -> PathBuf {
        self.root.join("apps").join(slug.as_str())
    }

    /// `app.json`.
    pub fn app_profile_file(&self, slug: &Slug) -> PathBuf {
        self.app_dir(slug).join("app.json")
    }

    /// `MEMORY.md` (spec §B.9).
    pub fn memory_file(&self, slug: &Slug) -> PathBuf {
        self.app_dir(slug).join("MEMORY.md")
    }

    /// `OWNER-NOTES.md`: free text written by the owner, never by the AI.
    pub fn owner_notes_file(&self, slug: &Slug) -> PathBuf {
        self.app_dir(slug).join("OWNER-NOTES.md")
    }

    /// `history.jsonl` (spec §B.1).
    pub fn history_file(&self, slug: &Slug) -> PathBuf {
        self.app_dir(slug).join("history.jsonl")
    }

    /// Directory of one audit.
    pub fn audit_dir(&self, audit: &AuditId) -> PathBuf {
        self.app_dir(audit.slug())
            .join("audits")
            .join(audit.as_str())
    }

    /// `audit.json` (spec §B.5).
    pub fn audit_record_file(&self, audit: &AuditId) -> PathBuf {
        self.audit_dir(audit).join("audit.json")
    }

    /// `coverage.json` (ASVS × entry points, spec §B.4).
    pub fn coverage_file(&self, audit: &AuditId) -> PathBuf {
        self.audit_dir(audit).join("coverage.json")
    }

    /// `llm_usage.jsonl`: one line per model call of the audit (never the key or the code).
    pub fn llm_usage_file(&self, audit: &AuditId) -> PathBuf {
        self.audit_dir(audit).join("llm_usage.jsonl")
    }

    /// `map/current.json`: the latest security map of the app.
    pub fn current_map_file(&self, slug: &Slug) -> PathBuf {
        self.app_dir(slug).join("map").join("current.json")
    }

    /// `map/versions/<map>.json`: one version of the security map.
    pub fn map_version_file(&self, map: &MapId) -> PathBuf {
        self.app_dir(map.slug())
            .join("map")
            .join("versions")
            .join(format!("{}.json", map.as_str()))
    }

    /// Raw scanner outputs of one audit (`scanners/<tool>.json`).
    pub fn scanner_output_dir(&self, audit: &AuditId) -> PathBuf {
        self.audit_dir(audit).join("scanners")
    }

    /// Offline databases and rules of one scanner, refreshed by `opentatu scanners update`.
    pub fn scanner_cache_dir(&self, scanner: ScannerKind) -> PathBuf {
        self.root.join("scanner-cache").join(scanner.as_str())
    }

    /// Bare mirror of the app's repository.
    pub fn mirror_dir(&self, slug: &Slug) -> PathBuf {
        self.root
            .join("repos")
            .join(format!("{}.git", slug.as_str()))
    }

    /// Checkout of one exact commit.
    pub fn snapshot_dir(&self, slug: &Slug, commit: &CommitHash) -> PathBuf {
        self.root
            .join("snapshots")
            .join(slug.as_str())
            .join(commit.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_spec_b4() {
        let layout = DataLayout::new("/data");
        let slug = Slug::new("abastevo").unwrap();
        let audit: AuditId = "AUD-ABASTEVO-20261129-1432".parse().unwrap();
        let commit: CommitHash = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap();
        assert_eq!(layout.database_file(), Path::new("/data/opentatu.db"));
        assert_eq!(
            layout.memory_file(&slug),
            Path::new("/data/apps/abastevo/MEMORY.md")
        );
        assert_eq!(
            layout.audit_record_file(&audit),
            Path::new("/data/apps/abastevo/audits/AUD-ABASTEVO-20261129-1432/audit.json")
        );
        assert_eq!(
            layout.mirror_dir(&slug),
            Path::new("/data/repos/abastevo.git")
        );
        assert_eq!(
            layout.map_version_file(&MapId::for_audit(&audit)),
            Path::new("/data/apps/abastevo/map/versions/MAP-ABASTEVO-20261129-1432.json")
        );
        assert_eq!(
            layout.coverage_file(&audit),
            Path::new("/data/apps/abastevo/audits/AUD-ABASTEVO-20261129-1432/coverage.json")
        );
        assert_eq!(
            layout.snapshot_dir(&slug, &commit),
            Path::new("/data/snapshots/abastevo/4c4a5df92ace8d8068e57362a7308e8b9d031b3a")
        );
    }
}
