//! The `<structure>` block: the file tree of the commit, bounded in lines.
//!
//! Code and configuration files are listed one per line with their size; documentation and
//! other files are summarized per directory. When the listing is longer than the budget,
//! the directories with the most listed files are summarized first.

use std::collections::BTreeMap;

use crate::files::{FileKind, SnapshotFiles};

#[derive(Default)]
struct Directory<'a> {
    listed: Vec<(&'a str, u64, FileKind)>,
    summarized: BTreeMap<String, usize>,
}

/// Renders the tree in at most `max_lines` lines (plus one header line).
pub fn render_tree(files: &SnapshotFiles, max_lines: usize) -> String {
    let mut directories: BTreeMap<&str, Directory<'_>> = BTreeMap::new();
    for file in files.files() {
        let directory = file.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let entry = directories.entry(directory).or_default();
        match file.kind {
            FileKind::Source | FileKind::Config | FileKind::Secret => {
                entry
                    .listed
                    .push((file.path.as_str(), file.size, file.kind));
            }
            FileKind::Doc | FileKind::Other => {
                *entry.summarized.entry(extension(&file.path)).or_default() += 1;
            }
        }
    }
    let lines = |dirs: &BTreeMap<&str, Directory<'_>>| -> usize {
        dirs.values()
            .map(|d| d.listed.len() + usize::from(!d.summarized.is_empty()))
            .sum()
    };
    while lines(&directories) > max_lines {
        let Some(biggest) = directories
            .iter_mut()
            .filter(|(_, d)| d.listed.len() > 1)
            .max_by_key(|(_, d)| d.listed.len())
            .map(|(_, d)| d)
        else {
            break;
        };
        for (path, _, _) in std::mem::take(&mut biggest.listed) {
            *biggest.summarized.entry(extension(path)).or_default() += 1;
        }
    }

    let total = files.files().len();
    let mut out = format!("{total} files (dependencies, build output and VCS metadata excluded)\n");
    for (directory, entry) in &directories {
        for (path, size, kind) in &entry.listed {
            if *kind == FileKind::Secret {
                out.push_str(&format!("{path} (not read: may contain secrets)\n"));
            } else {
                out.push_str(&format!("{path} ({} KB)\n", size.div_ceil(1024)));
            }
        }
        if !entry.summarized.is_empty() {
            let count: usize = entry.summarized.values().sum();
            let mut kinds: Vec<_> = entry.summarized.iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let kinds: Vec<_> = kinds
                .iter()
                .take(6)
                .map(|(ext, n)| format!("{ext} {n}"))
                .collect();
            let shown = if directory.is_empty() {
                "./"
            } else {
                directory
            };
            out.push_str(&format!(
                "{shown}/ — {count} file(s) not listed ({})\n",
                kinds.join(", ")
            ));
        }
    }
    out.replace("//", "/")
}

fn extension(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => "no extension".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> (tempfile::TempDir, SnapshotFiles) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (path, contents) in [
            ("src/main.rs", "fn main() {}"),
            ("src/routes.rs", "x"),
            ("src/auth.rs", "x"),
            ("assets/a.png", "x"),
            ("assets/b.png", "x"),
            ("README.md", "x"),
            (".env", "SECRET=1"),
            ("Dockerfile", "FROM scratch"),
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, contents).unwrap();
        }
        let files = SnapshotFiles::scan(root).unwrap();
        (dir, files)
    }

    #[test]
    fn lists_code_and_summarizes_the_rest() {
        let (_dir, files) = snapshot();
        let tree = render_tree(&files, 100);
        assert!(tree.starts_with("8 files"), "{tree}");
        assert!(tree.contains("src/routes.rs (1 KB)\n"), "{tree}");
        assert!(tree.contains("Dockerfile (1 KB)\n"), "{tree}");
        assert!(
            tree.contains(".env (not read: may contain secrets)"),
            "{tree}"
        );
        assert!(
            tree.contains("assets/ — 2 file(s) not listed (png 2)"),
            "{tree}"
        );
        assert!(tree.contains("./ — 1 file(s) not listed (md 1)"), "{tree}");
    }

    #[test]
    fn long_listings_are_summarized_biggest_directory_first() {
        let (_dir, files) = snapshot();
        let tree = render_tree(&files, 5);
        assert!(
            tree.contains("src/ — 3 file(s) not listed (rs 3)"),
            "{tree}"
        );
        assert!(tree.contains("Dockerfile (1 KB)"), "{tree}");
    }
}
