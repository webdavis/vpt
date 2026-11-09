use super::*;
use std::{fs, os::unix::fs::symlink};

#[test]
fn reads_whole_lines_with_punctuation_and_unicode_intact() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terms.txt");
    fs::write(&path, "  New York  \r\nStraße\nAnn,\n\n").unwrap();
    assert_eq!(read(&path).unwrap(), ["New York", "Straße", "Ann,"]);
}

#[test]
fn missing_file_or_parent_is_observationally_empty() {
    let temp = tempfile::tempdir().unwrap();
    assert!(read(&temp.path().join("missing")).unwrap().is_empty());
    assert!(read(&temp.path().join("missing/terms")).unwrap().is_empty());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn redirects_relative_paths_and_nonregular_files_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("real"), "secret").unwrap();
    symlink(temp.path().join("real"), temp.path().join("link")).unwrap();
    assert!(read(&temp.path().join("link")).is_err());
    assert!(read(temp.path()).is_err());
    assert_eq!(read(Path::new("relative.txt")), Err(TermsError::UnsafePath));
    assert_eq!(
        read(&temp.path().join("../terms")),
        Err(TermsError::UnsafePath)
    );
}

#[test]
fn oversized_invalid_utf8_and_control_characters_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("terms");
    for bytes in [vec![0xff], b"canary\0private".to_vec()] {
        fs::write(&path, bytes).unwrap();
        assert_eq!(read(&path), Err(TermsError::InvalidText));
    }
    fs::write(&path, b"").unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(read(&path), Err(TermsError::TooLarge));
}
