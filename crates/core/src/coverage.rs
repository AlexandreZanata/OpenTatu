//! ASVS coverage of an audit (spec §A.2, §B.4 `coverage.json`, §B.5 `coverage`, R14):
//! the applicable requirements × entry points matrix and its summary.
//!
//! Derived state: computed from the audit's [`SecurityMap`] and the verification results
//! (phase 4+), never stored as a source of truth. A requirement without a result is
//! `NOT_TESTED` — never "safe" (R14).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AsvsCatalog, AsvsLevel, AuditId, MapId, SecurityMap};

crate::text_enum! {
    /// Verification status of one applicable requirement.
    pub enum RequirementStatus {
        /// Verified as met.
        Verified => "VERIFIED",
        /// Verified as not met (a finding exists).
        Failed => "FAILED",
        /// Not verified in this audit.
        NotTested => "NOT_TESTED",
    }
}

/// Coverage numbers (the `coverage` section of `audit.json`, spec §B.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageSummary {
    /// Requirements associated with the app or one of its entry points.
    pub applicable_requirements: usize,
    /// Verified as met.
    pub verified: usize,
    /// Verified as not met.
    pub failed: usize,
    /// Not verified.
    pub not_tested: usize,
    /// `verified * 100 / applicable_requirements`, rounded down (0 when nothing applies).
    pub percent: usize,
}

/// One applicable requirement in the matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementCoverage {
    /// Requirement id, e.g. `V8.2.1`.
    pub id: String,
    /// Level that introduces it.
    pub level: AsvsLevel,
    /// Chapter, e.g. `V8 Authorization`.
    pub chapter: String,
    /// Status in this audit.
    pub status: RequirementStatus,
    /// Entry points it applies to (`EP-NNN`); empty when it applies to the app as a whole.
    pub entry_points: Vec<String>,
    /// Whether the cartographer associated it with the app as a whole.
    pub app_wide: bool,
}

/// The coverage matrix of one audit (`coverage.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Audit.
    pub audit_id: AuditId,
    /// Map the matrix is based on.
    pub map_id: MapId,
    /// ASVS version.
    pub asvs_version: String,
    /// Highest level considered applicable.
    pub applicable_level: AsvsLevel,
    /// Requirements of the catalog up to that level (applicable or not).
    pub catalog_requirements: usize,
    /// Summary numbers.
    pub summary: CoverageSummary,
    /// Applicable requirements, in catalog order.
    pub requirements: Vec<RequirementCoverage>,
}

/// Status of each requirement from what the audit did (ADR-0025):
/// - `FAILED` — a confirmed finding of the audit (not dismissed by the skeptic) cites it;
/// - `NOT_TESTED` — a hypothesis citing it is unresolved (inconclusive or not verified);
/// - `VERIFIED` — the hunter reported checking it and nothing above applies;
/// - everything else stays `NOT_TESTED` (missing from the map) — never "safe" (R14).
pub fn requirement_statuses(
    checked: &BTreeSet<String>,
    failed: &BTreeSet<String>,
    unresolved: &BTreeSet<String>,
) -> BTreeMap<String, RequirementStatus> {
    let mut statuses = BTreeMap::new();
    for id in checked {
        statuses.insert(id.clone(), RequirementStatus::Verified);
    }
    for id in unresolved {
        statuses.insert(id.clone(), RequirementStatus::NotTested);
    }
    for id in failed {
        statuses.insert(id.clone(), RequirementStatus::Failed);
    }
    statuses
}

impl Coverage {
    /// Builds the matrix of `map`: every requirement up to `level` that the map associates
    /// with the app or an entry point, with its status from `statuses` (missing =
    /// `NOT_TESTED`).
    pub fn compute(
        map: &SecurityMap,
        catalog: &AsvsCatalog,
        level: AsvsLevel,
        statuses: &BTreeMap<String, RequirementStatus>,
    ) -> Self {
        let mut by_requirement: BTreeMap<&str, (Vec<String>, bool)> = BTreeMap::new();
        for entry in &map.content.entry_points {
            for id in &entry.applicable_asvs {
                by_requirement
                    .entry(id)
                    .or_default()
                    .0
                    .push(entry.id.clone());
            }
        }
        for id in &map.content.app_asvs {
            by_requirement.entry(id).or_default().1 = true;
        }
        let requirements: Vec<RequirementCoverage> = catalog
            .up_to(level)
            .filter_map(|requirement| {
                let (entry_points, app_wide) = by_requirement.get(requirement.id.as_str())?;
                Some(RequirementCoverage {
                    id: requirement.id.clone(),
                    level: requirement.level,
                    chapter: format!("{} {}", requirement.chapter_id, requirement.chapter_name),
                    status: statuses
                        .get(&requirement.id)
                        .copied()
                        .unwrap_or(RequirementStatus::NotTested),
                    entry_points: entry_points.clone(),
                    app_wide: *app_wide,
                })
            })
            .collect();
        let count = |status| requirements.iter().filter(|r| r.status == status).count();
        let verified = count(RequirementStatus::Verified);
        let applicable = requirements.len();
        let summary = CoverageSummary {
            applicable_requirements: applicable,
            verified,
            failed: count(RequirementStatus::Failed),
            not_tested: count(RequirementStatus::NotTested),
            percent: (verified * 100).checked_div(applicable).unwrap_or(0),
        };
        Self {
            audit_id: map.audit_id.clone(),
            map_id: map.map_id.clone(),
            asvs_version: map.asvs_version.clone(),
            applicable_level: level,
            catalog_requirements: catalog.up_to(level).count(),
            summary,
            requirements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::tests::content;
    use crate::{AuthenticationRequirement, EntryPoint};

    fn map() -> SecurityMap {
        let entry = |id: &str, asvs: &[&str]| EntryPoint {
            id: id.into(),
            method: "POST".into(),
            path: "/login".into(),
            file: "src/routes.rs".into(),
            line: None,
            symbol: "login".into(),
            authentication: AuthenticationRequirement::No,
            allowed_roles: vec![],
            inputs: vec!["body".into()],
            authorization_check: "nenhuma visível".into(),
            sensitive_data: vec![],
            applicable_asvs: asvs.iter().map(|a| (*a).to_owned()).collect(),
        };
        let audit: AuditId = "AUD-DEMO-20261112-0900".parse().unwrap();
        SecurityMap {
            map_id: MapId::for_audit(&audit),
            app: audit.slug().clone(),
            audit_id: audit,
            commit: "4c4a5df92ace8d8068e57362a7308e8b9d031b3a".parse().unwrap(),
            created_at: "2026-11-12T09:05:00-03:00".parse().unwrap(),
            asvs_version: "5.0.0".into(),
            model: "deepseek-v4.1-flash".into(),
            prompt_versions: vec![],
            content: content(vec![
                entry("EP-001", &["V6.2.1", "V8.2.1"]),
                entry("EP-002", &["V8.2.1"]),
            ]),
            gaps: vec![],
            not_analyzed: vec![],
            system_notes: vec![],
        }
    }

    #[test]
    fn r14_untested_requirements_are_reported_as_not_tested() {
        let catalog = AsvsCatalog::embedded().unwrap();
        let coverage = Coverage::compute(&map(), catalog, AsvsLevel::L3, &BTreeMap::new());
        let ids: Vec<_> = coverage
            .requirements
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(ids, ["V6.2.1", "V8.2.1", "V13.1.1"], "catalog order");
        assert_eq!(coverage.requirements[1].entry_points, ["EP-001", "EP-002"]);
        assert!(coverage.requirements[2].app_wide);
        assert_eq!(
            coverage.summary,
            CoverageSummary {
                applicable_requirements: 3,
                verified: 0,
                failed: 0,
                not_tested: 3,
                percent: 0
            }
        );
        assert_eq!(coverage.catalog_requirements, 345);
        assert_eq!(coverage.map_id.as_str(), "MAP-DEMO-20261112-0900");
    }

    #[test]
    fn r14_failures_win_and_unresolved_hypotheses_are_not_verified() {
        let set = |ids: &[&str]| {
            ids.iter()
                .map(|id| (*id).to_owned())
                .collect::<BTreeSet<_>>()
        };
        let statuses = requirement_statuses(
            &set(&["V1.1.1", "V8.2.1", "V6.2.1"]),
            &set(&["V8.2.1"]),
            &set(&["V6.2.1"]),
        );
        assert_eq!(statuses["V1.1.1"], RequirementStatus::Verified);
        assert_eq!(statuses["V8.2.1"], RequirementStatus::Failed);
        assert_eq!(statuses["V6.2.1"], RequirementStatus::NotTested);
        assert!(
            !statuses.contains_key("V13.1.1"),
            "unknown = NOT_TESTED by default"
        );
    }

    #[test]
    fn statuses_and_level_filter_drive_the_summary() {
        let catalog = AsvsCatalog::embedded().unwrap();
        let statuses = BTreeMap::from([
            ("V8.2.1".to_owned(), RequirementStatus::Verified),
            ("V6.2.1".to_owned(), RequirementStatus::Failed),
        ]);
        let all = Coverage::compute(&map(), catalog, AsvsLevel::L3, &statuses);
        assert_eq!(
            (
                all.summary.verified,
                all.summary.failed,
                all.summary.percent
            ),
            (1, 1, 33)
        );
        let level_one = Coverage::compute(&map(), catalog, AsvsLevel::L1, &statuses);
        assert!(
            level_one
                .requirements
                .iter()
                .all(|r| r.level == AsvsLevel::L1)
        );
        assert_eq!(level_one.catalog_requirements, 70);
    }
}
