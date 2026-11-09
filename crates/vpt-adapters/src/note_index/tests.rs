use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

fn write(root: &Path, name: &str, text: &str) -> PathBuf {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn recursively_indexes_external_names_aliases_and_tags_without_writing() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let path = write(
        root,
        "people/Ada.md",
        "---\naliases: ['Countess of Lovelace', Ada]\ntags: [Project_Finance]\n---\nPrivate prose\n",
    );
    write(root, "Plain.MD", "# Ordinary note\n");
    write(root, "ignore.txt", "---\ntags: [ignore]\n---\n");
    let before = fs::read(&path).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let index = Index::read(&[root.to_path_buf()]).unwrap();
    assert_eq!(index.notes().len(), 2);
    assert_eq!(
        index.matching("countess OF lovelace"),
        std::slice::from_ref(&path)
    );
    assert_eq!(index.matching("Ada"), std::slice::from_ref(&path));
    assert_eq!(index.notes()[1].metadata.tags, ["Project_Finance"]);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::metadata(&path).unwrap().modified().unwrap(),
        metadata.modified().unwrap()
    );
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode(),
        metadata.permissions().mode()
    );
    assert!(index.diagnostics().is_empty());
}

#[test]
fn symlink_files_directories_and_cycles_are_not_followed() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("vault");
    fs::create_dir(&root).unwrap();
    let outside = write(
        temporary.path(),
        "outside/Private.md",
        "---\naliases: [PrivateCanary]\n---\n",
    );
    std::os::unix::fs::symlink(&outside, root.join("linked.md")).unwrap();
    std::os::unix::fs::symlink(outside.parent().unwrap(), root.join("linked-dir")).unwrap();
    std::os::unix::fs::symlink(&root, root.join("cycle")).unwrap();
    let local = write(&root, "Local.md", "# Local\n");
    let index = Index::read(std::slice::from_ref(&root)).unwrap();
    assert_eq!(index.matching("Local"), &[local]);
    assert!(index.matching("PrivateCanary").is_empty());
    assert_eq!(index.notes().len(), 1);
    assert!(Index::read(&[root.join("linked-dir")]).is_err());
}

#[test]
fn overlapping_roots_and_duplicate_aliases_do_not_create_false_ambiguity() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let first = write(
        root,
        "nested/First.md",
        "---\naliases: [Shared, SHARED, First]\n---\n",
    );
    let second = write(root, "Second.md", "---\naliases: [shared]\n---\n");
    let index =
        Index::read(&[root.join("nested"), root.to_path_buf(), root.join("absent")]).unwrap();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(index.matching("shared"), expected);
    assert_eq!(index.notes().len(), 2);
    assert_eq!(index.matching("first").len(), 1);
}

#[test]
fn malformed_metadata_keeps_the_filename_and_reports_only_a_safe_diagnostic() {
    let temporary = tempfile::tempdir().unwrap();
    let path = write(
        temporary.path(),
        "Broken.md",
        "---\naliases: [PrivateCanary\n---\n",
    );
    let index = Index::read(&[temporary.path().to_path_buf()]).unwrap();
    assert_eq!(index.matching("Broken"), std::slice::from_ref(&path));
    assert!(index.notes()[0].metadata.aliases.is_empty());
    assert_eq!(
        index.diagnostics(),
        &[Diagnostic {
            path,
            kind: DiagnosticKind::Metadata
        }]
    );
    assert!(!format!("{index:?}").contains("PrivateCanary"));
}

#[test]
fn entry_read_depth_and_retained_metadata_limits_are_enforced() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(root, "nested/Note.md", "---\naliases: [Alias]\n---\nBody\n");
    for limits in [
        Limits {
            entries: 1,
            ..Limits::default()
        },
        Limits {
            read_bytes: 1,
            ..Limits::default()
        },
        Limits {
            depth: 0,
            ..Limits::default()
        },
        Limits {
            metadata_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert!(Index::read_with(&[root.to_path_buf()], limits).is_err());
    }
}

#[test]
fn configured_paths_consume_capacity_even_when_the_index_is_empty() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(
        Index::read_with(
            &[temporary.path().to_path_buf()],
            Limits {
                metadata_bytes: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn empty_aliases_still_consume_retained_metadata_capacity() {
    let temporary = tempfile::tempdir().unwrap();
    write(
        temporary.path(),
        "Note.md",
        &format!("---\naliases: [{}]\n---\n", vec!["''"; 256].join(",")),
    );
    assert!(
        Index::read_with(
            &[temporary.path().to_path_buf()],
            Limits {
                metadata_bytes: 1024,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
