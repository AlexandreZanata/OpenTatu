//! Strict `{{variable}}` rendering for prompts.
//!
//! - A placeholder is `{{name}}` where `name` is `[a-z0-9_]+`. Anything else (`{{ x }}`,
//!   JSON braces) is copied literally.
//! - A placeholder without a value is an error: a prompt never reaches a model with
//!   unfilled variables.
//! - Single pass: a value that itself contains `{{x}}` is inserted verbatim, never expanded.

use std::collections::{BTreeMap, BTreeSet};

use crate::Error;

/// Renders `template`, replacing each `{{name}}` with `values[name]`.
pub fn render(template: &str, values: &BTreeMap<&str, String>) -> Result<String, Error> {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match placeholder_name(after) {
            Some(name) => {
                let value = values
                    .get(name)
                    .ok_or_else(|| Error::MissingVariable(name.to_owned()))?;
                output.push_str(value);
                rest = &after[name.len() + 2..];
            }
            None => {
                output.push_str("{{");
                rest = after;
            }
        }
    }
    output.push_str(rest);
    Ok(output)
}

/// Placeholder names used by `template` (sorted, unique).
pub fn placeholders(template: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start + 2..];
        if let Some(name) = placeholder_name(rest) {
            names.insert(name);
        }
    }
    names
}

/// If `text` starts with `name}}`, returns `name`.
fn placeholder_name(text: &str) -> Option<&str> {
    let end = text.find("}}")?;
    let name = &text[..end];
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    valid.then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&'static str, &str)]) -> BTreeMap<&'static str, String> {
        pairs.iter().map(|(k, v)| (*k, (*v).to_owned())).collect()
    }

    #[test]
    fn fills_placeholders() {
        let out = render(
            "Audit {{audit_id}} of {{app_slug}}",
            &values(&[("audit_id", "AUD-X-20261129-1432"), ("app_slug", "x")]),
        )
        .unwrap();
        assert_eq!(out, "Audit AUD-X-20261129-1432 of x");
    }

    #[test]
    fn missing_value_is_an_error() {
        let error = render("{{commit}}", &values(&[])).unwrap_err();
        assert_eq!(error, Error::MissingVariable("commit".into()));
    }

    #[test]
    fn values_are_not_expanded_again() {
        let out = render("{{a}}", &values(&[("a", "{{b}}")])).unwrap();
        assert_eq!(out, "{{b}}");
    }

    #[test]
    fn non_placeholders_are_literal() {
        let template = "json {{ \"a\": 1 }} and {{Name}} stay";
        assert_eq!(render(template, &values(&[])).unwrap(), template);
        assert!(placeholders(template).is_empty());
    }
}
