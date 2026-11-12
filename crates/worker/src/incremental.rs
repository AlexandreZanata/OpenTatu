//! Incremental scope and diff context (phase 7, ADR-0033, R14–R16).

use opentatu_context::{
    FileKind, Selection, SnapshotFiles, affected_entry_points, classify, diff_allowed, render_diff,
};
use opentatu_core::{AuditKind, EntryPoint, FileChange, FileIndex as _, SecurityMap};

use crate::{Error, ai_run::AiRun};

/// The diff of this run, absent for FULL/REVERIFICATION.
#[derive(Debug, Clone)]
pub(crate) struct Incremental {
    pub changes: Vec<FileChange>,
    pub diff: Selection,
    pub previous_map: Option<SecurityMap>,
    pub previous_files: SnapshotFiles,
}

impl Incremental {
    pub async fn load(run: &AiRun) -> Result<Option<Self>, Error> {
        if run.audit.kind != AuditKind::Incremental {
            return Ok(None);
        }
        let previous = run.previous_audit.as_ref().ok_or_else(|| {
            opentatu_services::Error::Conflict(
                "incremental audit has no baseline; run FULL first".into(),
            )
        })?;
        let changes = run
            .worker
            .snapshots
            .changed_files(&run.audit.app, &previous.commit, &run.audit.commit)
            .await?;
        let snapshot = run
            .worker
            .snapshots
            .capture(&run.audit.app, &previous.commit)
            .await?;
        let previous_files = SnapshotFiles::scan(&snapshot.dir)?;
        let mut patches = Vec::new();
        let mut bytes = 0usize;
        const BUDGET: usize = 120_000;
        for change in changes.iter().filter(|c| diff_allowed(c)) {
            // Snapshot membership excludes symlinks/build/dependency trees on either side.
            if run.files.read_text(&change.path).is_none()
                && previous_files.read_text(&change.path).is_none()
            {
                continue;
            }
            if bytes >= BUDGET {
                break;
            }
            let mut paths = vec![change.path.as_str()];
            paths.extend(change.previous_path.as_deref());
            let patch = run
                .worker
                .snapshots
                .unified_diff(&run.audit.app, &previous.commit, &run.audit.commit, &paths)
                .await?;
            bytes = bytes.saturating_add(patch.len());
            patches.push((change.path.clone(), patch));
        }
        let diff = render_diff(&changes, &patches, BUDGET);
        let previous_map = run.worker.services.map_of_audit(&previous.id).await?;
        Ok(Some(Self {
            changes,
            diff,
            previous_map,
            previous_files,
        }))
    }

    pub fn paths(&self) -> Vec<String> {
        self.changes
            .iter()
            .flat_map(|c| std::iter::once(c.path.clone()).chain(c.previous_path.clone()))
            .collect()
    }

    pub fn entries(&self, run: &AiRun, map: &SecurityMap) -> Vec<String> {
        if self.changes.is_empty() {
            return Vec::new();
        }
        let changed = self.paths();
        let current = affected_entry_points(&run.files, &map.content.entry_points, &changed);
        let old_entries = self
            .previous_map
            .as_ref()
            .map(|m| m.content.entry_points.as_slice())
            .unwrap_or_default();
        let previous = affected_entry_points(&self.previous_files, old_entries, &changed);
        map.content
            .entry_points
            .iter()
            .filter(|entry| {
                current.contains(&entry.id)
                    || self.previous_map.is_some()
                        && old_entries.iter().all(|old| !same_entry(entry, old))
                    || old_entries
                        .iter()
                        .any(|old| same_entry(entry, old) && previous.contains(&old.id))
            })
            .map(|entry| entry.id.clone())
            .collect()
    }

    pub fn source_seeds(&self, files: &SnapshotFiles) -> Vec<opentatu_context::Seed> {
        self.changes
            .iter()
            .filter(|c| {
                diff_allowed(c) && classify(&c.path) == FileKind::Source && files.contains(&c.path)
            })
            .map(|c| opentatu_context::Seed {
                path: c.path.clone(),
                line: None,
            })
            .collect()
    }
}

fn same_entry(a: &EntryPoint, b: &EntryPoint) -> bool {
    a.method == b.method && a.path == b.path && a.file == b.file && a.symbol == b.symbol
}
