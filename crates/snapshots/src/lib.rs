//! # opentatu-snapshots — the exact code an audit analyzes (R02)
//!
//! Keeps a bare mirror of each app's repository under the data directory, resolves a branch
//! or revision to a full commit hash, and checks that commit out into a snapshot directory
//! (`DataLayout::snapshot_dir`). Snapshots are reused when the same commit is audited again.
//!
//! The only code that runs `git`. Hardening (R12): no hooks (`core.hooksPath=/dev/null`), no
//! `ext::` transport, no interactive prompts, `--` / `--end-of-options` before every
//! user-provided value, and credentials in URLs redacted from error messages.

mod error;
mod git;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

use std::path::{Path, PathBuf};

use opentatu_core::{CommitHash, DataLayout, Slug};

pub use error::Error;

use crate::git::Git;

/// Marker written last: a snapshot directory without it is incomplete and is rebuilt.
const COMPLETE_MARKER: &str = ".opentatu-snapshot";

/// A checked-out commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Snapshot directory (`DataLayout::snapshot_dir`).
    pub dir: PathBuf,
    /// Whether an earlier audit had already captured this commit.
    pub reused: bool,
}

/// Mirrors, resolves and captures repositories under the data directory.
#[derive(Debug, Clone)]
pub struct Snapshots {
    layout: DataLayout,
}

impl Snapshots {
    /// Snapshots stored under `layout`.
    pub fn new(layout: DataLayout) -> Self {
        Self { layout }
    }

    /// Clones the repository into its mirror on first use; fetches it afterwards.
    pub async fn refresh_mirror(&self, slug: &Slug, repo_url: &str) -> Result<(), Error> {
        if repo_url.is_empty() || repo_url.starts_with('-') || repo_url.contains("ext::") {
            return Err(Error::UnsafeInput(repo_url.to_owned()));
        }
        let mirror = self.layout.mirror_dir(slug);
        if mirror.join("HEAD").is_file() {
            Git::in_repo(&mirror)
                .run("fetch mirror", &["fetch", "--prune", "--quiet", "origin"])
                .await?;
        } else {
            create_parent(&mirror).await?;
            let target = path_str(&mirror)?;
            Git::global()
                .run(
                    "clone mirror",
                    &["clone", "--mirror", "--quiet", "--", repo_url, target],
                )
                .await?;
        }
        Ok(())
    }

    /// Resolves `revision` (branch, tag or commit) in the mirror to a full commit hash.
    pub async fn resolve(&self, slug: &Slug, revision: &str) -> Result<CommitHash, Error> {
        if revision.is_empty() || revision.starts_with('-') {
            return Err(Error::UnsafeInput(revision.to_owned()));
        }
        let spec = format!("{revision}^{{commit}}");
        let output = Git::in_repo(&self.layout.mirror_dir(slug))
            .run(
                "resolve revision",
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    "--end-of-options",
                    &spec,
                ],
            )
            .await
            .map_err(|_| Error::UnknownRevision(revision.to_owned()))?;
        output
            .trim()
            .parse()
            .map_err(|_| Error::UnknownRevision(revision.to_owned()))
    }

    /// Commits every file of `dir` — a tree OpenTatu wrote itself, such as the sanitized copy
    /// of a benchmark app — creating the repository on branch `main` if needed. Returns the
    /// commit, so the tree can be registered and audited like any app.
    pub async fn record_tree(&self, dir: &Path, message: &str) -> Result<CommitHash, Error> {
        let git = Git::in_worktree(dir);
        if !dir.join(".git").exists() {
            git.run("init repository", &["init", "--quiet", "-b", "main"])
                .await?;
        }
        // `--force`: the tree's own `.gitignore` must not drop files OpenTatu wrote.
        git.run("stage tree", &["add", "--all", "--force"]).await?;
        git.run(
            "commit tree",
            &[
                "-c",
                "user.name=OpenTatu",
                "-c",
                "user.email=opentatu@localhost.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                message,
            ],
        )
        .await?;
        let head = git
            .run("read commit", &["rev-parse", "--verify", "HEAD"])
            .await?;
        head.trim()
            .parse()
            .map_err(|_| Error::UnknownRevision("HEAD".to_owned()))
    }

    /// Whether `ancestor` is in the history of `descendant` (both in the mirror). Used by
    /// reverifications: a closed finding's code in a commit that does not contain its fix
    /// is not a regression of that commit (R09).
    pub async fn is_ancestor(
        &self,
        slug: &Slug,
        ancestor: &CommitHash,
        descendant: &CommitHash,
    ) -> Result<bool, Error> {
        Git::in_repo(&self.layout.mirror_dir(slug))
            .check(
                "check ancestry",
                &[
                    "merge-base",
                    "--is-ancestor",
                    "--end-of-options",
                    ancestor.as_str(),
                    descendant.as_str(),
                ],
            )
            .await
    }

    /// Checks `commit` out into its snapshot directory. Reuses a complete snapshot of the
    /// same commit; rebuilds an incomplete one.
    pub async fn capture(&self, slug: &Slug, commit: &CommitHash) -> Result<Snapshot, Error> {
        let target = self.layout.snapshot_dir(slug, commit);
        if target.join(COMPLETE_MARKER).is_file() {
            return Ok(Snapshot {
                dir: target,
                reused: true,
            });
        }
        let partial = target.with_extension("partial");
        remove_dir_if_exists(&partial).await?;
        remove_dir_if_exists(&target).await?;
        create_parent(&partial).await?;
        let mirror = self.layout.mirror_dir(slug);
        Git::global()
            .run(
                "clone snapshot",
                &[
                    "clone",
                    "--quiet",
                    "--no-checkout",
                    "--",
                    path_str(&mirror)?,
                    path_str(&partial)?,
                ],
            )
            .await?;
        Git::in_worktree(&partial)
            .run(
                "checkout commit",
                &[
                    "-c",
                    "advice.detachedHead=false",
                    "checkout",
                    "--quiet",
                    "--detach",
                    commit.as_str(),
                ],
            )
            .await?;
        tokio::fs::write(partial.join(COMPLETE_MARKER), commit.as_str())
            .await
            .map_err(|source| Error::Io {
                path: partial.clone(),
                source,
            })?;
        tokio::fs::rename(&partial, &target)
            .await
            .map_err(|source| Error::Io {
                path: target.clone(),
                source,
            })?;
        Ok(Snapshot {
            dir: target,
            reused: false,
        })
    }
}

fn path_str(path: &Path) -> Result<&str, Error> {
    path.to_str()
        .ok_or_else(|| Error::UnsafeInput(path.display().to_string()))
}

async fn create_parent(path: &Path) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| Error::Io {
                path: parent.to_owned(),
                source,
            })?;
    }
    Ok(())
}

async fn remove_dir_if_exists(path: &Path) -> Result<(), Error> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_repo(dir: &Path) -> String {
        testing::create_repo(dir, &[("route.ts", "export const GET = () => 1;\n")])
    }

    #[tokio::test]
    async fn r02_mirror_resolve_and_capture_exact_commit() {
        let source = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let url = source_repo(source.path());
        let snapshots = Snapshots::new(DataLayout::new(data.path()));
        let slug = Slug::new("demo").unwrap();

        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        let commit = snapshots.resolve(&slug, "main").await.unwrap();
        let snapshot = snapshots.capture(&slug, &commit).await.unwrap();
        let dir = snapshot.dir.clone();
        assert!(!snapshot.reused);

        assert_eq!(
            std::fs::read_to_string(dir.join("route.ts")).unwrap(),
            "export const GET = () => 1;\n"
        );
        assert!(dir.join(COMPLETE_MARKER).is_file());
        // Second capture of the same commit reuses the snapshot; a second refresh fetches.
        assert_eq!(
            snapshots.capture(&slug, &commit).await.unwrap(),
            Snapshot { dir, reused: true }
        );
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        assert_eq!(
            snapshots.resolve(&slug, commit.as_str()).await.unwrap(),
            commit
        );
    }

    #[tokio::test]
    async fn ancestry_follows_the_history_of_the_mirror() {
        let source = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let url = source_repo(source.path());
        let snapshots = Snapshots::new(DataLayout::new(data.path()));
        let slug = Slug::new("demo").unwrap();
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        let first = snapshots.resolve(&slug, "main").await.unwrap();
        testing::commit_files(source.path(), &[("route.ts", "fixed\n")], "fix");
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        let second = snapshots.resolve(&slug, "main").await.unwrap();
        assert!(snapshots.is_ancestor(&slug, &first, &second).await.unwrap());
        assert!(!snapshots.is_ancestor(&slug, &second, &first).await.unwrap());
        let ghost: CommitHash = "0000000000000000000000000000000000000001".parse().unwrap();
        assert!(snapshots.is_ancestor(&slug, &ghost, &second).await.is_err());
    }

    #[tokio::test]
    async fn a_written_tree_becomes_an_auditable_commit() {
        let tree = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        std::fs::write(tree.path().join("app.ts"), "export {}\n").unwrap();
        std::fs::write(tree.path().join(".gitignore"), "vendor.js\n").unwrap();
        std::fs::write(tree.path().join("vendor.js"), "lib()\n").unwrap();
        let snapshots = Snapshots::new(DataLayout::new(data.path()));
        let commit = snapshots
            .record_tree(tree.path(), "benchmark copy")
            .await
            .unwrap();
        let slug = Slug::new("demo").unwrap();
        let url = tree.path().to_string_lossy().into_owned();
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        assert_eq!(snapshots.resolve(&slug, "main").await.unwrap(), commit);
        let snapshot = snapshots.capture(&slug, &commit).await.unwrap();
        assert!(snapshot.dir.join("app.ts").is_file());
        assert!(
            snapshot.dir.join("vendor.js").is_file(),
            "ignored by the tree's .gitignore, still committed"
        );
    }

    #[tokio::test]
    async fn unknown_revision_and_unsafe_input_are_rejected() {
        let source = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let url = source_repo(source.path());
        let snapshots = Snapshots::new(DataLayout::new(data.path()));
        let slug = Slug::new("demo").unwrap();
        snapshots.refresh_mirror(&slug, &url).await.unwrap();

        assert!(matches!(
            snapshots.resolve(&slug, "no-such-branch").await,
            Err(Error::UnknownRevision(_))
        ));
        assert!(matches!(
            snapshots.resolve(&slug, "--all").await,
            Err(Error::UnsafeInput(_))
        ));
        assert!(matches!(
            snapshots
                .refresh_mirror(&slug, "--upload-pack=touch /tmp/x")
                .await,
            Err(Error::UnsafeInput(_))
        ));
    }
}
