use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

mod database;
mod directory;
mod fifo;
mod files;

fn root() -> (tempfile::TempDir, RootDir) {
    let temp = tempfile::tempdir().expect("owned fixture");
    let path = temp.path().canonicalize().expect("canonical");
    fs::write(path.join("plain.m4a"), b"bytes").expect("plain");
    fs::create_dir(path.join("sub")).expect("sub");
    symlink(path.join("plain.m4a"), path.join("link.m4a")).expect("link");
    symlink(path.join("sub"), path.join("dirlink")).expect("dir link");
    let root = RootDir::open(&path).expect("root");
    (temp, root)
}

#[test]
fn a_bare_name_or_the_root_joined_with_it_is_the_one_accepted_shape() {
    let temp = tempfile::tempdir().expect("temp");
    let path = temp.path().canonicalize().expect("canonical");
    let expected = path.join("plain.m4a");
    assert_eq!(
        leaf_below(&path, Path::new("plain.m4a")),
        Ok(expected.clone())
    );
    let root = RootDir::open(&path).expect("root");
    assert_eq!(root.leaf(Path::new("plain.m4a")), Ok(expected.clone()));
    assert_eq!(root.leaf(&expected), Ok(expected));
}

#[test]
fn escapes_and_nested_paths_are_refused_naming_root_and_path() {
    let (_temp, root) = root();
    for path in [
        "..",
        "../x",
        "sub/x",
        ".",
        "",
        "/etc/passwd",
        "./plain.m4a",
        "sub/../plain.m4a",
    ] {
        let path = Path::new(path);
        assert_eq!(
            root.leaf(path),
            Err(ContainedError::Escape {
                root: root.path().into(),
                path: path.into()
            })
        );
    }
    let elsewhere = root.path().parent().expect("parent").join("plain.m4a");
    assert_eq!(
        root.leaf(&elsewhere),
        Err(ContainedError::Escape {
            root: root.path().into(),
            path: elsewhere
        })
    );
    assert!(matches!(
        root.c_name(Path::new("bad\0name")),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::InvalidInput,
            ..
        })
    ));
}

#[test]
fn a_root_must_be_a_directory_reached_through_no_link() {
    let (_temp, root) = root();
    fs::create_dir(root.path().join("sub/child")).expect("child");
    for leaf in ["dirlink", "dirlink/child", "plain.m4a"] {
        let path = root.path().join(leaf);
        assert_eq!(
            RootDir::open(&path).err(),
            Some(ContainedError::NotADirectory(path))
        );
    }
    assert!(RootDir::open(&root.path().join("sub")).is_ok());
    assert!(matches!(
        RootDir::open(&root.path().join("absent")),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn a_cloned_root_retains_its_identity_and_refuses_replacement() {
    let (temp, root) = root();
    let clone = root.try_clone().expect("clone");
    let identity = root.identity().expect("identity");
    assert_eq!(identity, clone.identity().expect("cloned identity"));
    drop(root);
    assert_eq!(clone.stat(Path::new("plain.m4a")).expect("stat").size, 5);
    assert_eq!(clone.revalidate(), Ok(()));
    let held = temp.path().join("held");
    fs::create_dir(&held).expect("held");
    let sub = clone.open_subdirectory("sub").expect("sub");
    fs::rename(sub.path(), held.join("sub")).expect("move");
    fs::create_dir(sub.path()).expect("replacement");
    assert!(matches!(
        sub.revalidate(),
        Err(ContainedError::Escape { .. })
    ));
    assert_eq!(clone.identity().expect("retained"), identity);
}
