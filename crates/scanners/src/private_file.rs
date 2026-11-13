//! Shared owner-private regular-file admission for dedicated external credentials (R16).
use std::path::Path;

#[cfg(target_os = "linux")]
pub(crate) fn is_private_file(path: &Path, max: u64) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    let Ok(owner) = std::fs::metadata("/proc/self") else {
        return false;
    };
    std::fs::symlink_metadata(path).is_ok_and(|m| {
        m.is_file()
            && m.uid() == owner.uid()
            && m.mode() & 0o077 == 0
            && m.len() > 0
            && m.len() <= max
    })
}
#[cfg(not(target_os = "linux"))]
pub(crate) fn is_private_file(_path: &Path, _max: u64) -> bool {
    false
}
