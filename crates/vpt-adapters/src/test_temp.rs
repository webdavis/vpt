/// Creates a temp directory under the resolved platform temp directory. The code under test
/// refuses a root behind a symlink, and the platform temp directory may sit behind one.
pub(crate) fn tempdir() -> std::io::Result<tempfile::TempDir> {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}
