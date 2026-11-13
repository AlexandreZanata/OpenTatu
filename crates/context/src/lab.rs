//! A bounded, secret-excluding copy for the VM; reuses SnapshotFiles (R10, R14–R16).
use crate::SnapshotFiles;
use opentatu_core::ContentHasher;
use std::path::Path;

/// Private temporary input copy, deleted when its last owner drops it.
#[derive(Debug)]
pub struct PreparedLabSource {
    dir: tempfile::TempDir,
    /// Digest of ordered relative paths and exact contents.
    pub digest: String,
    /// Paths excluded from the selected prefixes, never silently treated as tested.
    pub omitted: Vec<String>,
}
impl PreparedLabSource {
    /// Directory copied into the read-only VM input disk.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// Copies listed text files only. Secret files, symlinks, binary/oversized files and
/// ignored build/dependency trees never enter the guest. No application is built here.
pub fn prepare_lab_source(
    files: &SnapshotFiles,
    prefixes: &[String],
) -> std::io::Result<PreparedLabSource> {
    let dir = tempfile::tempdir()?;
    let mut digest = ContentHasher::default();
    let mut omitted = Vec::new();
    let mut bytes = 0usize;
    for file in files.files().iter().filter(|f| {
        prefixes
            .iter()
            .any(|p| p == "." || f.path == *p || f.path.starts_with(&(p.clone() + "/")))
    }) {
        let Some(text) = files.read_text(&file.path) else {
            omitted.push(file.path.clone());
            continue;
        };
        bytes = bytes.saturating_add(text.content.len());
        if bytes > 128 * 1024 * 1024 {
            return Err(std::io::Error::other("lab source copy exceeds 128 MiB"));
        }
        let path = dir.path().join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &text.content)?;
        digest.update_entry(&file.path, text.content.as_bytes());
    }
    Ok(PreparedLabSource {
        dir,
        digest: digest.finish(),
        omitted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r15_lab_copy_reuses_secret_and_symlink_exclusion() {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(source.path().join("src")).unwrap();
        std::fs::write(source.path().join("src/main.go"), "package main\n").unwrap();
        std::fs::write(source.path().join("src/.env"), "never_copy_me").unwrap();
        std::os::unix::fs::symlink(".env", source.path().join("src/link.go")).unwrap();
        let files = SnapshotFiles::scan(source.path()).unwrap();
        let copy = prepare_lab_source(&files, &["src".into()]).unwrap();
        assert!(copy.path().join("src/main.go").exists());
        assert!(!copy.path().join("src/.env").exists());
        assert!(!copy.path().join("src/link.go").exists());
        assert_eq!(
            copy.digest,
            prepare_lab_source(&files, &["src".into()]).unwrap().digest
        );
        assert!(copy.omitted.contains(&"src/.env".into()));
    }
}
