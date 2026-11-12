//! Code of one or more entry points (spec §C.7 step 6): the handler files plus the files
//! they reach — local imports and same-package files — up to [`DEPTH`] hops, ranked by
//! whether they define the functions the handlers call. Heuristic and language-aware for
//! Go, Rust, JavaScript/TypeScript and Python; never a full parser.

use std::collections::{BTreeMap, BTreeSet};

use crate::files::{FileKind, SnapshotFiles};
use crate::selection::{Excerpt, Selection, is_low_value, render_file};

/// Hops from the handler files.
const DEPTH: usize = 2;
/// Related files explored per hop (the best ranked).
const FRONTIER: usize = 12;

/// A handler to start from: its file and, when known, its first line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    /// Repository-relative path.
    pub path: String,
    /// First line of the handler (1-based), when known.
    pub line: Option<u32>,
}

/// The seeds' files (always first) and the related files that fit in `budget` bytes, each
/// at most `per_file` bytes, line-numbered.
pub fn entry_point_code(
    files: &SnapshotFiles,
    seeds: &[Seed],
    budget: usize,
    per_file: usize,
) -> Selection {
    let modules = go_modules(files);
    let mut focus: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for seed in seeds {
        let lines = focus.entry(seed.path.clone()).or_default();
        if let Some(line) = seed.line {
            lines.insert(line.saturating_sub(1) as usize);
        }
    }
    let seed_paths: Vec<String> = focus.keys().cloned().collect();
    let mut scores: BTreeMap<String, u32> = BTreeMap::new();
    let mut visited: BTreeSet<String> = seed_paths.iter().cloned().collect();
    let mut frontier = seed_paths.clone();
    for depth in 0..DEPTH {
        let weight = if depth == 0 { 2 } else { 1 };
        let mut found: BTreeMap<String, u32> = BTreeMap::new();
        for path in &frontier {
            let Some(text) = files.read_text(path) else {
                continue;
            };
            let calls = called_identifiers(&text.content);
            for candidate in neighbours(files, &modules, path, &text.content) {
                if visited.contains(&candidate) || is_low_value(&candidate) {
                    continue;
                }
                let Some(other) = files.read_text(&candidate) else {
                    continue;
                };
                let defined = definition_lines(&other.content, &calls);
                let score = 30 + 100 * u32::try_from(defined.len().min(5)).unwrap_or(5);
                *found.entry(candidate.clone()).or_default() += score * weight;
                focus.entry(candidate).or_default().extend(defined);
            }
        }
        let mut ranked: Vec<(String, u32)> = found.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        frontier = ranked
            .iter()
            .take(FRONTIER)
            .map(|(p, _)| p.clone())
            .collect();
        for (path, score) in ranked {
            visited.insert(path.clone());
            *scores.entry(path).or_default() += score;
        }
    }

    let mut selection = Selection::default();
    let mut ordered: Vec<(String, u32)> = seed_paths.into_iter().map(|p| (p, u32::MAX)).collect();
    let mut related: Vec<(String, u32)> = scores.into_iter().collect();
    related.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ordered.extend(related);
    for (path, _) in ordered {
        let Some(text) = files.read_text(&path) else {
            continue;
        };
        let lines: Vec<usize> = focus
            .get(&path)
            .map(|f| f.iter().copied().collect())
            .unwrap_or_default();
        let excerpt: Excerpt = render_file(&path, &text.content, per_file, &lines);
        if selection.bytes + excerpt.text.len() <= budget {
            selection.bytes += excerpt.text.len();
            selection.excerpts.push(excerpt);
        } else {
            selection.left_out.push(path);
        }
    }
    selection
}

/// `(module path, directory)` of every `go.mod` in the snapshot.
fn go_modules(files: &SnapshotFiles) -> Vec<(String, String)> {
    files
        .files()
        .iter()
        .filter(|f| f.path == "go.mod" || f.path.ends_with("/go.mod"))
        .filter_map(|f| {
            let text = files.read_text(&f.path)?;
            let module = text
                .content
                .lines()
                .find_map(|l| l.trim().strip_prefix("module "))?;
            let dir = f
                .path
                .strip_suffix("go.mod")
                .unwrap_or("")
                .trim_end_matches('/');
            Some((module.trim().to_owned(), dir.to_owned()))
        })
        .collect()
}

/// Files a source file reaches: same-directory files of the same language and resolved
/// local imports.
fn neighbours(
    files: &SnapshotFiles,
    modules: &[(String, String)],
    path: &str,
    content: &str,
) -> Vec<String> {
    let (dir, _) = path.rsplit_once('/').unwrap_or(("", path));
    let extension = path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    let mut out: BTreeSet<String> = files
        .files()
        .iter()
        .filter(|f| f.kind == FileKind::Source && f.path != path)
        .filter(|f| f.path.rsplit_once('/').map_or("", |(d, _)| d) == dir)
        .filter(|f| f.path.ends_with(&format!(".{extension}")))
        .map(|f| f.path.clone())
        .collect();
    for target in import_targets(content, path, extension, modules) {
        if files.get(&target).is_some() {
            out.insert(target);
        } else {
            // A directory (Go package): every non-test source file in it.
            let prefix = format!("{}/", target.trim_end_matches('/'));
            for f in files.files() {
                let inside = f
                    .path
                    .strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.contains('/'));
                if inside
                    && f.kind == FileKind::Source
                    && f.path.ends_with(&format!(".{extension}"))
                {
                    out.insert(f.path.clone());
                }
            }
        }
    }
    out.into_iter().collect()
}

/// Candidate paths (files or directories) imported by `content`.
fn import_targets(
    content: &str,
    path: &str,
    extension: &str,
    modules: &[(String, String)],
) -> Vec<String> {
    let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
    let mut targets = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        match extension {
            "go" => {
                if let Some(import) = quoted(line) {
                    for (module, module_dir) in modules {
                        if let Some(rest) = import.strip_prefix(module.as_str()) {
                            targets.push(join(module_dir, rest.trim_start_matches('/')));
                        }
                    }
                }
            }
            "rs" => {
                if let Some(rest) = line.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
                    let name = rest.trim();
                    let base = path.strip_suffix(".rs").unwrap_or(path);
                    let parent = if path.ends_with("/mod.rs")
                        || path.ends_with("/lib.rs")
                        || path.ends_with("/main.rs")
                    {
                        dir.to_owned()
                    } else {
                        base.to_owned()
                    };
                    targets.push(join(&parent, &format!("{name}.rs")));
                    targets.push(join(&parent, &format!("{name}/mod.rs")));
                }
                if let Some(rest) = line.strip_prefix("use crate::") {
                    let src = path.find("/src/").map_or("src", |i| &path[..i + 4]);
                    let module_path: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                        .collect();
                    let parts: Vec<&str> =
                        module_path.split("::").filter(|p| !p.is_empty()).collect();
                    for n in (1..=parts.len()).rev() {
                        let module = parts[..n].join("/");
                        targets.push(join(src, &format!("{module}.rs")));
                        targets.push(join(src, &format!("{module}/mod.rs")));
                    }
                }
            }
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "vue" | "svelte" => {
                if (line.starts_with("import") || line.contains("require("))
                    && let Some(spec) = quoted(line).filter(|s| s.starts_with('.'))
                {
                    let base = normalize(&join(dir, spec));
                    for ext in ["", ".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.js"] {
                        targets.push(format!("{base}{ext}"));
                    }
                }
            }
            "py" => {
                if let Some(rest) = line
                    .strip_prefix("from ")
                    .and_then(|r| r.split(" import").next())
                {
                    let module = rest.trim();
                    let relative = module.trim_start_matches('.');
                    let anchor = if module.starts_with('.') {
                        dir.to_owned()
                    } else {
                        String::new()
                    };
                    let module_path = relative.replace('.', "/");
                    targets.push(join(&anchor, &format!("{module_path}.py")));
                    targets.push(join(&anchor, &format!("{module_path}/__init__.py")));
                }
            }
            _ => {}
        }
    }
    targets.into_iter().map(|t| normalize(&t)).collect()
}

/// Identifiers followed by `(` (calls), at least 3 characters, minus keywords.
fn called_identifiers(content: &str) -> BTreeSet<String> {
    const KEYWORDS: &[&str] = &[
        "if", "for", "while", "match", "switch", "return", "func", "fn", "function", "def",
        "catch", "new", "make", "len", "append", "Some", "Ok", "Err", "println", "format", "print",
    ];
    let mut calls = BTreeSet::new();
    let bytes = content.as_bytes();
    let mut start = None;
    for (i, b) in bytes.iter().enumerate() {
        let ident = b.is_ascii_alphanumeric() || *b == b'_';
        match (ident, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if *b == b'(' && i - s >= 3 && !bytes[s].is_ascii_digit() {
                    let name = &content[s..i];
                    if !KEYWORDS.contains(&name) {
                        calls.insert(name.to_owned());
                    }
                }
                start = None;
            }
            _ => {}
        }
    }
    calls
}

/// 0-based lines of `content` that define one of `names`.
fn definition_lines(content: &str, names: &BTreeSet<String>) -> Vec<usize> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let t = line.trim_start();
            let declares = [
                "func ",
                "fn ",
                "pub fn ",
                "pub(crate) fn ",
                "async fn ",
                "pub async fn ",
                "def ",
                "async def ",
                "function ",
                "export function ",
                "export async function ",
                "export const ",
                "const ",
                "class ",
                "export class ",
                "public ",
                "private ",
            ];
            declares.iter().any(|d| t.starts_with(d)) && names.iter().any(|n| defines(t, n))
        })
        .map(|(index, _)| index)
        .take(20)
        .collect()
}

/// Whether a declaration line names `name` (`func (s *S) Name(`, `fn name(`, `name =`).
fn defines(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(i, _)| {
        let before = line[..i]
            .chars()
            .last()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after = line[i + name.len()..].trim_start();
        before
            && (after.starts_with('(')
                || after.starts_with('<')
                || after.starts_with('=')
                || after.starts_with(':'))
    })
}

fn quoted(line: &str) -> Option<&str> {
    let start = line.find(['"', '\''])?;
    let quote = line[start..].chars().next()?;
    let rest = &line[start + 1..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_owned()
    } else {
        format!("{dir}/{rest}")
    }
}

/// Resolves `.` and `..` segments.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(entries: &[(&str, &str)]) -> (tempfile::TempDir, SnapshotFiles) {
        let dir = tempfile::tempdir().unwrap();
        for (path, contents) in entries {
            let file = dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, contents).unwrap();
        }
        let files = SnapshotFiles::scan(dir.path()).unwrap();
        (dir, files)
    }

    fn paths(selection: &Selection) -> Vec<&str> {
        selection.excerpts.iter().map(|e| e.path.as_str()).collect()
    }

    #[test]
    fn go_handler_reaches_its_service_package_through_the_module_import() {
        let (_dir, files) = snapshot(&[
            (
                "backend/go.mod",
                "module example.com/app/backend\n\ngo 1.22\n",
            ),
            (
                "backend/internal/http/orders.go",
                "package http\n\nimport (\n\t\"example.com/app/backend/internal/orders\"\n)\n\nfunc GetOrder(w W, r R) {\n\torders.LoadOrder(r.Context(), id)\n}\n",
            ),
            (
                "backend/internal/orders/service.go",
                "package orders\n\nfunc LoadOrder(ctx Ctx, id string) (Order, error) {\n\treturn repo.FindByID(ctx, id)\n}\n",
            ),
            (
                "backend/internal/orders/repo.go",
                "package orders\n\nfunc FindByID(ctx Ctx, id string) (Order, error) {\n\treturn db.Query(\"SELECT * FROM orders WHERE id = $1\", id)\n}\n",
            ),
            (
                "backend/internal/orders/service_test.go",
                "package orders\n\nfunc TestLoadOrder(t *T) { LoadOrder(nil, \"1\") }\n",
            ),
            (
                "backend/internal/billing/unrelated.go",
                "package billing\n\nfunc Charge() {}\n",
            ),
        ]);
        let seeds = [Seed {
            path: "backend/internal/http/orders.go".into(),
            line: Some(7),
        }];
        let selection = entry_point_code(&files, &seeds, 100_000, 20_000);
        let included = paths(&selection);
        assert_eq!(included[0], "backend/internal/http/orders.go", "seed first");
        assert!(included.contains(&"backend/internal/orders/service.go"));
        assert!(
            included.contains(&"backend/internal/orders/repo.go"),
            "depth 2 via the package"
        );
        assert!(!included.contains(&"backend/internal/orders/service_test.go"));
        assert!(!included.contains(&"backend/internal/billing/unrelated.go"));
    }

    #[test]
    fn rust_ts_and_python_imports_resolve() {
        let (_dir, files) = snapshot(&[
            (
                "server/src/lib.rs",
                "mod auth;\nuse crate::store::sessions;\npub fn handler() { auth::check_token(t); }\n",
            ),
            (
                "server/src/auth.rs",
                "pub fn check_token(t: &str) -> bool { true }\n",
            ),
            ("server/src/store/sessions.rs", "pub fn find() {}\n"),
            (
                "web/src/routes/orders.ts",
                "import { loadOrder } from '../services/orders';\nexport async function GET() { return loadOrder(id); }\n",
            ),
            (
                "web/src/services/orders.ts",
                "export function loadOrder(id: string) { return db.get(id); }\n",
            ),
            (
                "api/views.py",
                "from .services import load_order\n\ndef get(request):\n    return load_order(request.id)\n",
            ),
            (
                "api/services.py",
                "def load_order(order_id):\n    return Order.objects.get(id=order_id)\n",
            ),
        ]);
        let rust = entry_point_code(
            &files,
            &[Seed {
                path: "server/src/lib.rs".into(),
                line: None,
            }],
            100_000,
            20_000,
        );
        assert!(paths(&rust).contains(&"server/src/auth.rs"));
        assert!(paths(&rust).contains(&"server/src/store/sessions.rs"));
        let ts = entry_point_code(
            &files,
            &[Seed {
                path: "web/src/routes/orders.ts".into(),
                line: Some(2),
            }],
            100_000,
            20_000,
        );
        assert!(paths(&ts).contains(&"web/src/services/orders.ts"));
        let py = entry_point_code(
            &files,
            &[Seed {
                path: "api/views.py".into(),
                line: Some(3),
            }],
            100_000,
            20_000,
        );
        assert!(paths(&py).contains(&"api/services.py"));
    }

    #[test]
    fn budget_keeps_seeds_first_and_reports_the_rest() {
        let big = "func Helper() {}\n".repeat(400);
        let (_dir, files) = snapshot(&[
            (
                "svc/handler.go",
                "package svc\n\nfunc Handle() { Helper() }\n",
            ),
            ("svc/helpers.go", big.as_str()),
        ]);
        let selection = entry_point_code(
            &files,
            &[Seed {
                path: "svc/handler.go".into(),
                line: Some(3),
            }],
            200,
            20_000,
        );
        assert_eq!(paths(&selection), ["svc/handler.go"]);
        assert_eq!(selection.left_out, ["svc/helpers.go"]);
    }
}
