//! Preservation of unchanged security-map records during incremental updates (R14).

use std::collections::BTreeMap;

use crate::{EntryPoint, MapContent};

impl MapContent {
    /// Keeps unchanged entry points and file-backed map parts. `affected` includes
    /// changed paths and handlers whose related code changed. The result is sanitized
    /// against the new commit afterward, so removed files never survive preservation.
    pub fn preserve_unchanged(mut self, previous: &Self, affected: &[String]) -> Self {
        let same = |a: &EntryPoint, b: &EntryPoint| {
            a.method == b.method && a.path == b.path && a.file == b.file && a.symbol == b.symbol
        };
        let mut ids = BTreeMap::new();
        for (n, entry) in self.entry_points.iter_mut().enumerate() {
            let incoming_id = entry.id.clone();
            if let Some(old) = previous.entry_points.iter().find(|old| same(entry, old)) {
                if !affected.contains(&old.file) {
                    *entry = old.clone();
                }
                entry.id = old.id.clone();
            } else {
                entry.id = format!("INC-{n}");
            }
            ids.insert(incoming_id, entry.id.clone());
        }
        for boundary in &mut self.trust_boundaries {
            boundary.entry_points = boundary
                .entry_points
                .iter()
                .filter_map(|id| ids.get(id).cloned())
                .collect();
        }
        self.authorization.entry_points_without_check = self
            .authorization
            .entry_points_without_check
            .iter()
            .filter_map(|id| ids.get(id).cloned())
            .collect();
        for old in &previous.entry_points {
            if !affected.contains(&old.file)
                && !self.entry_points.iter().any(|entry| same(entry, old))
            {
                self.entry_points.push(old.clone());
            }
        }
        for old in &previous.components {
            if old.files.iter().all(|p| !affected.contains(p)) {
                self.components.retain(|c| c.name != old.name);
                self.components.push(old.clone());
            }
        }
        for old in &previous.integrations {
            if old.files.iter().all(|p| !affected.contains(p)) {
                self.integrations.retain(|c| c.name != old.name);
                self.integrations.push(old.clone());
            }
        }
        for old in &previous.critical_dependencies {
            if !affected.contains(&old.file) {
                self.critical_dependencies.retain(|c| c.name != old.name);
                self.critical_dependencies.push(old.clone());
            }
        }
        for old in &previous.trust_boundaries {
            if let Some(current) = self
                .trust_boundaries
                .iter_mut()
                .find(|b| b.name == old.name)
            {
                for id in &old.entry_points {
                    if previous
                        .entry_points
                        .iter()
                        .any(|e| &e.id == id && !affected.contains(&e.file))
                        && !current.entry_points.contains(id)
                    {
                        current.entry_points.push(id.clone());
                    }
                }
            } else {
                self.trust_boundaries.push(old.clone());
            }
        }
        let touches = |references: &[String]| {
            references
                .iter()
                .any(|reference| affected.iter().any(|path| reference.contains(path)))
        };
        for old in &previous.sensitive_data {
            let changed =
                touches(&old.read_at) || touches(&old.written_at) || touches(&old.exposed_at);
            if !changed
                && !self.sensitive_data.iter().any(|current| {
                    current.kind == old.kind && current.description == old.description
                })
            {
                self.sensitive_data.push(old.clone());
            }
        }
        if !previous.authentication.files.is_empty()
            && !touches(&previous.authentication.files)
            && self
                .authentication
                .files
                .iter()
                .all(|path| previous.authentication.files.contains(path))
        {
            self.authentication = previous.authentication.clone();
        }
        for id in &previous.authorization.entry_points_without_check {
            if previous
                .entry_points
                .iter()
                .any(|entry| &entry.id == id && !affected.contains(&entry.file))
                && !self.authorization.entry_points_without_check.contains(id)
            {
                self.authorization
                    .entry_points_without_check
                    .push(id.clone());
            }
        }
        for id in &previous.app_asvs {
            if !self.app_asvs.contains(id) {
                self.app_asvs.push(id.clone());
            }
        }
        self
    }
}
