use super::*;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

#[test]
fn stat_judges_the_leaf_itself_and_regular_refuses_a_link() {
    let (_temp, root) = root();
    let fixture = fs::File::open(root.path().join("plain.m4a")).expect("fixture");
    let modified = std::time::UNIX_EPOCH + std::time::Duration::new(1_700_000_000, 123_456_789);
    fixture
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("fixture time");
    assert_eq!(
        // SAFETY: fixture owns a live regular-file descriptor; UF_HIDDEN is a mutable fixture flag.
        unsafe { libc::fchflags(fixture.as_raw_fd(), libc::UF_HIDDEN) },
        0
    );
    let plain = root.stat(Path::new("plain.m4a")).expect("stat");
    assert_eq!((plain.kind, plain.size), (Kind::File, 5));
    assert_eq!(
        (plain.mtime_secs, plain.mtime_nanos),
        (1_700_000_000, 123_456_789)
    );
    assert_eq!(plain.flags, libc::UF_HIDDEN);
    assert_eq!(
        root.stat(Path::new("link.m4a")).expect("stat").kind,
        Kind::Link
    );
    assert_eq!(
        root.stat(Path::new("sub")).expect("stat").kind,
        Kind::Directory
    );
    assert_eq!(
        root.regular(Path::new("plain.m4a")),
        Ok(root.path().join("plain.m4a"))
    );
    for leaf in ["link.m4a", "sub"] {
        assert_eq!(
            root.regular(Path::new(leaf)),
            Err(ContainedError::NotRegular(root.path().join(leaf)))
        );
    }
    assert!(matches!(
        root.stat(Path::new("absent")),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn open_file_refuses_a_link_and_a_directory_at_the_leaf() {
    let (_temp, root) = root();
    for access in [Access::Read, Access::Write, Access::ReadWrite] {
        for leaf in ["link.m4a", "sub"] {
            assert_eq!(
                root.open_file(Path::new(leaf), access).err(),
                Some(ContainedError::NotRegular(root.path().join(leaf)))
            );
        }
    }
    let mut file = root
        .open_file(Path::new("plain.m4a"), Access::Read)
        .expect("read");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("bytes");
    assert_eq!(bytes, b"bytes");
    assert!(file.write_all(b"no").is_err());
    let mut file = root
        .open_file(Path::new("plain.m4a"), Access::Write)
        .expect("write");
    file.write_all(b"W").expect("write byte");
    assert!(file.read(&mut [0]).is_err());
    let mut file = root
        .open_file(Path::new("plain.m4a"), Access::ReadWrite)
        .expect("read write");
    file.write_all(b"R").expect("write byte");
    file.read_exact(&mut [0]).expect("read byte");
    assert_eq!(
        fs::read(root.path().join("plain.m4a")).expect("updated"),
        b"Rytes"
    );
    assert_ne!(
        // SAFETY: the owned file descriptor remains open throughout fcntl.
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
}

#[test]
fn create_file_is_exclusive_and_sets_the_mode() {
    let (_temp, root) = root();
    let file = root
        .create_file(Path::new("new.m4a"), 0o600)
        .expect("created");
    assert_eq!(
        file.metadata().expect("meta").permissions().mode() & 0o777,
        0o600
    );
    assert!(matches!(
        root.create_file(Path::new("new.m4a"), 0o600),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        })
    ));
    assert!(matches!(
        root.create_file(Path::new("link.m4a"), 0o600),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        })
    ));
    assert_eq!(
        fs::read(root.path().join("plain.m4a")).expect("untouched"),
        b"bytes"
    );
}

#[test]
fn rename_exclusive_keeps_an_existing_target_and_rename_over_replaces_it() {
    let (_temp, root) = root();
    fs::write(root.path().join("a"), b"A").expect("a");
    fs::write(root.path().join("b"), b"B").expect("b");
    assert_eq!(
        root.rename_exclusive(Path::new("a"), Path::new("b")),
        Ok(false)
    );
    assert_eq!(fs::read(root.path().join("b")).expect("kept"), b"B");
    assert!(root.path().join("a").exists());
    assert_eq!(
        root.rename_exclusive(Path::new("a"), Path::new("c")),
        Ok(true)
    );
    assert!(!root.path().join("a").exists());
    assert_eq!(root.rename_over(Path::new("c"), Path::new("b")), Ok(()));
    assert_eq!(fs::read(root.path().join("b")).expect("replaced"), b"A");
    assert!(!root.path().join("c").exists());
    assert!(matches!(
        root.rename_over(Path::new("../x"), Path::new("b")),
        Err(ContainedError::Escape { .. })
    ));
    assert!(matches!(
        root.rename_over(Path::new("b"), Path::new("sub/x")),
        Err(ContainedError::Escape { .. })
    ));
    assert!(matches!(
        root.rename_exclusive(Path::new("missing"), Path::new("d")),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn file_operations_stay_below_the_retained_root_after_substitution() {
    let (temp, root) = root();
    let held = temp.path().join("sub");
    let directory = root.open_subdirectory("sub").expect("sub");
    fs::write(held.join("a"), b"A").expect("a");
    fs::rename(&held, temp.path().join("original")).expect("move");
    symlink(root.path(), &held).expect("replacement");
    let clone = directory.try_clone().expect("clone retained directory");
    assert_eq!(clone.stat(Path::new("a")).expect("cloned stat").size, 1);
    assert_eq!(directory.stat(Path::new("a")).expect("stat").size, 1);
    let mut file = directory
        .open_file(Path::new("a"), Access::Read)
        .expect("read retained");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("bytes");
    assert_eq!(bytes, b"A");
    directory
        .create_file(Path::new("new"), 0o600)
        .expect("create retained");
    directory
        .rename_over(Path::new("a"), Path::new("b"))
        .expect("rename retained");
    directory.subdirectory("child").expect("retained child");
    let child = directory
        .open_subdirectory("child")
        .expect("read retained child");
    child
        .create_file(Path::new("inside"), 0o600)
        .expect("retained grandchild");
    assert_eq!(
        fs::read(temp.path().join("original/b")).expect("renamed"),
        b"A"
    );
    assert!(temp.path().join("original/new").exists());
    assert!(temp.path().join("original/child/inside").exists());
    assert!(!root.path().join("child").exists());
    assert!(!root.path().join("new").exists());
    assert!(!root.path().join("b").exists());
}
