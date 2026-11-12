//! The app's own vulnerability annotations, e.g. OWASP Juice Shop's
//! `// vuln-code-snippet vuln-line loginAdminChallenge`.
//!
//! A marker is the marker word right after a comment introducer (`//`, `#`, `/*`, `<!--`),
//! followed by a kind (`vuln-line`, `start`, `end`, `neutral-line`, ...) and the keys it
//! applies to. The same word inside code (a string or a regular expression) is not a
//! marker. Stripping removes the comment and keeps the line, so line numbers stay valid.

/// Comment introducers recognized before the marker word, longest first.
const INTRODUCERS: &[&str] = &["<!--", "//", "/*", "#"];
/// Tokens that close a comment and end the key list.
const CLOSERS: &[&str] = &["*/", "-->"];

/// One marker comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// 1-based line.
    pub line: u32,
    /// Kind (`vuln-line`, `start`, ...).
    pub kind: String,
    /// Keys the marker applies to.
    pub keys: Vec<String>,
}

/// Byte index of the comment introducer of a marker in `line`, and the introducer.
fn marker_start(line: &str, marker: &str) -> Option<(usize, &'static str)> {
    let at = line.find(marker)?;
    let before = line[..at].trim_end();
    INTRODUCERS
        .iter()
        .find(|introducer| before.ends_with(**introducer))
        .map(|introducer| (before.len() - introducer.len(), *introducer))
}

/// `line` without its marker comment; code after a closed `/* … */` or `<!-- … -->`
/// comment is kept.
fn without_marker(line: &str, start: usize, introducer: &str) -> String {
    let closer = match introducer {
        "/*" => Some("*/"),
        "<!--" => Some("-->"),
        _ => None,
    };
    let rest = closer
        .and_then(|closer| {
            line[start..]
                .find(closer)
                .map(|at| &line[start + at + closer.len()..])
        })
        .unwrap_or("");
    if rest.trim().is_empty() {
        line[..start].trim_end().to_owned()
    } else {
        format!("{}{}", &line[..start], rest.trim_start())
    }
}

/// Every marker comment of `content`.
pub fn find_markers(content: &str, marker: &str) -> Vec<Marker> {
    let mut found = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if marker_start(line, marker).is_none() {
            continue;
        }
        let Some(at) = line.find(marker) else {
            continue;
        };
        let mut tokens = line[at + marker.len()..]
            .split_whitespace()
            .take_while(|token| !CLOSERS.contains(token));
        let Some(kind) = tokens.next() else {
            continue;
        };
        found.push(Marker {
            line: u32::try_from(index + 1).unwrap_or(u32::MAX),
            kind: kind.to_owned(),
            keys: tokens.map(str::to_owned).collect(),
        });
    }
    found
}

/// `content` without its marker comments (each line kept, trailing spaces trimmed), or
/// `None` when it has none.
pub fn strip_markers(content: &str, marker: &str) -> Option<String> {
    let mut changed = false;
    let mut output = String::with_capacity(content.len());
    for piece in content.split_inclusive('\n') {
        let (line, newline) = match piece.strip_suffix('\n') {
            Some(line) => match line.strip_suffix('\r') {
                Some(line) => (line, "\r\n"),
                None => (line, "\n"),
            },
            None => (piece, ""),
        };
        match marker_start(line, marker) {
            Some((start, introducer)) => {
                changed = true;
                output.push_str(&without_marker(line, start, introducer));
            }
            None => output.push_str(line),
        }
        output.push_str(newline);
    }
    changed.then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKER: &str = "vuln-code-snippet";

    const SOURCE: &str = "\
// vuln-code-snippet start loginAdminChallenge loginJimChallenge
export function login () {
  query(`SELECT * FROM Users WHERE email = '${email}'`) // vuln-code-snippet vuln-line loginAdminChallenge loginJimChallenge
  const re = new RegExp(`vuln-code-snippet vuln-line.*${key}`)
  question: 'Your favorite movie?' # vuln-code-snippet vuln-line resetPasswordUvoginChallenge
  /* vuln-code-snippet neutral-line x */ keep()
}
// vuln-code-snippet end loginAdminChallenge loginJimChallenge
";

    #[test]
    fn markers_are_comments_with_a_kind_and_keys() {
        let markers = find_markers(SOURCE, MARKER);
        let summary: Vec<(u32, &str, usize)> = markers
            .iter()
            .map(|m| (m.line, m.kind.as_str(), m.keys.len()))
            .collect();
        assert_eq!(
            summary,
            [
                (1, "start", 2),
                (3, "vuln-line", 2),
                (5, "vuln-line", 1),
                (6, "neutral-line", 1),
                (8, "end", 2)
            ],
            "the RegExp string on line 4 is code, not a marker"
        );
        assert_eq!(
            markers[1].keys,
            ["loginAdminChallenge", "loginJimChallenge"]
        );
        assert_eq!(markers[3].keys, ["x"], "`*/` ends the keys");
    }

    #[test]
    fn stripping_keeps_code_and_line_numbers() {
        let stripped = strip_markers(SOURCE, MARKER).unwrap();
        assert_eq!(stripped.lines().count(), SOURCE.lines().count());
        assert!(!stripped.contains("vuln-line loginAdminChallenge"));
        assert!(!stripped.contains("resetPasswordUvoginChallenge"));
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines[0], "");
        assert_eq!(
            lines[2],
            "  query(`SELECT * FROM Users WHERE email = '${email}'`)"
        );
        assert!(
            lines[3].contains("new RegExp(`vuln-code-snippet"),
            "code is untouched"
        );
        assert_eq!(lines[4], "  question: 'Your favorite movie?'");
        assert_eq!(lines[5], "  keep()", "code after a closed comment stays");
        assert!(stripped.ends_with('\n'));
        assert_eq!(strip_markers("no markers\n", MARKER), None);
        assert_eq!(
            strip_markers("a // vuln-code-snippet vuln-line k\r\nb\r\n", MARKER).unwrap(),
            "a\r\nb\r\n"
        );
    }
}
