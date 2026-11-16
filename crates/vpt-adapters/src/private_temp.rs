use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

/// Creates a folder only this user can open, under the resolved platform temp directory, for
/// `vpt test` to work in. It is removed when dropped.
pub fn private_tempdir() -> std::io::Result<TempDir> {
    tempfile::Builder::new()
        .prefix("vpt-test-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(std::env::temp_dir().canonicalize()?)
}
