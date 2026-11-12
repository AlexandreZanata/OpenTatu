//! Deserializers for values written by models, which vary in harmless ways (`"High"` for
//! `HIGH`, `12` for `"12"`). Only spelling is relaxed: an unknown value is still an error.

use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

/// A `text_enum!` value in any letter case (`"high"`, `"High"`, `"HIGH"`).
pub fn any_case<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    let text = String::deserialize(deserializer)?;
    let trimmed = text.trim();
    trimmed
        .parse()
        .or_else(|_| trimmed.to_ascii_uppercase().parse())
        .or_else(|_| trimmed.to_ascii_lowercase().parse())
        .map_err(serde::de::Error::custom)
}

/// A text that models sometimes write as `null` or as a number: `null` → `""`,
/// `12` → `"12"`.
pub fn text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::String(text)) => text,
            Some(serde_json::Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::Severity;

    #[derive(Deserialize)]
    struct Probe {
        #[serde(deserialize_with = "super::any_case")]
        severity: Severity,
    }

    #[derive(Deserialize)]
    struct Text {
        #[serde(default, deserialize_with = "super::text")]
        value: String,
    }

    #[test]
    fn null_and_numbers_become_text() {
        let parse = |v: serde_json::Value| serde_json::from_value::<Text>(v).unwrap().value;
        assert_eq!(parse(serde_json::json!({ "value": null })), "");
        assert_eq!(parse(serde_json::json!({ "value": 12 })), "12");
        assert_eq!(parse(serde_json::json!({})), "");
        assert_eq!(parse(serde_json::json!({ "value": "x" })), "x");
    }

    #[test]
    fn spelling_is_relaxed_values_are_not() {
        for text in ["HIGH", "High", " high "] {
            let probe: Probe =
                serde_json::from_value(serde_json::json!({ "severity": text })).unwrap();
            assert_eq!(probe.severity, Severity::High);
        }
        assert!(
            serde_json::from_value::<Probe>(serde_json::json!({ "severity": "SEVERE" })).is_err()
        );
    }
}
