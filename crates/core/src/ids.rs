//! Identifiers (spec §B.2). Values of these types are always well-formed: they are
//! validated when built and when parsed.
//!
//! | Type | Format | Example |
//! |---|---|---|
//! | [`Slug`] | lowercase letters, digits, hyphens | `abastevo` |
//! | [`AuditId`] | `AUD-<SLUG>-<YYYYMMDD>-<HHMM>[-N]` | `AUD-ABASTEVO-20261129-1432` |
//! | [`MapId`] | `MAP-<SLUG>-<YYYYMMDD>-<HHMM>[-N]` | `MAP-ABASTEVO-20261129-1432` |
//! | [`FindingId`] | `<SLUG>-SEC-<NNNN>` | `ABASTEVO-SEC-0004` |
//! | [`HypothesisId`] | `<AUDIT>-H-<NNN>` | `AUD-ABASTEVO-20261129-1432-H-001` |
//! | [`EvidenceId`] | `<AUDIT>-EV-<NNN>` | `AUD-ABASTEVO-20261129-1432-EV-001` |
//! | [`TaskId`] | `<FINDING>-T<N>` | `ABASTEVO-SEC-0004-T1` |
//! | [`EventId`] | `EVT-<NNNNNNNN>` | `EVT-00000184` |
//! | [`CommitHash`] | 40 or 64 lowercase hex (SHA-1 / SHA-256 repositories) | `4c4a5df9…` |
//!
//! Never build or parse these strings by hand: use the constructors in this module.
//! Numbers wider than their padding are allowed (`-SEC-10000`), but padding is canonical:
//! `-SEC-04` is rejected.

use std::str::FromStr;

use crate::macros::impl_text;
use crate::{Error, Timestamp};

const SLUG_MIN_LEN: usize = 2;
const SLUG_MAX_LEN: usize = 40;

/// Builds the error for an identifier that does not match its format.
fn invalid(kind: &'static str, value: &str, format: &'static str) -> Error {
    Error::InvalidId {
        kind,
        value: value.to_owned(),
        format,
    }
}

/// Parses `digits` as a number whose canonical form is zero-padded to `width`.
fn parse_padded(digits: &str, width: usize) -> Option<u64> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let number: u64 = digits.parse().ok()?;
    (format!("{number:0width$}") == digits).then_some(number)
}

// ---------------------------------------------------------------------------------------
// Slug
// ---------------------------------------------------------------------------------------

/// App slug: 2–40 chars of lowercase letters, digits and single hyphens; starts with a
/// letter and does not end with a hyphen. Example: `abastevo`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slug(String);

impl Slug {
    /// Validates and builds a slug.
    pub fn new(text: &str) -> Result<Self, Error> {
        let bytes = text.as_bytes();
        let valid = (SLUG_MIN_LEN..=SLUG_MAX_LEN).contains(&bytes.len())
            && bytes.first().is_some_and(u8::is_ascii_lowercase)
            && bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
            && !text.ends_with('-')
            && !text.contains("--");
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(Error::InvalidSlug(text.to_owned()))
        }
    }

    /// The slug text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Uppercase form used inside IDs (`abastevo` → `ABASTEVO`).
    pub fn upper(&self) -> String {
        self.0.to_ascii_uppercase()
    }

    /// Parses the uppercase form found inside IDs.
    fn from_upper(text: &str) -> Option<Self> {
        if text.bytes().any(|b| b.is_ascii_lowercase()) {
            return None;
        }
        Self::new(&text.to_ascii_lowercase()).ok()
    }
}

impl FromStr for Slug {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl_text!(Slug);

// ---------------------------------------------------------------------------------------
// Stamped IDs: `<PREFIX>-<SLUG>-<YYYYMMDD>-<HHMM>[-N]` (AuditId, MapId)
// ---------------------------------------------------------------------------------------

/// Builds a stamped ID. `sequence` <= 1 means "first in that minute" (no suffix).
fn build_stamped(prefix: &str, slug: &Slug, at: &Timestamp, sequence: u32) -> String {
    let mut text = format!(
        "{prefix}-{}-{}-{}",
        slug.upper(),
        at.compact_date(),
        at.compact_time()
    );
    if sequence > 1 {
        text.push('-');
        text.push_str(&sequence.to_string());
    }
    text
}

/// Parses a stamped ID from the right, so slugs may contain hyphens and digits.
fn parse_stamped(prefix: &str, text: &str) -> Option<Slug> {
    let rest = text.strip_prefix(prefix)?.strip_prefix('-')?;
    let parts: Vec<&str> = rest.split('-').collect();
    let slug_parts = match parts.as_slice() {
        [slug @ .., date, time, seq]
            if !slug.is_empty() && is_date(date) && is_time(time) && is_sequence(seq) =>
        {
            slug
        }
        [slug @ .., date, time] if !slug.is_empty() && is_date(date) && is_time(time) => slug,
        _ => return None,
    };
    Slug::from_upper(&slug_parts.join("-"))
}

/// `YYYYMMDD` that is a real calendar date.
fn is_date(text: &str) -> bool {
    if text.len() != 8 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        text[0..4].parse::<i16>(),
        text[4..6].parse::<i8>(),
        text[6..8].parse::<i8>(),
    ) else {
        return false;
    };
    jiff::civil::Date::new(year, month, day).is_ok()
}

/// `HHMM` with a valid hour and minute.
fn is_time(text: &str) -> bool {
    if text.len() != 4 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    matches!((text[0..2].parse::<u8>(), text[2..4].parse::<u8>()), (Ok(h), Ok(m)) if h < 24 && m < 60)
}

/// Collision suffix: canonical number >= 2.
fn is_sequence(text: &str) -> bool {
    parse_padded(text, 1).is_some_and(|n| n >= 2)
}

/// Audit identifier: `AUD-<SLUG>-<YYYYMMDD>-<HHMM>[-N]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditId {
    text: String,
    slug: Slug,
}

impl AuditId {
    const FORMAT: &'static str = "AUD-<SLUG>-<YYYYMMDD>-<HHMM>[-N]";

    /// ID of an audit of `slug` stamped with `at` (local time of the default zone).
    ///
    /// Always pass the audit's `created_at`: the ID exists from creation, even while the
    /// audit waits in the queue (Q-001, answered).
    ///
    /// `sequence` is the audit's order within that minute: 1 for the first (no suffix),
    /// then 2, 3... on collision (spec §B.2). Values below 1 count as 1.
    pub fn new(slug: &Slug, at: &Timestamp, sequence: u32) -> Self {
        Self {
            text: build_stamped("AUD", slug, at, sequence),
            slug: slug.clone(),
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Slug of the audited app.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FromStr for AuditId {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_stamped("AUD", text)
            .map(|slug| Self {
                text: text.to_owned(),
                slug,
            })
            .ok_or_else(|| invalid("AuditId", text, Self::FORMAT))
    }
}

impl_text!(AuditId);

/// Security map version identifier: `MAP-<SLUG>-<YYYYMMDD>-<HHMM>[-N]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MapId {
    text: String,
    slug: Slug,
}

impl MapId {
    const FORMAT: &'static str = "MAP-<SLUG>-<YYYYMMDD>-<HHMM>[-N]";

    /// Map version produced by `audit` (same stamp: one map version per audit).
    pub fn for_audit(audit: &AuditId) -> Self {
        let stamp = audit.as_str().strip_prefix("AUD").unwrap_or(audit.as_str());
        Self {
            text: format!("MAP{stamp}"),
            slug: audit.slug().clone(),
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Slug of the mapped app.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FromStr for MapId {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_stamped("MAP", text)
            .map(|slug| Self {
                text: text.to_owned(),
                slug,
            })
            .ok_or_else(|| invalid("MapId", text, Self::FORMAT))
    }
}

impl_text!(MapId);

// ---------------------------------------------------------------------------------------
// FindingId: `<SLUG>-SEC-<NNNN>`
// ---------------------------------------------------------------------------------------

/// Finding identifier: `<SLUG>-SEC-<NNNN>`, sequential per app and never reused.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingId {
    text: String,
    slug: Slug,
    number: u32,
}

impl FindingId {
    const FORMAT: &'static str = "<SLUG>-SEC-<NNNN>";

    /// Finding number `number` of app `slug` (the sequence comes from the database).
    pub fn new(slug: &Slug, number: u32) -> Self {
        Self {
            text: format!("{}-SEC-{number:04}", slug.upper()),
            slug: slug.clone(),
            number,
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Slug of the app the finding belongs to.
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// Sequential number within the app.
    pub fn number(&self) -> u32 {
        self.number
    }
}

impl FromStr for FindingId {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parsed = text.rsplit_once("-SEC-").and_then(|(slug, digits)| {
            let number = u32::try_from(parse_padded(digits, 4)?).ok()?;
            Some(Self {
                text: text.to_owned(),
                slug: Slug::from_upper(slug)?,
                number,
            })
        });
        parsed.ok_or_else(|| invalid("FindingId", text, Self::FORMAT))
    }
}

impl_text!(FindingId);

// ---------------------------------------------------------------------------------------
// Audit children: `<AUDIT>-H-<NNN>` and `<AUDIT>-EV-<NNN>`
// ---------------------------------------------------------------------------------------

/// Declares an identifier numbered within an audit (`<AUDIT><marker><NNN>`).
macro_rules! audit_child_id {
    ($(#[$meta:meta])* $name:ident, $marker:literal, $format:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            text: String,
            audit: AuditId,
            number: u32,
        }

        impl $name {
            const FORMAT: &'static str = $format;

            /// Item number `number` of `audit` (numbering is per audit).
            pub fn new(audit: &AuditId, number: u32) -> Self {
                Self { text: format!("{audit}{}{number:03}", $marker), audit: audit.clone(), number }
            }

            /// The identifier text.
            pub fn as_str(&self) -> &str {
                &self.text
            }

            /// The audit this item belongs to.
            pub fn audit(&self) -> &AuditId {
                &self.audit
            }

            /// Sequential number within the audit.
            pub fn number(&self) -> u32 {
                self.number
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let parsed = text.rsplit_once($marker).and_then(|(audit, digits)| {
                    let number = u32::try_from(parse_padded(digits, 3)?).ok()?;
                    Some(Self { text: text.to_owned(), audit: audit.parse().ok()?, number })
                });
                parsed.ok_or_else(|| invalid(stringify!($name), text, Self::FORMAT))
            }
        }

        impl_text!($name);
    };
}

audit_child_id!(
    /// Hypothesis identifier: `<AUDIT>-H-<NNN>`.
    HypothesisId,
    "-H-",
    "<AUDIT>-H-<NNN>"
);

audit_child_id!(
    /// Evidence identifier: `<AUDIT>-EV-<NNN>`.
    EvidenceId,
    "-EV-",
    "<AUDIT>-EV-<NNN>"
);

// ---------------------------------------------------------------------------------------
// TaskId: `<FINDING>-T<N>`
// ---------------------------------------------------------------------------------------

/// Fix task identifier: `<FINDING>-T<N>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    text: String,
    finding: FindingId,
    number: u32,
}

impl TaskId {
    const FORMAT: &'static str = "<FINDING>-T<N>";

    /// Task number `number` of `finding`.
    pub fn new(finding: &FindingId, number: u32) -> Self {
        Self {
            text: format!("{finding}-T{number}"),
            finding: finding.clone(),
            number,
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The finding this task fixes.
    pub fn finding(&self) -> &FindingId {
        &self.finding
    }

    /// Sequential number within the finding.
    pub fn number(&self) -> u32 {
        self.number
    }
}

impl FromStr for TaskId {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parsed = text.rsplit_once("-T").and_then(|(finding, digits)| {
            let number = u32::try_from(parse_padded(digits, 1)?).ok()?;
            Some(Self {
                text: text.to_owned(),
                finding: finding.parse().ok()?,
                number,
            })
        });
        parsed.ok_or_else(|| invalid("TaskId", text, Self::FORMAT))
    }
}

impl_text!(TaskId);

// ---------------------------------------------------------------------------------------
// EventId: `EVT-<NNNNNNNN>`
// ---------------------------------------------------------------------------------------

/// Event identifier: `EVT-<NNNNNNNN>`, global sequence assigned by the database.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId {
    text: String,
    number: u64,
}

impl EventId {
    const FORMAT: &'static str = "EVT-<NNNNNNNN>";

    /// Event number `number` (the event table's row id).
    pub fn new(number: u64) -> Self {
        Self {
            text: format!("EVT-{number:08}"),
            number,
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Global sequential number.
    pub fn number(&self) -> u64 {
        self.number
    }
}

impl FromStr for EventId {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix("EVT-")
            .and_then(|digits| parse_padded(digits, 8))
            .map(Self::new)
            .ok_or_else(|| invalid("EventId", text, Self::FORMAT))
    }
}

impl_text!(EventId);

// ---------------------------------------------------------------------------------------
// CommitHash: full git commit id (R02)
// ---------------------------------------------------------------------------------------

/// Full commit hash: 40 (SHA-1) or 64 (SHA-256) lowercase hex characters. Audits always
/// reference one — never a branch name (R02).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitHash(String);

impl CommitHash {
    const FORMAT: &'static str = "40 or 64 lowercase hex characters";

    /// The full hash.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// First 12 characters, for display only.
    pub fn short(&self) -> &str {
        &self.0[..12]
    }
}

impl FromStr for CommitHash {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let valid = matches!(text.len(), 40 | 64)
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(invalid("CommitHash", text, Self::FORMAT))
        }
    }
}

impl_text!(CommitHash);

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(text: &str) -> Slug {
        Slug::new(text).unwrap()
    }

    fn at(iso: &str) -> Timestamp {
        iso.parse().unwrap()
    }

    #[test]
    fn slug_rules() {
        for ok in ["abastevo", "my-app", "app2", "a1-b2-c3"] {
            assert!(Slug::new(ok).is_ok(), "{ok} should be valid");
        }
        for bad in [
            "", "a", "Abastevo", "1app", "-app", "app-", "my--app", "my_app", "ção",
        ] {
            assert!(Slug::new(bad).is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn audit_id_follows_spec_b2() {
        let id = AuditId::new(&slug("abastevo"), &at("2026-11-29T14:32:00-03:00"), 1);
        assert_eq!(id.as_str(), "AUD-ABASTEVO-20261129-1432");
        assert_eq!(id.slug().as_str(), "abastevo");
    }

    #[test]
    fn audit_id_collision_suffix() {
        let id = AuditId::new(&slug("abastevo"), &at("2026-11-29T14:32:00-03:00"), 2);
        assert_eq!(id.as_str(), "AUD-ABASTEVO-20261129-1432-2");
        assert_eq!(id.as_str().parse::<AuditId>().unwrap(), id);
    }

    #[test]
    fn audit_id_uses_default_time_zone() {
        let id = AuditId::new(&slug("abastevo"), &at("2026-11-29T17:32:00Z"), 1);
        assert_eq!(id.as_str(), "AUD-ABASTEVO-20261129-1432");
    }

    #[test]
    fn audit_id_parses_slugs_with_hyphens_and_digits() {
        for text in ["AUD-MY-APP-20261129-1432", "AUD-A-12345678-20261129-1432-3"] {
            let id: AuditId = text.parse().unwrap();
            assert_eq!(id.as_str(), text);
        }
        let id: AuditId = "AUD-A-12345678-20261129-1432".parse().unwrap();
        assert_eq!(id.slug().as_str(), "a-12345678");
    }

    #[test]
    fn audit_id_rejects_malformed() {
        for bad in [
            "AUD-ABASTEVO-20261332-1432",
            "AUD-ABASTEVO-20261129-2460",
            "AUD-abastevo-20261129-1432",
            "AUD-20261129-1432",
            "ABASTEVO-20261129-1432",
            "AUD-ABASTEVO-20261129-1432-1",
            "AUD-ABASTEVO-20261129-1432-02",
        ] {
            assert!(bad.parse::<AuditId>().is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn map_id_shares_the_audit_stamp() {
        let audit: AuditId = "AUD-ABASTEVO-20261129-1432-2".parse().unwrap();
        let map = MapId::for_audit(&audit);
        assert_eq!(map.as_str(), "MAP-ABASTEVO-20261129-1432-2");
        assert_eq!(map.as_str().parse::<MapId>().unwrap(), map);
    }

    #[test]
    fn finding_id_follows_spec_b2() {
        let id = FindingId::new(&slug("abastevo"), 4);
        assert_eq!(id.as_str(), "ABASTEVO-SEC-0004");
        assert_eq!(id.as_str().parse::<FindingId>().unwrap(), id);
        assert_eq!(
            FindingId::new(&slug("abastevo"), 12345).as_str(),
            "ABASTEVO-SEC-12345"
        );
        assert!("ABASTEVO-SEC-04".parse::<FindingId>().is_err());
        assert!("abastevo-SEC-0004".parse::<FindingId>().is_err());
    }

    #[test]
    fn hypothesis_and_evidence_ids() {
        let audit: AuditId = "AUD-ABASTEVO-20261129-1432".parse().unwrap();
        let hypothesis = HypothesisId::new(&audit, 1);
        let evidence = EvidenceId::new(&audit, 1);
        assert_eq!(hypothesis.as_str(), "AUD-ABASTEVO-20261129-1432-H-001");
        assert_eq!(evidence.as_str(), "AUD-ABASTEVO-20261129-1432-EV-001");
        assert_eq!(
            hypothesis.as_str().parse::<HypothesisId>().unwrap().audit(),
            &audit
        );
        assert_eq!(evidence.as_str().parse::<EvidenceId>().unwrap().number(), 1);
        assert!(
            "AUD-ABASTEVO-20261129-1432-H-1"
                .parse::<HypothesisId>()
                .is_err()
        );
    }

    #[test]
    fn task_id_follows_spec_b2() {
        let finding = FindingId::new(&slug("app-t1"), 4);
        let task = TaskId::new(&finding, 2);
        assert_eq!(task.as_str(), "APP-T1-SEC-0004-T2");
        let parsed: TaskId = task.as_str().parse().unwrap();
        assert_eq!(parsed.finding(), &finding);
        assert_eq!(parsed.number(), 2);
    }

    #[test]
    fn event_id_follows_spec_b2() {
        let id = EventId::new(184);
        assert_eq!(id.as_str(), "EVT-00000184");
        assert_eq!("EVT-00000184".parse::<EventId>().unwrap().number(), 184);
        assert!("EVT-184".parse::<EventId>().is_err());
    }

    #[test]
    fn r02_commit_hash_is_full_and_lowercase() {
        let sha1 = "4c4a5df92ace8d8068e57362a7308e8b9d031b3a";
        assert_eq!(sha1.parse::<CommitHash>().unwrap().short(), "4c4a5df92ace");
        assert!(
            format!("{sha1}{}", "0".repeat(24))
                .parse::<CommitHash>()
                .is_ok()
        );
        for bad in ["main", "4c4a5df", &sha1.to_uppercase(), &format!("{sha1}0")] {
            assert!(
                bad.parse::<CommitHash>().is_err(),
                "{bad} should be invalid"
            );
        }
    }

    #[test]
    fn ids_serialize_as_strings() {
        let id = FindingId::new(&slug("abastevo"), 4);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"ABASTEVO-SEC-0004\"");
        assert_eq!(serde_json::from_str::<FindingId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<FindingId>("\"nope\"").is_err());
    }
}
