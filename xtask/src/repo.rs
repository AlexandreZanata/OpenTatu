//! Repository helpers: root path, file reading and directory walking.

use std::path::{Path, PathBuf};

/// Directories never scanned.
const SKIPPED_DIRS: &[&str] = &[".git", "target", "data", "node_modules"];

/// Repository root (the parent of `xtask/`).
pub fn root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(manifest_dir).to_path_buf()
}

/// Reads a UTF-8 file, describing failures as a problem message.
pub fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("{}: cannot read ({error})", path.display()))
}

/// Reads and parses a TOML file.
pub fn read_toml(path: &Path) -> Result<toml::Table, String> {
    read(path)?
        .parse::<toml::Table>()
        .map_err(|error| format!("{}: invalid TOML ({error})", path.display()))
}

/// Every file under `dir` (recursively) whose extension is in `extensions`.
pub fn files_with_extensions(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(dir, &mut |path| {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e))
        {
            found.push(path.to_path_buf());
        }
    });
    found.sort();
    found
}

fn walk(dir: &Path, visit: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if !SKIPPED_DIRS.iter().any(|skipped| name == *skipped) {
                walk(&path, visit);
            }
        } else {
            visit(&path);
        }
    }
}

/// Path relative to the repository root, for messages.
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
