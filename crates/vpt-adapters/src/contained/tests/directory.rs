use super::*;
use crate::contained::directory::{checked_entry_name, entry_name};

#[test]
fn subdirectory_creates_0700_once_and_refuses_a_link_or_a_file() {
    let (_temp, root) = root();
    let made = root.subdirectory("title-copy").expect("made");
    assert_eq!(made.path(), root.path().join("title-copy"));
    assert_eq!(
        fs::metadata(made.path())
            .expect("meta")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        root.subdirectory("title-copy").expect("again").identity(),
        made.identity()
    );
    for leaf in ["dirlink", "plain.m4a"] {
        assert_eq!(
            root.subdirectory(leaf).err(),
            Some(ContainedError::NotADirectory(root.path().join(leaf)))
        );
    }
    assert!(matches!(
        root.subdirectory("../escape"),
        Err(ContainedError::Escape { .. })
    ));
}

#[test]
fn names_lists_every_entry_sorted_and_sync_succeeds() {
    let (_temp, root) = root();
    assert_eq!(
        root.names().expect("names"),
        vec!["dirlink", "link.m4a", "plain.m4a", "sub"]
    );
    assert_eq!(
        root.names().expect("names again"),
        vec!["dirlink", "link.m4a", "plain.m4a", "sub"]
    );
    assert_eq!(root.sync(), Ok(()));
    assert_eq!(
        root.c_name(Path::new("plain.m4a"))
            .expect("name")
            .as_bytes(),
        b"plain.m4a"
    );
    assert!(matches!(
        root.c_name(Path::new("sub/x")),
        Err(ContainedError::Escape { .. })
    ));
}

#[test]
fn directory_names_skip_invalid_utf8_and_dot_entries() {
    for name in [c".", c"..", c"\xff"] {
        assert_eq!(entry_name(name), None);
    }
    assert_eq!(entry_name(c"caf\xc3\xa9"), Some("café"));
}

#[test]
fn strict_name_decoder_refuses_invalid_bytes_with_typed_invalid_data() {
    let (_temp, root) = root();
    assert!(matches!(
        checked_entry_name(c"\xff", root.path()),
        Err(ContainedError::Io { path, kind: std::io::ErrorKind::InvalidData }) if path == root.path()
    ));
}

#[test]
fn strict_name_decoder_keeps_unicode() {
    let (_temp, root) = root();
    assert_eq!(
        checked_entry_name(c"caf\xc3\xa9", root.path()),
        Ok(Some("café"))
    );
}

#[test]
fn strict_name_decoder_skips_dot_entries() {
    let (_temp, root) = root();
    assert_eq!(checked_entry_name(c".", root.path()), Ok(None));
    assert_eq!(checked_entry_name(c"..", root.path()), Ok(None));
}

#[test]
fn strict_names_list_valid_entries_from_the_retained_directory() {
    let (_temp, root) = root();
    fs::write(root.path().join("café"), b"owned").unwrap();
    assert_eq!(
        root.names_strict().unwrap(),
        ["café", "dirlink", "link.m4a", "plain.m4a", "sub"]
    );
}

#[test]
fn strict_names_refuse_a_replaced_retained_root() {
    let (_temp, root) = root();
    let sub = root.open_subdirectory("sub").unwrap();
    fs::rename(sub.path(), root.path().join("held-sub")).unwrap();
    fs::create_dir(sub.path()).unwrap();
    assert!(matches!(
        sub.names_strict(),
        Err(ContainedError::Escape { .. })
    ));
}

#[test]
fn revalidation_refuses_a_replaced_root_directory() {
    let (_temp, root) = root();
    let sub = root.open_subdirectory("sub").expect("sub");
    assert_eq!(sub.revalidate(), Ok(()));
    fs::rename(sub.path(), root.path().join("original")).expect("move");
    fs::create_dir(sub.path()).expect("replacement");
    assert!(matches!(sub.names(), Err(ContainedError::Escape { .. })));
    assert!(matches!(
        sub.revalidate(),
        Err(ContainedError::Escape { .. })
    ));
    fs::rename(sub.path(), root.path().join("replacement")).expect("move replacement");
    symlink(root.path().join("replacement"), sub.path()).expect("link");
    assert!(sub.names().is_err());
    assert!(sub.revalidate().is_err());
}

#[test]
fn retained_enumeration_reads_the_original_directory_twice_after_path_substitution() {
    let (_temp, root) = root();
    let a = root.open_subdirectory("sub").expect("A");
    for name in ["zulu", "alpha", "middle"] {
        fs::write(a.path().join(name), b"A").expect("A entry");
    }
    let b = root.subdirectory("B").expect("B");
    fs::write(b.path().join("impostor"), b"B").expect("B entry");
    a.revalidate().expect("validate A before substitution");
    fs::rename(a.path(), root.path().join("held-A")).expect("retain A");
    symlink(b.path(), a.path()).expect("substitute link to B");
    for _ in 0..2 {
        assert_eq!(
            a.retained_names().expect("retained names"),
            vec!["alpha", "middle", "zulu"]
        );
    }
}

#[test]
fn reading_a_subdirectory_creates_nothing_and_refuses_a_link() {
    let (_temp, root) = root();
    assert!(root.open_subdirectory("sub").is_ok());
    for leaf in ["dirlink", "plain.m4a"] {
        assert_eq!(
            root.open_subdirectory(leaf).err(),
            Some(ContainedError::NotADirectory(root.path().join(leaf)))
        );
    }
    assert!(matches!(
        root.open_subdirectory("missing"),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
    assert!(!root.path().join("missing").exists());
}
