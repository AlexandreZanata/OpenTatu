//! Vocabulary guardrail: the pt-BR terms of the original spec must not appear in code
//! identifiers, comments, configuration or documentation. Everything is English
//! (docs/DECISIONS.md, ADR-0001); the mapping lives in docs/GLOSSARY.md.
//!
//! Source of the forbidden words: the backticked words in the "Spec term (pt-BR)" column
//! of the table under `## Terms` in docs/GLOSSARY.md.
//!
//! Not scanned on purpose: string literals (owner-facing pt-BR text), HTML templates
//! (pt-BR UI), prompt files (they contain pt-BR output headings), the glossary itself and
//! the archived original spec.

use std::collections::BTreeMap;
use std::path::Path;

use crate::repo::{files_with_extensions, read, relative};

const GLOSSARY: &str = "docs/GLOSSARY.md";

/// Paths (relative, `/`-separated prefixes) never scanned.
const EXCLUDED: &[&str] = &[GLOSSARY, "docs/spec/original/"];

/// Prompt files are runtime content; only their agent docs are scanned.
const PROMPT_DOCS: &[&str] = &["AGENTS.md", "CLAUDE.md", "README.md"];

/// Every glossary problem found (empty = ok).
pub fn check(root: &Path) -> Vec<String> {
    let terms = match forbidden_terms(&read(&root.join(GLOSSARY)).unwrap_or_default()) {
        Ok(terms) => terms,
        Err(problem) => return vec![problem],
    };
    let mut problems = Vec::new();
    for file in files_with_extensions(root, &["rs", "md", "sql", "toml", "css"]) {
        let path = relative(root, &file).replace('\\', "/");
        if is_excluded(&path) {
            continue;
        }
        let Ok(text) = read(&file) else { continue };
        let extension = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        for (index, line) in scannable(&text, extension).lines().enumerate() {
            for word in words(line) {
                if let Some(english) = terms.get(&word) {
                    problems.push(format!(
                        "{path}:{}: spec (pt-BR) term `{word}` — use `{english}` (see {GLOSSARY})",
                        index + 1
                    ));
                }
            }
        }
    }
    problems
}

fn is_excluded(path: &str) -> bool {
    if EXCLUDED.iter().any(|prefix| path.starts_with(prefix)) {
        return true;
    }
    match path.strip_prefix("prompts/") {
        Some(name) => !PROMPT_DOCS.contains(&name),
        None => false,
    }
}

/// Parses `forbidden word → English term` from the glossary table.
fn forbidden_terms(glossary: &str) -> Result<BTreeMap<String, String>, String> {
    let mut terms = BTreeMap::new();
    let table = glossary.split("## Terms").nth(1).unwrap_or_default();
    for line in table.lines().filter(|l| l.trim_start().starts_with('|')) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // ["", term, spec terms, ...]
        let (Some(term), Some(spec)) = (cells.get(1), cells.get(2)) else {
            continue;
        };
        let english = term.trim_matches(|c| c == '*' || c == '`');
        for word in spec.split('`').skip(1).step_by(2) {
            terms.insert(word.to_ascii_lowercase(), english.to_owned());
        }
    }
    if terms.is_empty() {
        return Err(format!(
            "{GLOSSARY}: no forbidden terms parsed from the `## Terms` table"
        ));
    }
    Ok(terms)
}

/// Blanks string literals of code-like files (keeping line breaks, so line numbers hold)
/// so owner-facing pt-BR text is allowed there. Strings may span lines.
fn scannable(text: &str, extension: &str) -> String {
    let quote = match extension {
        "rs" | "toml" => '"',
        "sql" => '\'',
        _ => return text.to_owned(),
    };
    // A Rust char literal holding a double quote would otherwise open a "string".
    let text = if extension == "rs" {
        text.replace("'\"'", "' '").replace("'\\\"'", "' '")
    } else {
        text.to_owned()
    };
    let mut output = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for current in text.chars() {
        if current == '\n' {
            output.push('\n');
            escaped = false;
        } else if in_string {
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if current == quote {
                in_string = false;
            }
        } else if current == quote {
            in_string = true;
            output.push(' ');
        } else {
            output.push(current);
        }
    }
    output
}

/// Lowercase ASCII words: accents folded, split on non-alphanumerics and camelCase.
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for raw in text.chars() {
        let c = fold_accent(raw);
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
            current.push(c.to_ascii_lowercase());
        } else {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous_lower = false;
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_identifiers_and_folds_accents() {
        assert_eq!(
            words("FindingId audit_kind"),
            ["finding", "id", "audit", "kind"]
        );
        assert_eq!(
            words("Relatório de Auditoria"),
            ["relatorio", "de", "auditoria"]
        );
    }

    #[test]
    fn string_literals_are_not_scanned_in_code() {
        assert_eq!(
            words(&scannable("let x = \"Aplicações\"; // ok", "rs")),
            ["let", "x", "ok"]
        );
        assert_eq!(
            scannable("same \"text\" in md", "md"),
            "same \"text\" in md"
        );
        let multi_line = "let s = \"first\n second\"; let q = '\"'; tail";
        assert_eq!(
            words(&scannable(multi_line, "rs")),
            ["let", "s", "let", "q", "tail"]
        );
    }

    #[test]
    fn parses_the_terms_table() {
        let glossary = "# G\n## Terms\n| Term | Spec term (pt-BR) | Definition |\n|---|---|---|\n\
                        | **Finding** | Achado (`achado`, `achados`) | x |\n## Other\n";
        let terms = forbidden_terms(glossary).unwrap();
        assert_eq!(terms.get("achados").map(String::as_str), Some("Finding"));
    }

    #[test]
    fn prompt_files_are_excluded_but_their_docs_are_not() {
        assert!(is_excluded("prompts/verifier.md"));
        assert!(!is_excluded("prompts/AGENTS.md"));
        assert!(is_excluded("docs/spec/original/x.txt"));
        assert!(!is_excluded("docs/ARCHITECTURE.md"));
    }
}
