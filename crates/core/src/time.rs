//! Dates and times (spec §B.3, rule R03).
//!
//! - Storage and interchange: ISO 8601 with offset, e.g. `2026-11-29T14:32:00-03:00`
//!   ([`Timestamp::iso`]).
//! - Owner-facing display (pt-BR): `29/11/2026 às 14:32 (horário de Brasília)`
//!   ([`Timestamp::display_pt_br`]).
//! - Default time zone: [`DEFAULT_TIME_ZONE`] (`America/Sao_Paulo`).
//! - R03: "now" ALWAYS comes from an injected [`Clock`]. Never call `now()` on a time
//!   library directly (clippy blocks it via `clippy.toml`). AI models never produce dates.
//! - Precision is whole seconds, so values round-trip exactly through [`Timestamp::iso`].

use std::fmt;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex, PoisonError};

use jiff::tz::TimeZone;

use crate::Error;

/// IANA name of the default time zone (spec §B.3).
pub const DEFAULT_TIME_ZONE: &str = "America/Sao_Paulo";

/// Owner-facing (pt-BR) label of [`DEFAULT_TIME_ZONE`] (spec §B.3).
const TIME_ZONE_LABEL_PT_BR: &str = "horário de Brasília";

/// Resolved default zone. The tz database is bundled (jiff `tzdb-bundle-always`), so the
/// fallback (fixed UTC-3, Brazil's offset without DST) is purely defensive.
static ZONE: LazyLock<TimeZone> = LazyLock::new(|| {
    TimeZone::get(DEFAULT_TIME_ZONE).unwrap_or_else(|_| TimeZone::fixed(jiff::tz::offset(-3)))
});

/// An instant in time with whole-second precision, shown in the default time zone.
///
/// Serialized as ISO 8601 with offset (the [`Display`](fmt::Display) form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(jiff::Timestamp);

impl Timestamp {
    /// Builds a timestamp from Unix seconds.
    pub fn from_unix_seconds(seconds: i64) -> Result<Self, Error> {
        jiff::Timestamp::from_second(seconds)
            .map(Self)
            .map_err(|_| Error::InvalidTimestamp(seconds.to_string()))
    }

    /// Unix seconds.
    pub fn unix_seconds(&self) -> i64 {
        self.0.as_second()
    }

    /// ISO 8601 with offset in the default zone, e.g. `2026-11-29T14:32:00-03:00`.
    pub fn iso(&self) -> String {
        self.zoned().strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
    }

    /// Owner-facing pt-BR display, e.g. `29/11/2026 às 14:32 (horário de Brasília)`.
    pub fn display_pt_br(&self) -> String {
        let local = self.zoned().strftime("%d/%m/%Y às %H:%M").to_string();
        format!("{local} ({TIME_ZONE_LABEL_PT_BR})")
    }

    /// Seconds elapsed since `earlier` (negative when `earlier` is after `self`).
    pub fn seconds_since(&self, earlier: &Timestamp) -> i64 {
        self.unix_seconds() - earlier.unix_seconds()
    }

    /// Local date in the default zone as `YYYYMMDD` (used by IDs, spec §B.2).
    pub(crate) fn compact_date(&self) -> String {
        self.zoned().strftime("%Y%m%d").to_string()
    }

    /// Local time in the default zone as `HHMM` (used by IDs, spec §B.2).
    pub(crate) fn compact_time(&self) -> String {
        self.zoned().strftime("%H%M").to_string()
    }

    /// Wraps a jiff timestamp, truncating to whole seconds.
    fn from_jiff(timestamp: jiff::Timestamp) -> Self {
        Self(jiff::Timestamp::from_second(timestamp.as_second()).unwrap_or(timestamp))
    }

    fn zoned(&self) -> jiff::Zoned {
        self.0.to_zoned(ZONE.clone())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.iso())
    }
}

impl FromStr for Timestamp {
    type Err = Error;

    /// Parses ISO 8601 / RFC 3339 **with offset** (`-03:00` or `Z`). Fractions are truncated.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse::<jiff::Timestamp>()
            .map(Self::from_jiff)
            .map_err(|_| Error::InvalidTimestamp(text.to_owned()))
    }
}

impl serde::Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.iso())
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <String as serde::Deserialize>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Source of "now" (R03). Inject it everywhere a date is needed; use [`FixedClock`] in tests.
pub trait Clock: Send + Sync + fmt::Debug {
    /// Current instant.
    fn now(&self) -> Timestamp;
}

/// The real system clock. The ONLY place allowed to read the system time.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    // R03: this is the single authorized read of the system clock.
    #[allow(clippy::disallowed_methods)]
    fn now(&self) -> Timestamp {
        Timestamp::from_jiff(jiff::Timestamp::now())
    }
}

/// Deterministic clock for tests and simulations.
#[derive(Debug)]
pub struct FixedClock {
    current: Mutex<Timestamp>,
}

impl FixedClock {
    /// Clock frozen at `at`.
    pub fn new(at: Timestamp) -> Self {
        Self {
            current: Mutex::new(at),
        }
    }

    /// Clock frozen at an ISO 8601 instant, e.g. `FixedClock::at("2026-11-29T14:32:00-03:00")`.
    pub fn at(iso: &str) -> Result<Self, Error> {
        Ok(Self::new(iso.parse()?))
    }

    /// Moves the clock forward (or backward, if negative). Out-of-range moves are ignored.
    pub fn advance_seconds(&self, seconds: i64) {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if let Ok(next) = Timestamp::from_unix_seconds(current.unix_seconds() + seconds) {
            *current = next;
        }
    }

    /// Sets the clock to `at`.
    pub fn set(&self, at: Timestamp) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = at;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = "2026-11-29T14:32:00-03:00";

    #[test]
    fn iso_round_trip_with_offset() {
        let ts: Timestamp = EXAMPLE.parse().unwrap();
        assert_eq!(ts.iso(), EXAMPLE);
        assert_eq!(ts.to_string(), EXAMPLE);
    }

    #[test]
    fn display_pt_br_matches_spec_b3() {
        let ts: Timestamp = EXAMPLE.parse().unwrap();
        assert_eq!(
            ts.display_pt_br(),
            "29/11/2026 às 14:32 (horário de Brasília)"
        );
    }

    #[test]
    fn utc_input_is_shown_in_default_zone() {
        let ts: Timestamp = "2026-11-29T17:32:00Z".parse().unwrap();
        assert_eq!(ts.iso(), EXAMPLE);
    }

    #[test]
    fn rejects_timestamp_without_offset() {
        assert!("2026-11-29T14:32:00".parse::<Timestamp>().is_err());
        assert!("29/11/2026".parse::<Timestamp>().is_err());
    }

    #[test]
    fn truncates_fractional_seconds() {
        let ts: Timestamp = "2026-11-29T14:32:00.999-03:00".parse().unwrap();
        assert_eq!(ts.iso(), EXAMPLE);
    }

    #[test]
    fn compact_parts_used_by_ids() {
        let ts: Timestamp = EXAMPLE.parse().unwrap();
        assert_eq!(ts.compact_date(), "20261129");
        assert_eq!(ts.compact_time(), "1432");
    }

    #[test]
    fn seconds_since_measures_duration() {
        let start: Timestamp = "2026-11-29T14:32:00-03:00".parse().unwrap();
        let end: Timestamp = "2026-11-29T15:47:12-03:00".parse().unwrap();
        assert_eq!(end.seconds_since(&start), 4512);
    }

    #[test]
    fn serde_uses_iso_string() {
        let ts: Timestamp = EXAMPLE.parse().unwrap();
        let json = serde_json::to_string(&ts).unwrap();
        assert_eq!(json, format!("\"{EXAMPLE}\""));
        assert_eq!(serde_json::from_str::<Timestamp>(&json).unwrap(), ts);
    }

    #[test]
    fn fixed_clock_is_deterministic_and_advances() {
        let clock = FixedClock::at(EXAMPLE).unwrap();
        assert_eq!(clock.now().iso(), EXAMPLE);
        clock.advance_seconds(60);
        assert_eq!(clock.now().iso(), "2026-11-29T14:33:00-03:00");
    }

    #[test]
    fn r03_system_clock_has_whole_seconds() {
        let now = SystemClock.now();
        assert_eq!(now.iso().parse::<Timestamp>().unwrap(), now);
    }
}
