//! System values of the base prompt (spec §C.0), built from the audit being run. Dates come
//! from the audit record (R03); the model never generates them.

use std::collections::BTreeMap;

use opentatu_core::{App, Audit};

use crate::{Block, fence};

/// What the base prompt says about the current run.
#[derive(Debug, Clone, Copy)]
pub struct RunContext<'a> {
    /// Owner's name (`OPENTATU_OWNER`, Q-009).
    pub owner: &'a str,
    /// App under analysis.
    pub app: &'a App,
    /// Running audit.
    pub audit: &'a Audit,
    /// Previous final audit of the app, if any.
    pub previous: Option<&'a Audit>,
    /// Remaining budget summary.
    pub remaining_budget: &'a str,
    /// Generated `MEMORY.md`.
    pub memory: &'a str,
}

impl RunContext<'_> {
    /// Values of every base-prompt variable except `role` and `audit_id` (added per call).
    pub fn variables(&self) -> BTreeMap<&'static str, String> {
        let started = self.audit.started_at.unwrap_or(self.audit.created_at);
        let profile = &self.app.profile;
        let none = || "none".to_owned();
        BTreeMap::from([
            ("owner", self.owner.to_owned()),
            ("audit_kind", self.audit.kind.to_string()),
            ("app_name", profile.name.clone()),
            ("app_slug", profile.slug.to_string()),
            ("commit", self.audit.commit.to_string()),
            ("started_display", started.display_pt_br()),
            ("started_iso", started.iso()),
            (
                "previous_audit_id",
                self.previous.map_or_else(none, |a| a.id.to_string()),
            ),
            (
                "previous_display",
                self.previous.map_or_else(none, |a| {
                    a.started_at.unwrap_or(a.created_at).display_pt_br()
                }),
            ),
            ("remaining_budget", self.remaining_budget.to_owned()),
            ("memory", fence(Block::Memory, self.memory)),
            (
                "lab_available",
                if profile.lab_url.is_some() {
                    "yes"
                } else {
                    "no"
                }
                .to_owned(),
            ),
            ("lab_url", profile.lab_url.clone().unwrap_or_else(none)),
        ])
    }
}
