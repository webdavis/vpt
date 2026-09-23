use super::support;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use support::{Sandbox, run, stderr};

fn snapshot(root: &Path) -> Vec<(std::path::PathBuf, u32, i64, i64, Vec<u8>)> {
    let mut paths = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(path) = paths.pop() {
        let metadata = path.symlink_metadata().expect("metadata");
        if metadata.is_dir() {
            paths.extend(
                std::fs::read_dir(&path)
                    .expect("entries")
                    .map(|entry| entry.expect("entry").path()),
            );
        }
        let content = if metadata.is_file() {
            std::fs::read(&path).expect("content")
        } else {
            Vec::new()
        };
        entries.push((
            path,
            metadata.mode(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            content,
        ));
    }
    entries.sort();
    entries
}

#[test]
fn config_equal_to_or_inside_source_is_refused_before_prompt_or_write() {
    let sandbox = Sandbox::new("source-read-only");
    let source = sandbox
        .path()
        .join("home/Library/Group Containers/group.com.apple.VoiceMemos.shared");
    std::fs::create_dir_all(source.join("Recordings/nested")).expect("synthetic source");
    std::fs::write(source.join("Recordings/nested/keep"), "source bytes").expect("source file");
    let before = snapshot(&source);
    for selected in [
        source.join("config.toml"),
        source.join("missing/config.toml"),
    ] {
        let output = run(sandbox.vpt().args(["setup", "--config"]).arg(selected));
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains("Overlap"), "{}", stderr(&output));
        assert!(!stderr(&output).contains("terminal"));
        assert_eq!(snapshot(&source), before);
        assert!(!sandbox.path().join("home/.vpt").exists());
        assert!(!sandbox.path().join("state/vpt").exists());
    }
}
