//! Hardened diff and stable patch identity (phase 7, ADR-0032/0033).

use opentatu_core::{CommitHash, FileChange, FixIdentity, Slug};

use crate::{Error, Snapshots, git::Git};

impl Snapshots {
    /// Changed paths, preserving deletions and both sides of renames. NUL delimiters
    /// avoid ambiguity from whitespace, tabs or newlines in untrusted filenames.
    pub async fn changed_files(
        &self,
        slug: &Slug,
        base: &CommitHash,
        head: &CommitHash,
    ) -> Result<Vec<FileChange>, Error> {
        let git = Git::in_repo(&self.layout.mirror_dir(slug));
        let text = git
            .run(
                "changed paths",
                &[
                    "diff",
                    "--name-status",
                    "-z",
                    "--find-renames",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--end-of-options",
                    base.as_str(),
                    head.as_str(),
                    "--",
                ],
            )
            .await?;
        let mut fields = text.split_terminator('\0');
        let mut changes = Vec::new();
        while let Some(status) = fields.next() {
            let first = fields
                .next()
                .ok_or_else(|| Error::UnsafeInput("invalid git name-status output".into()))?;
            let (path, previous_path) = if status.starts_with('R') || status.starts_with('C') {
                (
                    fields
                        .next()
                        .ok_or_else(|| Error::UnsafeInput("incomplete rename".into()))?,
                    Some(first.to_owned()),
                )
            } else {
                (first, None)
            };
            changes.push(FileChange {
                path: path.to_owned(),
                previous_path,
                status: status.to_owned(),
            });
        }
        Ok(changes)
    }

    /// Unified diff for explicitly selected paths only. The caller excludes secret and
    /// unsafe paths before requesting bytes; all paths follow `--` as literal pathspecs.
    pub async fn unified_diff(
        &self,
        slug: &Slug,
        base: &CommitHash,
        head: &CommitHash,
        paths: &[&str],
    ) -> Result<String, Error> {
        if paths.is_empty() {
            return Ok(String::new());
        }
        for path in paths {
            if path.is_empty() || path.starts_with('/') || path.split('/').any(|p| p == "..") {
                return Err(Error::UnsafeInput((*path).to_owned()));
            }
        }
        let mut args = vec![
            "--literal-pathspecs",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--find-renames",
            "--unified=3",
            "--end-of-options",
            base.as_str(),
            head.as_str(),
            "--",
        ];
        args.extend_from_slice(paths);
        Git::in_repo(&self.layout.mirror_dir(slug))
            .run("unified diff", &args)
            .await
    }

    /// Records all fix-commit patch IDs and the squash-equivalent full diff before
    /// approval. `branch_head` is the configured application branch, not the fix branch.
    pub async fn fix_identity(
        &self,
        slug: &Slug,
        branch_head: &CommitHash,
        approved: &CommitHash,
    ) -> Result<FixIdentity, Error> {
        let git = Git::in_repo(&self.layout.mirror_dir(slug));
        let base = git
            .run(
                "fix merge base",
                &[
                    "merge-base",
                    "--end-of-options",
                    branch_head.as_str(),
                    approved.as_str(),
                ],
            )
            .await?;
        let base: CommitHash = base
            .trim()
            .parse()
            .map_err(|_| Error::UnknownRevision("fix merge base".into()))?;
        let commits = self.patch_ids(slug, &base, approved).await?;
        let diff = git
            .run(
                "whole fix diff",
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--end-of-options",
                    base.as_str(),
                    approved.as_str(),
                    "--",
                ],
            )
            .await?;
        let whole = patch_id(&git, &diff).await?;
        Ok(FixIdentity {
            base,
            branch_head: branch_head.clone(),
            commits,
            whole,
        })
    }

    /// Stable patch IDs of commits reachable from `head` but not `base`.
    pub async fn patch_ids(
        &self,
        slug: &Slug,
        base: &CommitHash,
        head: &CommitHash,
    ) -> Result<Vec<String>, Error> {
        let git = Git::in_repo(&self.layout.mirror_dir(slug));
        let range = format!("{}..{}", base.as_str(), head.as_str());
        let list = git
            .run(
                "list changed commits",
                &["rev-list", "--reverse", "--end-of-options", &range, "--"],
            )
            .await?;
        let mut ids = Vec::new();
        for line in list.lines() {
            let commit: CommitHash = line
                .parse()
                .map_err(|_| Error::UnknownRevision("patch commit".into()))?;
            let diff = git
                .run(
                    "commit patch",
                    &[
                        "show",
                        "--format=",
                        "--diff-merges=first-parent",
                        "--no-ext-diff",
                        "--no-textconv",
                        "--no-color",
                        "--end-of-options",
                        commit.as_str(),
                        "--",
                    ],
                )
                .await?;
            if let Some(id) = patch_id(&git, &diff).await? {
                ids.push(id);
            }
        }
        Ok(ids)
    }
}

async fn patch_id(git: &Git, diff: &str) -> Result<Option<String>, Error> {
    let output = git
        .run_input(
            "stable patch identity",
            &["patch-id", "--stable"],
            Some(diff.as_bytes()),
        )
        .await?;
    Ok(output.split_whitespace().next().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use opentatu_core::DataLayout;

    #[tokio::test]
    async fn diff_preserves_renames_deletions_and_literal_paths() {
        let source = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let url = testing::create_repo(
            source.path(),
            &[
                ("old.go", "package old\n"),
                ("gone.go", "delete\n"),
                ("a[1].go", "before\n"),
            ],
        );
        let snapshots = Snapshots::new(DataLayout::new(data.path()));
        let slug = Slug::new("demo").unwrap();
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        let base = snapshots.resolve(&slug, "main").await.unwrap();
        std::fs::rename(source.path().join("old.go"), source.path().join("new.go")).unwrap();
        std::fs::remove_file(source.path().join("gone.go")).unwrap();
        testing::commit_files(source.path(), &[("a[1].go", "after\n")], "change");
        snapshots.refresh_mirror(&slug, &url).await.unwrap();
        let head = snapshots.resolve(&slug, "main").await.unwrap();
        let changes = snapshots.changed_files(&slug, &base, &head).await.unwrap();
        assert!(
            changes
                .iter()
                .any(|c| c.path == "new.go" && c.previous_path.as_deref() == Some("old.go"))
        );
        assert!(
            changes
                .iter()
                .any(|c| c.path == "gone.go" && c.status == "D")
        );
        let diff = snapshots
            .unified_diff(&slug, &base, &head, &["a[1].go"])
            .await
            .unwrap();
        assert!(diff.contains("-before") && diff.contains("+after"));
        assert!(!diff.contains("gone.go"));
        assert!(
            snapshots
                .unified_diff(&slug, &base, &head, &["../escape"])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn approved_identity_survives_squash_and_rebase() {
        for squash in [true, false] {
            let source = tempfile::tempdir().unwrap();
            let data = tempfile::tempdir().unwrap();
            let url = testing::create_repo(source.path(), &[("route.go", "unsafe\n")]);
            testing::checkout_new_branch(source.path(), "fix", "main");
            testing::commit_files(source.path(), &[("route.go", "safe\n")], "fix");
            testing::commit_files(
                source.path(),
                &[("route_test.go", "regression test\n")],
                "test",
            );
            testing::checkout(source.path(), "main");
            testing::commit_files(
                source.path(),
                &[("other.go", "unrelated\n")],
                "advance main",
            );
            let snapshots = Snapshots::new(DataLayout::new(data.path()));
            let slug = Slug::new("demo").unwrap();
            snapshots.refresh_mirror(&slug, &url).await.unwrap();
            let head = snapshots.resolve(&slug, "main").await.unwrap();
            let approved = snapshots.resolve(&slug, "fix").await.unwrap();
            let identity = snapshots
                .fix_identity(&slug, &head, &approved)
                .await
                .unwrap();
            assert_eq!(identity.commits.len(), 2);
            assert_eq!(
                identity.matched_by(&snapshots.patch_ids(&slug, &head, &head).await.unwrap()),
                None,
                "never merged"
            );
            if squash {
                testing::squash_merge(source.path(), "fix");
            } else {
                testing::checkout(source.path(), "fix");
                testing::rebase(source.path(), "main");
                testing::checkout(source.path(), "main");
                testing::merge(source.path(), "fix");
            }
            snapshots.refresh_mirror(&slug, &url).await.unwrap();
            let merged = snapshots.resolve(&slug, "main").await.unwrap();
            assert!(
                !snapshots
                    .is_ancestor(&slug, &approved, &merged)
                    .await
                    .unwrap()
            );
            let patches = snapshots
                .patch_ids(&slug, &identity.branch_head, &merged)
                .await
                .unwrap();
            assert_eq!(
                identity.matched_by(&patches),
                Some(if squash {
                    "patch_id_squash"
                } else {
                    "patch_id_rebase"
                })
            );
        }
    }
}
