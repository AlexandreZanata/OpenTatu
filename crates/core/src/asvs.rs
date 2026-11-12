//! OWASP ASVS — the requirement catalog (spec §A.4 "Requirement", §C.1, R14).
//!
//! The official ASVS 5.0.0 flat JSON (`config/asvs/asvs-5.0.0.flat.json`, CC BY-SA 4.0,
//! unmodified) is embedded in the binary, so requirement ids stored in maps and coverage
//! always refer to one fixed version. Which levels count as applicable is an owner
//! decision ([`APPLICABLE_LEVEL`], Q-013).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::Error;

/// Version of the embedded catalog.
pub const ASVS_VERSION: &str = "5.0.0";

crate::text_enum! {
    /// ASVS verification level (1 = baseline, 3 = most demanding).
    pub enum AsvsLevel {
        /// Level 1.
        L1 => "1",
        /// Level 2.
        L2 => "2",
        /// Level 3.
        L3 => "3",
    }
}

/// Requirements up to this level are applicable to every app: levels 1, 2 and 3 — all 345
/// requirements of ASVS 5.0.0 (owner decision Q-013, 2026-11-12).
pub const APPLICABLE_LEVEL: AsvsLevel = AsvsLevel::L3;

/// One ASVS requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsvsRequirement {
    /// Requirement id, e.g. `V8.2.1`.
    pub id: String,
    /// Chapter id, e.g. `V8`.
    pub chapter_id: String,
    /// Chapter name, e.g. `Authorization`.
    pub chapter_name: String,
    /// Section id, e.g. `V8.2`.
    pub section_id: String,
    /// Section name.
    pub section_name: String,
    /// Requirement text (official English text).
    pub description: String,
    /// Level that introduces it.
    pub level: AsvsLevel,
}

/// The requirements of one ASVS version, in catalog order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsvsCatalog {
    requirements: Vec<AsvsRequirement>,
    index: BTreeMap<String, usize>,
}

/// Shape of the official flat JSON export.
#[derive(Deserialize)]
struct FlatExport {
    requirements: Vec<FlatRequirement>,
}

#[derive(Deserialize)]
struct FlatRequirement {
    chapter_id: String,
    chapter_name: String,
    section_id: String,
    section_name: String,
    req_id: String,
    req_description: String,
    #[serde(rename = "L")]
    level: AsvsLevel,
}

static EMBEDDED: LazyLock<Result<AsvsCatalog, String>> = LazyLock::new(|| {
    AsvsCatalog::from_flat_json(include_str!("../../../config/asvs/asvs-5.0.0.flat.json"))
        .map_err(|error| error.to_string())
});

impl AsvsCatalog {
    /// The embedded ASVS [`ASVS_VERSION`] catalog.
    pub fn embedded() -> Result<&'static Self, Error> {
        EMBEDDED
            .as_ref()
            .map_err(|message| Error::InvalidCatalog(message.clone()))
    }

    /// Parses an official ASVS flat JSON export.
    pub fn from_flat_json(text: &str) -> Result<Self, Error> {
        let export: FlatExport =
            serde_json::from_str(text).map_err(|error| Error::InvalidCatalog(error.to_string()))?;
        let mut index = BTreeMap::new();
        let mut requirements = Vec::with_capacity(export.requirements.len());
        for raw in export.requirements {
            if index
                .insert(raw.req_id.clone(), requirements.len())
                .is_some()
            {
                return Err(Error::InvalidCatalog(format!(
                    "duplicate id {}",
                    raw.req_id
                )));
            }
            requirements.push(AsvsRequirement {
                id: raw.req_id,
                chapter_id: raw.chapter_id,
                chapter_name: raw.chapter_name,
                section_id: raw.section_id,
                section_name: raw.section_name,
                description: raw.req_description,
                level: raw.level,
            });
        }
        Ok(Self {
            requirements,
            index,
        })
    }

    /// Every requirement, in catalog order.
    pub fn requirements(&self) -> &[AsvsRequirement] {
        &self.requirements
    }

    /// Requirements introduced at `level` or below, in catalog order.
    pub fn up_to(&self, level: AsvsLevel) -> impl Iterator<Item = &AsvsRequirement> {
        self.requirements.iter().filter(move |r| r.level <= level)
    }

    /// The requirement with this id. Accepts the usual spellings of an id written by a
    /// model (`V8.2.1`, `v8.2.1`, `8.2.1`, ` V8.2.1 `).
    pub fn resolve(&self, id: &str) -> Option<&AsvsRequirement> {
        let trimmed = id.trim();
        let digits = trimmed
            .strip_prefix('V')
            .or_else(|| trimmed.strip_prefix('v'))
            .unwrap_or(trimmed);
        self.index
            .get(&format!("V{digits}"))
            .and_then(|position| self.requirements.get(*position))
    }

    /// Position of a requirement in catalog order (for stable sorting).
    pub fn position(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_asvs_5_with_345_requirements() {
        let catalog = AsvsCatalog::embedded().unwrap();
        assert_eq!(catalog.requirements().len(), 345);
        assert_eq!(catalog.up_to(AsvsLevel::L1).count(), 70);
        assert_eq!(catalog.up_to(AsvsLevel::L2).count(), 253);
        assert_eq!(catalog.up_to(APPLICABLE_LEVEL).count(), 345);
        let first = &catalog.requirements()[0];
        assert_eq!(first.id, "V1.1.1");
        assert_eq!(first.chapter_name, "Encoding and Sanitization");
    }

    #[test]
    fn ids_resolve_with_common_spellings_only_when_they_exist() {
        let catalog = AsvsCatalog::embedded().unwrap();
        for spelling in ["V8.2.1", "v8.2.1", "8.2.1", " V8.2.1 "] {
            assert_eq!(
                catalog.resolve(spelling).map(|r| r.id.as_str()),
                Some("V8.2.1")
            );
        }
        assert!(catalog.resolve("V99.1.1").is_none());
        assert!(catalog.resolve("V4.2.1 (ASVS 4)").is_none());
    }
}
