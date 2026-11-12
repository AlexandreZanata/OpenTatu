//! `benchmark/expected/<name>.json`: the known flaws of a benchmark app, derived only from
//! its `vuln-line` markers and its challenge catalog. Keys marking exactly the same lines
//! (e.g. three login challenges solved through one SQL injection) are one flaw.

use std::collections::{BTreeMap, BTreeSet};

use opentatu_core::CommitHash;
use serde::{Deserialize, Serialize};

use crate::{BenchmarkApp, Marker};

/// The marker kind that points at a vulnerable line.
const VULN_LINE: &str = "vuln-line";

/// One line of a known flaw.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Location {
    /// Repository-relative path.
    pub file: String,
    /// 1-based line.
    pub line: u32,
}

/// One known flaw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedFlaw {
    /// `EXP-NNN`, by first location.
    pub id: String,
    /// Marker keys (challenge keys) of the flaw.
    pub keys: Vec<String>,
    /// Their names in the catalog.
    pub names: Vec<String>,
    /// Their categories in the catalog (`/`-joined when they differ).
    pub category: String,
    /// The marked lines.
    pub locations: Vec<Location>,
}

/// The known flaws of one benchmark app at its pinned commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expected {
    /// Benchmark name.
    pub benchmark: String,
    /// Release tag.
    pub revision: String,
    /// Upstream commit the markers were read from.
    pub upstream_commit: CommitHash,
    /// How the list was derived.
    pub source: String,
    /// Upstream license.
    pub license: String,
    /// Known flaws, by first location.
    pub flaws: Vec<ExpectedFlaw>,
}

/// One entry of the challenge catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// Display name.
    pub name: String,
    /// Category.
    pub category: String,
}

/// Reads `name`, `category` and `key` of each entry of a YAML list of flat mappings
/// (OWASP Juice Shop's `challenges.yml`). Nested values are ignored.
pub fn parse_challenges(yaml: &str) -> BTreeMap<String, Challenge> {
    let mut catalog = BTreeMap::new();
    let mut entry: BTreeMap<&str, String> = BTreeMap::new();
    let mut flush = |entry: &mut BTreeMap<&str, String>| {
        if let Some(key) = entry.remove("key") {
            catalog.insert(
                key,
                Challenge {
                    name: entry.remove("name").unwrap_or_default(),
                    category: entry.remove("category").unwrap_or_default(),
                },
            );
        }
        entry.clear();
    };
    for line in yaml.lines() {
        if line.starts_with('-') {
            flush(&mut entry);
        }
        let field = line.trim_start_matches('-');
        let indent = field.len() - field.trim_start().len();
        let Some((name, value)) = field.trim_start().split_once(':') else {
            continue;
        };
        let top_level = line.starts_with('-') || indent == 2;
        if top_level && matches!(name, "name" | "category" | "key") {
            let value = value.trim().trim_matches(|c| c == '\'' || c == '"');
            entry.insert(name, value.to_owned());
        }
    }
    flush(&mut entry);
    catalog
}

/// The known flaws of `app` from the markers of every audited file (`(path, markers)`).
pub fn expected_flaws(
    app: &BenchmarkApp,
    markers: &[(String, Vec<Marker>)],
    catalog: &BTreeMap<String, Challenge>,
) -> Expected {
    let mut by_key: BTreeMap<&str, BTreeSet<Location>> = BTreeMap::new();
    for (file, file_markers) in markers {
        for marker in file_markers.iter().filter(|m| m.kind == VULN_LINE) {
            for key in &marker.keys {
                by_key.entry(key).or_default().insert(Location {
                    file: file.clone(),
                    line: marker.line,
                });
            }
        }
    }
    let mut by_lines: BTreeMap<BTreeSet<Location>, Vec<&str>> = BTreeMap::new();
    for (key, lines) in by_key {
        by_lines.entry(lines).or_default().push(key);
    }
    let mut groups: Vec<(Vec<Location>, Vec<&str>)> = by_lines
        .into_iter()
        .map(|(lines, keys)| (lines.into_iter().collect(), keys))
        .collect();
    groups.sort();
    let flaws = groups
        .into_iter()
        .enumerate()
        .map(|(index, (locations, keys))| {
            let challenges: Vec<Option<&Challenge>> =
                keys.iter().map(|key| catalog.get(*key)).collect();
            let mut categories: Vec<&str> = challenges
                .iter()
                .flatten()
                .map(|c| c.category.as_str())
                .filter(|c| !c.is_empty())
                .collect();
            categories.sort_unstable();
            categories.dedup();
            ExpectedFlaw {
                id: format!("EXP-{:03}", index + 1),
                names: challenges
                    .iter()
                    .zip(&keys)
                    .map(|(c, key)| c.map_or_else(|| (*key).to_owned(), |c| c.name.clone()))
                    .collect(),
                keys: keys.iter().map(|k| (*k).to_owned()).collect(),
                category: if categories.is_empty() {
                    "unknown".to_owned()
                } else {
                    categories.join(" / ")
                },
                locations,
            }
        })
        .collect();
    Expected {
        benchmark: app.name.clone(),
        revision: app.revision.clone(),
        upstream_commit: app.commit.clone(),
        source: format!(
            "`{} {VULN_LINE}` markers of the upstream commit; names and categories from {}",
            app.marker, app.challenges_file
        ),
        license: app.license.clone(),
        flaws,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::APP;
    use crate::find_markers;

    const CATALOG: &str = "\
-
  name: 'Login Admin'
  category: 'Injection'
  hints:
    - 'key: not a key'
  key: loginAdminChallenge
-
  name: 'Login Jim'
  category: 'Injection'
  key: loginJimChallenge
- name: \"Forged Review\"
  category: 'Broken Access Control'
  key: forgedReviewChallenge
";

    #[test]
    fn catalog_entries_are_read_without_nested_values() {
        let catalog = parse_challenges(CATALOG);
        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog["loginAdminChallenge"].name, "Login Admin");
        assert_eq!(
            catalog["forgedReviewChallenge"].category,
            "Broken Access Control"
        );
    }

    #[test]
    fn keys_on_the_same_lines_are_one_flaw() {
        let app = BenchmarkApp::from_toml(APP).unwrap();
        let login =
            "x\nquery() // vuln-code-snippet vuln-line loginAdminChallenge loginJimChallenge\n";
        let reviews = "\
// vuln-code-snippet start forgedReviewChallenge
a() // vuln-code-snippet vuln-line forgedReviewChallenge
b() // vuln-code-snippet neutral-line forgedReviewChallenge
c() // vuln-code-snippet vuln-line forgedReviewChallenge unknownChallenge
";
        let markers = vec![
            (
                "routes/login.ts".to_owned(),
                find_markers(login, &app.marker),
            ),
            (
                "routes/reviews.ts".to_owned(),
                find_markers(reviews, &app.marker),
            ),
        ];
        let expected = expected_flaws(&app, &markers, &parse_challenges(CATALOG));
        let flaws = &expected.flaws;
        assert_eq!(flaws.len(), 3);
        assert_eq!(flaws[0].id, "EXP-001");
        assert_eq!(flaws[0].keys, ["loginAdminChallenge", "loginJimChallenge"]);
        assert_eq!(flaws[0].category, "Injection");
        assert_eq!(
            flaws[0].locations,
            [Location {
                file: "routes/login.ts".into(),
                line: 2
            }]
        );
        assert_eq!(flaws[1].keys, ["forgedReviewChallenge"]);
        assert_eq!(flaws[1].locations.len(), 2, "neutral lines are not flaws");
        assert_eq!(flaws[2].keys, ["unknownChallenge"]);
        assert_eq!(flaws[2].category, "unknown");
        assert_eq!(flaws[2].names, ["unknownChallenge"]);
    }
}
