use super::*;
use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::os::macos::fs::MetadataExt as _;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, symlink};
use vpt_domain::time::FileTime;

pub(super) fn depth_one(f: &Fixture) {
    for name in ["z.m4a", "b.M4A", "a.waveform", "notes.txt"] {
        fs::write(f.recordings.join(name), b"owned fixture").unwrap();
    }
    fs::create_dir(f.recordings.join("directory.m4a")).unwrap();
    symlink("a.m4a", f.recordings.join("link.m4a")).unwrap();
    fifo(&f.recordings.join("pipe.m4a"));
    f.unchanged(|| {
        let candidates = f.store.candidates().expect("list");
        let names: Vec<_> = candidates
            .iter()
            .map(|entry| entry.file_name.as_str())
            .collect();
        assert_eq!(names, ["a.m4a", "z.m4a"]);
        assert!(
            candidates
                .iter()
                .all(|entry| entry.path.parent() == Some(&f.recordings))
        );
    });
}

pub(super) fn flags(f: &Fixture) {
    let file = f.source();
    let flags = file.metadata().unwrap().st_flags() | libc::UF_NODUMP;
    // SAFETY: file owns this fixture descriptor; UF_NODUMP does not inhibit cleanup.
    assert_eq!(unsafe { libc::fchflags(file.as_raw_fd(), flags) }, 0);
    let metadata = file.metadata().unwrap();
    assert_ne!(metadata.st_flags() & libc::UF_NODUMP, 0);
    f.unchanged(|| {
        let candidate = f.store.candidates().unwrap().remove(0);
        assert_eq!(
            candidate,
            Candidate {
                path: f.recordings.join("a.m4a"),
                file_name: "a.m4a".into(),
                size: metadata.len(),
                mtime: FileTime {
                    secs: metadata.mtime(),
                    nanos: metadata.mtime_nsec().try_into().unwrap()
                },
                flags: metadata.st_flags(),
            }
        );
        assert_eq!(f.store.candidate(&candidate.path), Ok(candidate));
    });
}

pub(super) fn candidate_guards(f: &Fixture) {
    symlink("a.m4a", f.recordings.join("link.m4a")).unwrap();
    fs::create_dir(f.recordings.join("directory.m4a")).unwrap();
    f.unchanged(|| {
        for name in ["absent.m4a", "directory.m4a", "Capture", "metadata.db"] {
            let path = f.recordings.join(name);
            assert_eq!(f.store.candidate(&path), Err(RecorderError::NotFound(path)));
        }
        let link = f.recordings.join("link.m4a");
        assert_eq!(
            f.store.candidate(&link),
            Err(RecorderError::NotRegular(link))
        );
        for path in [
            f.audio.join("a.m4a"),
            f.recordings.join("Capture/inner.m4a"),
            PathBuf::from("../a.m4a"),
        ] {
            assert_eq!(f.store.candidate(&path), Err(RecorderError::Escape(path)));
        }
    });
}

pub(super) fn counts(f: &Fixture) {
    fs::create_dir(f.recordings.join("Capture/deeper")).unwrap();
    fs::write(f.recordings.join("Capture/deeper/extra"), b"not counted").unwrap();
    fs::rename(
        f.recordings.join("CaptureRecovery"),
        f.recordings.join("old-recovery"),
    )
    .unwrap();
    symlink(&f.audio, f.recordings.join("CaptureRecovery")).unwrap();
    fs::write(f.audio.join("outside"), b"do not follow").unwrap();
    fs::rename(
        f.recordings.join("CloudRecordings_ckAssets"),
        f.recordings.join("old-cloud"),
    )
    .unwrap();
    let outside = snapshot(&f.audio);
    f.unchanged(|| {
        assert_eq!(
            f.store.subdirectory_counts(),
            vec![
                ("Capture".into(), Some(2)),
                ("CaptureRecovery".into(), None),
                ("CloudRecordings_ckAssets".into(), None),
                ("EncryptedCloudRecordings".into(), Some(1)),
            ]
        );
        assert_eq!(f.store.candidates().unwrap().len(), 1);
    });
    assert_eq!(snapshot(&f.audio), outside);
}

pub(super) fn root_guards(f: &Fixture) {
    let absent = f.audio.join("absent");
    assert!(matches!(
        VoiceMemosStore::open(&absent),
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
    let linked = f.audio.join("linked");
    symlink(&f.recordings, &linked).unwrap();
    assert!(matches!(
        VoiceMemosStore::open(&linked),
        Err(ContainedError::NotADirectory(_))
    ));
    let saved = f.recordings.with_extension("saved");
    fs::rename(&f.recordings, &saved).unwrap();
    fs::create_dir(&f.recordings).unwrap();
    fs::write(f.recordings.join("replacement.m4a"), b"other tree").unwrap();
    let old = snapshot(&saved);
    f.unchanged(|| {
        assert_eq!(
            f.store.candidates(),
            Err(RecorderError::Escape(f.recordings.clone()))
        );
    });
    assert_eq!(snapshot(&saved), old);
}

pub(super) fn fifo(path: &Path) {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: name identifies a nonexistent FIFO in this child-owned fixture.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
}
