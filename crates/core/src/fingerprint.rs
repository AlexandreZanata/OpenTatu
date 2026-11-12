//! Finding identity across audits (spec §B.8; rules R08, R09).
//!
//! ```text
//! fingerprint = "sha256:" + hex(sha256(cwe + "|" + file + "|" + symbol + "|" + normalized_snippet))
//! ```
//!
//! Normalization is deterministic and heuristic (not a parser): it removes comments
//! according to the file extension (respecting string literals) and collapses all
//! whitespace, including line breaks, into single spaces.
//!
//! WARNING: changing anything here changes every fingerprint, which breaks deduplication,
//! false-positive memory (R08) and regression detection (R09). The golden test
//! `golden_fingerprint_is_stable` exists to catch that; update it only with an ADR and a
//! migration plan for stored fingerprints.

use std::str::FromStr;

use sha2::{Digest, Sha256};

use crate::Error;
use crate::macros::impl_text;

const PREFIX: &str = "sha256:";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Input of [`Fingerprint::compute`].
#[derive(Debug, Clone, Copy)]
pub struct FingerprintInput<'a> {
    /// CWE identifier, e.g. `CWE-639`.
    pub cwe: &'a str,
    /// Path of the file relative to the repository root.
    pub file: &'a str,
    /// Function, route or class the finding is attached to.
    pub symbol: &'a str,
    /// Relevant code snippet (normalized before hashing).
    pub snippet: &'a str,
}

/// Stable identity of a finding: `sha256:<64 lowercase hex>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Computes the fingerprint of a finding (spec §B.8).
    pub fn compute(input: &FingerprintInput<'_>) -> Self {
        let material = format!(
            "{}|{}|{}|{}",
            input.cwe.trim(),
            normalize_path(input.file),
            input.symbol.trim(),
            normalize_snippet(input.snippet, input.file)
        );
        let digest = Sha256::digest(material.as_bytes());
        let mut text = String::with_capacity(PREFIX.len() + 64);
        text.push_str(PREFIX);
        for byte in digest.iter() {
            text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        }
        Self(text)
    }

    /// The fingerprint text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Fingerprint {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let valid = text.strip_prefix(PREFIX).is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(Error::InvalidFingerprint(text.to_owned()))
        }
    }
}

impl_text!(Fingerprint);

/// Normalizes a path: trims, uses `/` separators and drops a leading `./`.
fn normalize_path(path: &str) -> String {
    let path = path.trim().replace('\\', "/");
    path.strip_prefix("./").map(str::to_owned).unwrap_or(path)
}

/// Comment syntax of a language family.
struct Syntax {
    line: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
    quotes: &'static [char],
}

const NO_COMMENTS: Syntax = Syntax {
    line: &[],
    block: None,
    quotes: &[],
};

/// Picks the comment syntax from the file extension.
fn syntax_for(file: &str) -> Syntax {
    let extension = file
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("rs") => Syntax {
            line: &["//"],
            block: Some(("/*", "*/")),
            quotes: &['"'],
        },
        Some(
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "java" | "kt" | "kts" | "go" | "c" | "h"
            | "cc" | "cpp" | "hpp" | "cs" | "swift" | "scala" | "dart" | "php",
        ) => Syntax {
            line: &["//"],
            block: Some(("/*", "*/")),
            quotes: &['"', '\'', '`'],
        },
        Some("py" | "rb" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml" | "pl" | "r" | "tf") => {
            Syntax {
                line: &["#"],
                block: None,
                quotes: &['"', '\''],
            }
        }
        Some("sql") => Syntax {
            line: &["--"],
            block: Some(("/*", "*/")),
            quotes: &['\''],
        },
        Some("css" | "scss" | "less") => Syntax {
            line: &[],
            block: Some(("/*", "*/")),
            quotes: &['"', '\''],
        },
        Some("html" | "htm" | "xml" | "vue" | "svelte") => Syntax {
            line: &[],
            block: Some(("<!--", "-->")),
            quotes: &[],
        },
        _ => NO_COMMENTS,
    }
}

/// Normalizes a snippet for fingerprinting: strips comments (by file extension) and
/// collapses whitespace, so formatting and comment edits keep the same fingerprint.
pub fn normalize_snippet(snippet: &str, file: &str) -> String {
    strip_comments(snippet, &syntax_for(file))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_comments(text: &str, syntax: &Syntax) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(current) = rest.chars().next() {
        if syntax.quotes.contains(&current) {
            let end = string_literal_end(rest, current);
            output.push_str(&rest[..end]);
            rest = &rest[end..];
        } else if syntax.line.iter().any(|marker| rest.starts_with(marker)) {
            rest = &rest[rest.find('\n').unwrap_or(rest.len())..];
            output.push(' ');
        } else if let Some((open, close)) = syntax.block.filter(|(open, _)| rest.starts_with(open))
        {
            let after_open = &rest[open.len()..];
            let end = after_open
                .find(close)
                .map_or(after_open.len(), |i| i + close.len());
            rest = &after_open[end..];
            output.push(' ');
        } else {
            output.push(current);
            rest = &rest[current.len_utf8()..];
        }
    }
    output
}

/// Byte index right after the closing `quote` (handles `\` escapes). `text` starts at the
/// opening quote. Unterminated strings run to the end of the text.
fn string_literal_end(text: &str, quote: char) -> usize {
    let mut escaped = false;
    for (index, current) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if current == '\\' {
            escaped = true;
        } else if current == quote {
            return index + current.len_utf8();
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(file: &str, snippet: &str) -> Fingerprint {
        Fingerprint::compute(&FingerprintInput {
            cwe: "CWE-639",
            file,
            symbol: "GET",
            snippet,
        })
    }

    #[test]
    fn golden_fingerprint_is_stable() {
        // sha256("CWE-639|src/app/api/orders/[id]/route.ts|GET|const order = await db.find(id);")
        let value = fingerprint(
            "src/app/api/orders/[id]/route.ts",
            "const order = await db.find(id);",
        );
        assert_eq!(
            value.as_str(),
            "sha256:4c4a5df92ace8d8068e57362a7308e8b9d031b3ab85a91f460a5e39a962b79fe"
        );
    }

    #[test]
    fn whitespace_and_comments_do_not_change_identity() {
        let a = fingerprint("route.ts", "const order = await db.find(id);");
        let b = fingerprint(
            "route.ts",
            "// load the order\nconst   order =\n   await db.find(id); /* no owner check */",
        );
        assert_eq!(a, b);
    }

    #[test]
    fn code_changes_change_identity() {
        let a = fingerprint("route.ts", "const order = await db.find(id);");
        let b = fingerprint("route.ts", "const order = await db.findOwned(id, user);");
        assert_ne!(a, b);
    }

    #[test]
    fn comment_markers_inside_strings_are_kept() {
        assert_eq!(
            normalize_snippet("fetch(\"https://x.test\") // call", "app.js"),
            "fetch(\"https://x.test\")"
        );
        assert_eq!(
            normalize_snippet("s.split('#')  # split", "tool.py"),
            "s.split('#')"
        );
    }

    #[test]
    fn path_is_normalized() {
        assert_eq!(
            fingerprint("./src\\a.ts", "x"),
            fingerprint("src/a.ts", "x")
        );
    }

    #[test]
    fn unknown_extension_only_collapses_whitespace() {
        assert_eq!(normalize_snippet("a  // b\n c", "notes.txt"), "a // b c");
    }

    #[test]
    fn parse_validates_format() {
        let value = fingerprint("a.rs", "x");
        assert_eq!(value.as_str().parse::<Fingerprint>().unwrap(), value);
        assert!("sha256:ABC".parse::<Fingerprint>().is_err());
        assert!("md5:00".parse::<Fingerprint>().is_err());
    }
}
