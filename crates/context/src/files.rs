//! The files of a snapshot: a safe walk (no symlinks, no vendored or build directories),
//! a coarse classification, and bounded text reads.
//!
//! Files that usually hold real secrets (`.env`, private keys, keystores) are listed in the
//! tree by name only and never read (R15, R16).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opentatu_core::FileIndex;

use crate::Error;

/// Directories never walked (dependencies, build output, caches, VCS metadata).
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    "coverage",
    ".idea",
    ".vscode",
    ".gradle",
    "Pods",
    ".dart_tool",
    "obj",
    ".terraform",
    ".cache",
    ".pytest_cache",
    ".mypy_cache",
    "site-packages",
    "bower_components",
    ".parcel-cache",
];

/// Files above this size are never read (generated code, bundles, data dumps).
const MAX_READ_BYTES: u64 = 1024 * 1024;

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "mjs", "cjs", "ts", "tsx", "jsx", "go", "java", "kt", "kts", "scala", "cs",
    "rb", "php", "swift", "c", "h", "cc", "cpp", "hpp", "m", "mm", "ex", "exs", "erl", "clj",
    "dart", "lua", "sh", "bash", "ps1", "sql", "vue", "svelte", "html", "graphql", "proto",
];

const DOC_EXTENSIONS: &[&str] = &["md", "rst", "txt", "adoc"];

/// Lockfiles: large, and manifests already say what matters to the map.
const LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "poetry.lock",
    "Pipfile.lock",
    "composer.lock",
    "Gemfile.lock",
    "go.sum",
    "packages.lock.json",
];

/// Coarse classification of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileKind {
    /// Application code.
    Source,
    /// Manifests, container and deployment configuration, CI workflows, `.env` examples.
    Config,
    /// Documentation.
    Doc,
    /// Likely holds real secrets: listed by name only, never read.
    Secret,
    /// Anything else (images, models, binaries, lockfiles, data).
    Other,
}

/// A file of the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Classification.
    pub kind: FileKind,
}

/// A text file read within a byte limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    /// Contents (lossless UTF-8).
    pub content: String,
    /// Number of lines of the whole file.
    pub lines: usize,
}

/// Every file of one snapshot directory.
#[derive(Debug, Clone)]
pub struct SnapshotFiles {
    root: PathBuf,
    files: Vec<SourceFile>,
    by_path: BTreeMap<String, usize>,
}

impl SnapshotFiles {
    /// Walks `root` without following symlinks, skipping dependency, build, cache and VCS
    /// directories.
    pub fn scan(root: &Path) -> Result<Self, Error> {
        let mut files = Vec::new();
        let mut pending = vec![PathBuf::new()];
        while let Some(relative) = pending.pop() {
            let dir = root.join(&relative);
            let entries = std::fs::read_dir(&dir).map_err(|source| Error::Io {
                path: dir.clone(),
                source,
            })?;
            for entry in entries {
                let entry = entry.map_err(|source| Error::Io {
                    path: dir.clone(),
                    source,
                })?;
                let name = entry.file_name().to_string_lossy().into_owned();
                // symlink_metadata: a symlink is neither followed nor listed (R11: the
                // snapshot is untrusted and could point outside itself).
                let meta = entry
                    .path()
                    .symlink_metadata()
                    .map_err(|source| Error::Io {
                        path: entry.path(),
                        source,
                    })?;
                let child = relative.join(&name);
                if meta.is_dir() {
                    if !IGNORED_DIRS.contains(&name.as_str()) {
                        pending.push(child);
                    }
                } else if meta.is_file() {
                    let path = child
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    let kind = classify(&path);
                    files.push(SourceFile {
                        path,
                        size: meta.len(),
                        kind,
                    });
                }
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let by_path = files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.clone(), i))
            .collect();
        Ok(Self {
            root: root.to_owned(),
            files,
            by_path,
        })
    }

    /// Every file, sorted by path.
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// The file at `path`, if listed.
    pub fn get(&self, path: &str) -> Option<&SourceFile> {
        self.by_path.get(path).and_then(|i| self.files.get(*i))
    }

    /// Reads a listed text file. `None` for secrets, files above 1 MiB, unreadable or
    /// binary files.
    pub fn read_text(&self, path: &str) -> Option<TextFile> {
        let file = self.get(path)?;
        if file.kind == FileKind::Secret || file.size > MAX_READ_BYTES {
            return None;
        }
        let bytes = std::fs::read(self.root.join(path)).ok()?;
        if bytes.iter().take(8192).any(|b| *b == 0) {
            return None;
        }
        let content = String::from_utf8(bytes).ok()?;
        let lines = content.lines().count();
        Some(TextFile { content, lines })
    }
}

impl FileIndex for SnapshotFiles {
    fn contains(&self, path: &str) -> bool {
        self.by_path.contains_key(path)
    }

    fn line_count(&self, path: &str) -> Option<usize> {
        self.read_text(path).map(|text| text.lines)
    }
}

/// Path segments and file stems of automated tests.
const TEST_SEGMENTS: &[&str] = &["test", "tests", "__tests__", "spec", "specs"];

/// Whether a repository-relative path looks like an automated test (`tests/`, `*_test.go`,
/// `*.test.ts`, `test_*.py`, `*.spec.js`...).
pub fn is_test_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    path.split('/')
        .any(|segment| TEST_SEGMENTS.contains(&segment))
        || TEST_SEGMENTS.contains(&stem)
        || name.starts_with("test_")
        || name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
}

/// Classifies a repository-relative path.
pub fn classify(path: &str) -> FileKind {
    let name = path.rsplit('/').next().unwrap_or(path);
    let lower = name.to_ascii_lowercase();
    let extension = lower.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    if is_secret(&lower, extension) {
        FileKind::Secret
    } else if LOCKFILES.contains(&name) {
        FileKind::Other
    } else if is_config(path, &lower, extension) {
        FileKind::Config
    } else if SOURCE_EXTENSIONS.contains(&extension) {
        FileKind::Source
    } else if DOC_EXTENSIONS.contains(&extension) {
        FileKind::Doc
    } else {
        FileKind::Other
    }
}

fn is_secret(lower: &str, extension: &str) -> bool {
    let env_example = ["example", "sample", "template", "dist"]
        .iter()
        .any(|marker| lower.contains(marker));
    (lower == ".env" || lower.starts_with(".env.") || extension == "env") && !env_example
        || [
            "pem", "key", "p12", "pfx", "jks", "keystore", "tfvars", "kdbx",
        ]
        .contains(&extension)
        || lower.starts_with("id_rsa")
        || lower.starts_with("id_ed25519")
        // `credentials.json` holds secrets; `credentials.rs` is code that handles them.
        || (lower.starts_with("credentials") || lower.starts_with("secrets."))
            && !SOURCE_EXTENSIONS.contains(&extension)
}

fn is_config(path: &str, lower: &str, extension: &str) -> bool {
    const NAMES: &[&str] = &[
        "cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "pipfile",
        "go.mod",
        "gemfile",
        "composer.json",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "settings.gradle",
        "procfile",
        "fly.toml",
        "vercel.json",
        "netlify.toml",
        "app.yaml",
        "app.yml",
        "serverless.yml",
        "serverless.yaml",
        "chart.yaml",
        "trunk.toml",
        "appsettings.json",
        "application.properties",
        "application.yml",
        "nginx.conf",
        "firebase.json",
        "wrangler.toml",
        "render.yaml",
        "railway.json",
        "makefile",
    ];
    NAMES.contains(&lower)
        || lower.starts_with("dockerfile")
        || extension == "dockerfile"
        || lower.starts_with("docker-compose")
        || lower.starts_with("compose.")
        || lower.starts_with("caddyfile")
        || lower.starts_with("requirements") && extension == "txt"
        || lower.starts_with(".env.") && !is_secret(lower, extension)
        || ["tf", "service", "socket", "timer", "conf", "csproj", "ini"].contains(&extension)
        || (path.starts_with(".github/workflows/")
            || path.contains("/k8s/")
            || path.starts_with("k8s/")
            || path.contains("helm/"))
            && (extension == "yml" || extension == "yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_are_recognized() {
        for path in [
            "tests/api.rs",
            "api/orders_test.go",
            "src/app.test.ts",
            "test_login.py",
            "web/__tests__/x.js",
            "src/a.spec.js",
        ] {
            assert!(is_test_path(path), "{path}");
        }
        for path in ["src/latest.rs", "api/orders.go", "contest/main.go"] {
            assert!(!is_test_path(path), "{path}");
        }
    }

    #[test]
    fn classification_keeps_secrets_out_and_finds_config() {
        assert_eq!(
            classify("app/src/auth/credentials.rs"),
            FileKind::Source,
            "code, not a secret"
        );
        for secret in [
            ".env",
            ".env.production",
            "deploy/prod.env",
            "certs/server.key",
            "id_rsa",
            "infra/prod.tfvars",
            "gcp/credentials.json",
        ] {
            assert_eq!(classify(secret), FileKind::Secret, "{secret}");
        }
        for config in [
            ".env.example",
            "deploy/.env.deploy.example",
            "Dockerfile",
            "docker-compose.yml",
            "Cargo.toml",
            "deploy/tbg-server.service",
            ".github/workflows/ci.yml",
            "deploy/Caddyfile.template",
            "requirements-dev.txt",
        ] {
            assert_eq!(classify(config), FileKind::Config, "{config}");
        }
        assert_eq!(classify("crates/server/src/main.rs"), FileKind::Source);
        assert_eq!(classify("Cargo.lock"), FileKind::Other);
        assert_eq!(classify("art/logo.png"), FileKind::Other);
        assert_eq!(classify("docs/README.md"), FileKind::Doc);
    }

    #[test]
    fn scan_skips_ignored_dirs_and_symlinks_and_never_reads_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n// two\n").unwrap();
        std::fs::write(root.join("node_modules/x/index.js"), "x").unwrap();
        std::fs::write(root.join(".env"), "API_KEY=real").unwrap();
        std::fs::write(root.join("blob.bin"), [0u8, 1, 2]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", root.join("escape")).unwrap();

        let files = SnapshotFiles::scan(root).unwrap();
        let paths: Vec<_> = files.files().iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, [".env", "blob.bin", "src/main.rs"]);
        assert!(files.read_text(".env").is_none(), "R16: never read");
        assert!(files.read_text("blob.bin").is_none(), "binary");
        assert_eq!(files.line_count("src/main.rs"), Some(2));
        assert!(files.contains("src/main.rs") && !files.contains("escape"));
    }
}
