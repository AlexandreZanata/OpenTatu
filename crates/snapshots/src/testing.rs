//! Test support (feature `test-support`): throwaway git repositories for tests of this and
//! upper crates. The one place tests create repositories — reuse it.

// Test fixtures fail loudly: panicking is the right behavior here, never in product code.
#![allow(clippy::panic)]

use std::path::Path;
use std::process::Command;

/// Creates a git repository in `dir` with `files` committed on branch `main`; returns the
/// path to use as `repo_url`.
pub fn create_repo(dir: &Path, files: &[(&str, &str)]) -> String {
    git(dir, &["init", "--quiet", "-b", "main"]);
    commit_files(dir, files, "initial commit");
    dir.to_string_lossy().into_owned()
}

/// Writes `files` and commits them on the current branch.
pub fn commit_files(dir: &Path, files: &[(&str, &str)], message: &str) {
    for (name, content) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {parent:?}: {e}"));
        }
        std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    }
    git(dir, &["add", "--all"]);
    git(dir, &["commit", "--quiet", "-m", message]);
}

/// Creates branch `name` at `start` (a branch or commit) and checks it out; later
/// `commit_files` calls commit on it.
pub fn checkout_new_branch(dir: &Path, name: &str, start: &str) {
    git(dir, &["checkout", "--quiet", "-b", name, start]);
}

fn git(dir: &Path, args: &[&str]) {
    // Test fixture: builds a throwaway repository with the git CLI (this crate owns git).
    #[allow(clippy::disallowed_methods)]
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=OpenTatu Test",
            "-c",
            "user.email=test@example.test",
        ])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("cannot run git: {e}"));
    assert!(status.success(), "git {args:?} failed");
}
