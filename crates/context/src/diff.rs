//! Budgeted incremental diff context; secret and non-source/config content is excluded
//! before git is asked for patch bytes (R14–R16).

use opentatu_core::FileChange;

use crate::{Excerpt, FileKind, Selection, SnapshotFiles, classify};

/// Whether both sides of a change are eligible for diff context. This check is used
/// before requesting a patch, including deleted files and renames out of secret paths.
pub fn diff_allowed(change: &FileChange) -> bool {
    let allowed = |path: &str| matches!(classify(path), FileKind::Source | FileKind::Config);
    allowed(&change.path) && change.previous_path.as_deref().is_none_or(allowed)
}

/// Packs complete per-file unified patches in an explicit byte budget. Never truncates
/// UTF-8 or silently discards a changed path; excluded patches are `left_out` (R14).
pub fn render_diff(
    changes: &[FileChange],
    patches: &[(String, String)],
    budget: usize,
) -> Selection {
    let mut selection = Selection::default();
    for change in changes {
        let patch = patches.iter().find(|(path, _)| path == &change.path);
        if diff_allowed(change)
            && let Some((_, text)) = patch
            && selection.bytes.saturating_add(text.len()) <= budget
        {
            selection.bytes += text.len();
            selection.excerpts.push(Excerpt {
                path: change.path.clone(),
                text: text.clone(),
                partial: false,
            });
            continue;
        }
        selection.left_out.push(change.path.clone());
    }
    selection
}

/// Entry-point files whose depth-2 selected related context includes a changed path.
/// The normal selector owns dependency discovery; this function does not duplicate it.
pub fn affected_entry_points(
    files: &SnapshotFiles,
    entries: &[opentatu_core::EntryPoint],
    changed: &[String],
) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| {
            if changed.contains(&entry.file) {
                return true;
            }
            let selected = crate::entry_point_code(
                files,
                &[crate::Seed {
                    path: entry.file.clone(),
                    line: entry.line,
                }],
                usize::MAX,
                1,
            );
            selected.excerpts.iter().any(|e| changed.contains(&e.path))
        })
        .map(|e| e.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r14_r15_diff_is_bounded_and_secret_renames_are_excluded() {
        let change = |path: &str, old: Option<&str>| FileChange {
            path: path.into(),
            previous_path: old.map(str::to_owned),
            status: "M".into(),
        };
        let changes = vec![
            change("a.go", None),
            change("b.go", None),
            change(".env", None),
            change("key.go", Some(".env")),
        ];
        let patches = vec![
            ("a.go".into(), "éé".into()),
            ("b.go".into(), "large".into()),
            (".env".into(), "secret".into()),
        ];
        let selected = render_diff(&changes, &patches, 4);
        assert_eq!(selected.render(), "éé");
        assert_eq!(selected.left_out, ["b.go", ".env", "key.go"]);
        assert_eq!(selected.bytes, 4);
    }

    #[test]
    fn scope_follows_related_code_without_selecting_an_unrelated_handler() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("api")).unwrap();
        std::fs::create_dir_all(dir.path().join("other")).unwrap();
        std::fs::write(
            dir.path().join("api/route.ts"),
            "import { auth } from './auth';\nexport const GET = auth();\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("api/auth.ts"),
            "export function auth() {}\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("other/route.ts"), "export const GET = 1;\n").unwrap();
        let files = SnapshotFiles::scan(dir.path()).unwrap();
        let entry = opentatu_core::EntryPoint {
            id: "EP-001".into(),
            method: "GET".into(),
            path: "/one".into(),
            file: "api/route.ts".into(),
            line: Some(1),
            symbol: "GET".into(),
            authentication: opentatu_core::AuthenticationRequirement::Unknown,
            allowed_roles: vec![],
            inputs: vec![],
            authorization_check: "unknown".into(),
            sensitive_data: vec![],
            applicable_asvs: vec![],
        };
        let mut other = entry.clone();
        other.id = "EP-002".into();
        other.file = "other/route.ts".into();
        assert_eq!(
            affected_entry_points(&files, &[entry, other], &["api/auth.ts".into()]),
            ["EP-001"]
        );
    }
}
