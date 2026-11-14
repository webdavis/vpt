use super::super::cloning::{byte_copy, clone_into_with};
use super::*;
use std::io;
use std::os::unix::fs::{PermissionsExt, symlink};

pub(super) fn native(f: &Fixture) {
    let source = f.source();
    fs::rename(
        f.recordings.join("a.m4a"),
        f.recordings.join("original.m4a"),
    )
    .unwrap();
    fs::write(f.recordings.join("a.m4a"), b"replacement").unwrap();
    f.unchanged(|| {
        assert_eq!(
            f.store
                .clone_into(&source, &f.audio, f.identity(), "clone.m4a"),
            Ok(CloneKind::CopyOnWrite)
        );
        assert_eq!(fs::read(f.audio.join("clone.m4a")).unwrap(), b"hello world");
    });
}

pub(super) fn identity(f: &Fixture) {
    let expected = f.identity();
    let saved = f.audio.with_extension("saved");
    fs::rename(&f.audio, &saved).unwrap();
    fs::create_dir(&f.audio).unwrap();
    let previous = snapshot(&saved);
    let replacement = snapshot(&f.audio);
    f.unchanged(|| {
        assert_eq!(
            f.store
                .clone_into(&f.source(), &f.audio, expected, "clone.m4a"),
            Err(CloneError::Escape(f.audio.clone()))
        );
        assert_eq!(
            clone_into_with(
                &f.source(),
                &f.audio,
                expected,
                "copy.m4a",
                |_, _, _| panic!("clone called before identity guard")
            ),
            Err(CloneError::Escape(f.audio.clone()))
        );
    });
    assert_eq!(snapshot(&saved), previous);
    assert_eq!(snapshot(&f.audio), replacement);
}

pub(super) fn names(f: &Fixture) {
    let linked = f.audio.with_extension("link");
    symlink(&f.audio, &linked).unwrap();
    let parent_link = f.audio.with_extension("parent-link");
    symlink(f.audio.parent().unwrap(), &parent_link).unwrap();
    let before = snapshot(&f.audio);
    f.unchanged(|| {
        let source = f.source();
        for name in ["", ".", "..", "../escape.m4a", "child/name.m4a"] {
            assert!(
                matches!(
                    f.store.clone_into(&source, &f.audio, f.identity(), name),
                    Err(CloneError::Escape(_))
                ),
                "{name:?}"
            );
            assert!(matches!(
                clone_into_with(&source, &f.audio, f.identity(), name, |_, _, _| panic!(
                    "clone called before name guard"
                )),
                Err(CloneError::Escape(_))
            ));
        }
        let outside = f.recordings.join("escaped.m4a");
        assert!(matches!(
            f.store
                .clone_into(&source, &f.audio, f.identity(), outside.to_str().unwrap()),
            Err(CloneError::Escape(_))
        ));
        assert_eq!(
            f.store
                .clone_into(&source, &f.audio, f.identity(), "nul\0name"),
            Err(CloneError::Io(io::ErrorKind::InvalidInput.to_string()))
        );
        for path in [&linked, &parent_link.join("audio")] {
            assert_eq!(
                f.store
                    .clone_into(&source, path, f.identity(), "linked.m4a"),
                Err(CloneError::Escape(path.clone()))
            );
        }
    });
    assert_eq!(snapshot(&f.audio), before);
}

pub(super) fn collisions(f: &Fixture) {
    prepare_collisions(f);
    let destination = snapshot(&f.audio);
    f.unchanged(|| {
        let source = f.source();
        for name in ["existing.m4a", "link.m4a", "directory.m4a", "pipe.m4a"] {
            assert_eq!(
                f.store.clone_into(&source, &f.audio, f.identity(), name),
                Err(CloneError::Exists),
                "{name}"
            );
        }
    });
    assert_eq!(snapshot(&f.audio), destination);
}

pub(super) fn exdev(f: &Fixture) {
    let bytes: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
    fs::write(f.recordings.join("a.m4a"), &bytes).unwrap();
    let source = f.source();
    fs::rename(f.recordings.join("a.m4a"), f.recordings.join("old.m4a")).unwrap();
    fs::write(f.recordings.join("a.m4a"), b"replacement").unwrap();
    let mut calls = 0;
    f.unchanged(|| {
        assert_eq!(
            clone_into_with(
                &source,
                &f.audio,
                f.identity(),
                "copy.m4a",
                |_, directory, name| {
                    calls += 1;
                    assert_eq!(directory.identity().unwrap(), f.identity());
                    assert_eq!(name, f.audio.join("copy.m4a"));
                    Err(io::Error::from_raw_os_error(libc::EXDEV))
                }
            ),
            Ok(CloneKind::ByteCopy)
        );
    });
    assert_eq!(calls, 1);
    assert_eq!(fs::read(f.audio.join("copy.m4a")).unwrap(), bytes);
    assert_eq!(
        fs::metadata(f.audio.join("copy.m4a"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

pub(super) fn errors(f: &Fixture) {
    let source = f.source();
    let destination = snapshot(&f.audio);
    f.unchanged(|| {
        for code in [
            libc::ENOSPC,
            libc::EEXIST,
            libc::EIO,
            libc::EACCES,
            libc::ENOTSUP,
            libc::ENOSYS,
            libc::EINVAL,
        ] {
            let expected = match code {
                libc::ENOSPC => CloneError::NoSpace,
                libc::EEXIST => CloneError::Exists,
                _ => CloneError::Io(io::Error::from_raw_os_error(code).kind().to_string()),
            };
            let mut called = false;
            let outcome =
                clone_into_with(&source, &f.audio, f.identity(), "failed.m4a", |_, _, _| {
                    called = true;
                    Err(io::Error::from_raw_os_error(code))
                });
            assert!(called, "native boundary was not called");
            assert_eq!(outcome, Err(expected), "errno {code}");
            assert_eq!(
                snapshot(&f.audio),
                destination,
                "unexpected fallback or output"
            );
        }
    });
}

pub(super) fn prepare_collisions(f: &Fixture) {
    fs::write(f.audio.join("existing.m4a"), b"existing owner's bytes").unwrap();
    symlink(f.recordings.join("a.m4a"), f.audio.join("link.m4a")).unwrap();
    fs::create_dir(f.audio.join("directory.m4a")).unwrap();
    listing::fifo(&f.audio.join("pipe.m4a"));
}

pub(super) fn fallback_collision(f: &Fixture, name: &str) {
    let source = f.source();
    let directory = RootDir::open(&f.audio).unwrap();
    assert_eq!(
        byte_copy(&source, &directory, Path::new(name)),
        Err(CloneError::Exists)
    );
    assert_eq!(
        clone_into_with(&source, &f.audio, f.identity(), name, |_, _, _| Err(
            io::Error::from_raw_os_error(libc::EXDEV)
        )),
        Err(CloneError::Exists)
    );
}
