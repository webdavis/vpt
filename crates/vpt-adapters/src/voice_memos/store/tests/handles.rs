use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, symlink};
use vpt_domain::time::FileTime;
use vpt_domain::{container, fixtures};

pub(super) fn open_guards(f: &Fixture) {
    symlink("a.m4a", f.recordings.join("link.m4a")).unwrap();
    fs::create_dir(f.recordings.join("directory.m4a")).unwrap();
    listing::fifo(&f.recordings.join("pipe.m4a"));
    f.unchanged(|| {
        for name in ["link.m4a", "directory.m4a", "pipe.m4a"] {
            let path = f.recordings.join(name);
            assert_eq!(
                f.store.open(&path).err(),
                Some(RecorderError::NotRegular(path))
            );
        }
        let absent = f.recordings.join("missing.m4a");
        assert_eq!(
            f.store.open(&absent).err(),
            Some(RecorderError::NotFound(absent))
        );
        for path in [
            f.audio.join("outside"),
            f.recordings.join("Capture/inner.m4a"),
            PathBuf::from("../escape"),
        ] {
            assert_eq!(f.store.open(&path).err(), Some(RecorderError::Escape(path)));
        }
        let handle = f
            .store
            .open(&f.recordings.join("a.m4a"))
            .expect("regular source");
        // SAFETY: handle retains this descriptor throughout the read-only flag queries.
        let (access, descriptor) = unsafe {
            (
                libc::fcntl(handle.as_raw_fd(), libc::F_GETFL),
                libc::fcntl(handle.as_raw_fd(), libc::F_GETFD),
            )
        };
        assert!(access >= 0 && descriptor >= 0);
        assert_eq!(access & libc::O_ACCMODE, libc::O_RDONLY);
        assert_ne!(descriptor & libc::FD_CLOEXEC, 0);
    });
}

pub(super) fn descriptor(f: &Fixture) {
    let bytes = fixtures::m4a(1_787_604_456, 3, b"owned payload");
    fs::write(f.recordings.join("a.m4a"), &bytes).unwrap();
    let handle = f.source();
    let expected = handle.metadata().unwrap();
    fs::rename(
        f.recordings.join("a.m4a"),
        f.recordings.join("original.m4a"),
    )
    .unwrap();
    fs::write(f.recordings.join("a.m4a"), b"replacement bytes").unwrap();
    f.unchanged(|| {
        let metadata = f.store.metadata(&handle).unwrap();
        assert_eq!(
            metadata,
            SourceMetadata {
                device: expected.dev(),
                inode: expected.ino(),
                size: expected.len(),
                mtime: FileTime {
                    secs: expected.mtime(),
                    nanos: expected.mtime_nsec().try_into().unwrap()
                },
            }
        );
        let mut read = vec![0; bytes.len()];
        f.store.read_at(&handle, 0, &mut read).unwrap();
        assert_eq!(read, bytes);
        let container = container::inspect(metadata.size, |offset, buffer| {
            f.store.read_at(&handle, offset, buffer)
        })
        .unwrap();
        assert_eq!(container.creation_time.secs, 1_787_604_456);
        assert_eq!(container.duration_secs, 3);
    });
}

pub(super) fn reads(f: &Fixture) {
    let handle = f.source();
    f.unchanged(|| {
        let mut buffer = [0; 5];
        f.store.read_at(&handle, 6, &mut buffer).unwrap();
        assert_eq!(&buffer, b"world");
        f.store.read_at(&handle, 0, &mut buffer).unwrap();
        assert_eq!(&buffer, b"hello");
        assert_eq!(f.store.read_at(&handle, 7, &mut buffer), Err(ReadFailure));
        assert_eq!(
            f.store.read_at(&handle, u64::MAX, &mut buffer),
            Err(ReadFailure)
        );
        assert_eq!(f.store.read_at(&handle, 11, &mut []), Ok(()));
    });
}
