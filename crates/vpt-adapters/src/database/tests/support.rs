use super::super::FileRole;
use crate::RootDir;
use std::ffi::{CStr, CString, OsStr};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(super) const ROLES: [(FileRole, &str); 4] = [
    (FileRole::Database, "vpt.db"),
    (FileRole::Journal, "vpt.db-journal"),
    (FileRole::Wal, "vpt.db-wal"),
    (FileRole::SharedMemory, "vpt.db-shm"),
];

pub(crate) fn root() -> (tempfile::TempDir, RootDir) {
    let temp = crate::test_temp::tempdir().expect("owned temp");
    let path = temp
        .path()
        .canonicalize()
        .expect("canonical temp")
        .join("state");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .expect("state");
    let root = RootDir::open(&path).expect("checked root");
    (temp, root)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    path: PathBuf,
    metadata: [u64; 12],
    bytes: Vec<u8>,
}

impl Snapshot {
    pub(crate) fn ignore_mapping_release_time(&mut self, mapped: &Path) {
        if self.path == mapped {
            // macOS updates ctime when the final mapping closes after prior mapped writes.
            self.metadata[9..11].fill(0);
        }
    }
}

pub(crate) fn snapshot(path: &Path) -> Vec<Snapshot> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path.parent().expect("owned snapshot parent"))
        .expect("owned snapshot parent");
    let mut result = Vec::new();
    let name = CString::new(path.file_name().unwrap().as_bytes()).unwrap();
    child(&file, &name, path, &mut result);
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

fn metadata(file: &File) -> libc::stat {
    let mut stat = std::mem::MaybeUninit::uninit();
    assert_eq!(
        // SAFETY: stat is writable output and file retains the descriptor.
        unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) },
        0
    );
    // SAFETY: successful fstat initialized every stat field.
    unsafe { stat.assume_init() }
}

fn entry(path: &Path, stat: libc::stat, bytes: Vec<u8>) -> Snapshot {
    Snapshot {
        path: path.into(),
        metadata: [
            u64::from(stat.st_mode),
            u64::from(stat.st_uid),
            u64::from(stat.st_gid),
            stat.st_dev as u64,
            stat.st_ino,
            u64::from(stat.st_nlink),
            stat.st_size as u64,
            stat.st_mtime as u64,
            stat.st_mtime_nsec as u64,
            stat.st_ctime as u64,
            stat.st_ctime_nsec as u64,
            u64::from(stat.st_flags),
        ],
        bytes,
    }
}

fn directory(file: &File, path: &Path, result: &mut Vec<Snapshot>) {
    result.push(entry(path, metadata(file), Vec::new()));
    let descriptor = file.try_clone().unwrap();
    // SAFETY: fdopendir takes the newly duplicated descriptor on success.
    let stream = unsafe { libc::fdopendir(descriptor.as_raw_fd()) };
    assert!(!stream.is_null());
    let _ = descriptor.into_raw_fd();
    let stream = Directory(stream);
    loop {
        // SAFETY: stream owns this DIR; errno distinguishes exhaustion from an error.
        let item = unsafe {
            *libc::__error() = 0;
            libc::readdir(stream.0)
        };
        if item.is_null() {
            assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(0));
            break;
        }
        // SAFETY: the entry name remains live until the next readdir; it is copied here.
        let name = unsafe { CStr::from_ptr((*item).d_name.as_ptr()) }.to_owned();
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        child(
            file,
            &name,
            &path.join(OsStr::from_bytes(name.to_bytes())),
            result,
        );
    }
}

fn child(directory: &File, name: &CString, path: &Path, result: &mut Vec<Snapshot>) {
    let mut stat = std::mem::MaybeUninit::uninit();
    assert_eq!(
        // SAFETY: the retained directory anchors this no-follow metadata lookup.
        unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        },
        0
    );
    // SAFETY: successful fstatat initialized stat.
    let stat = unsafe { stat.assume_init() };
    let kind = stat.st_mode & libc::S_IFMT;
    if kind == libc::S_IFLNK {
        let mut bytes = vec![0; usize::try_from(stat.st_size).unwrap() + 1];
        // SAFETY: readlinkat writes only to the owned buffer and never follows the entry.
        let length = unsafe {
            libc::readlinkat(
                directory.as_raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        assert!(length >= 0 && (length as usize) < bytes.len());
        bytes.truncate(length as usize);
        result.push(entry(path, stat, bytes));
    } else if matches!(kind, libc::S_IFREG | libc::S_IFDIR) {
        // SAFETY: no-follow and nonblocking acquisition prevent redirection and FIFO blocking.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        assert!(fd >= 0);
        // SAFETY: successful openat transferred this fresh descriptor.
        let mut file = unsafe { File::from_raw_fd(fd) };
        if path.as_os_str().as_bytes().ends_with(b"-shm") {
            write_back(&file);
        }
        let opened = metadata(&file);
        assert_eq!(
            (opened.st_dev, opened.st_ino, opened.st_mode),
            (stat.st_dev, stat.st_ino, stat.st_mode)
        );
        if kind == libc::S_IFDIR {
            self::directory(&file, path, result);
        } else {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            result.push(entry(path, opened, bytes));
        }
    } else {
        result.push(entry(path, stat, Vec::new()));
    }
}

/// Writes back the WAL index pages a writer left dirty through its shared mapping. The kernel
/// otherwise writes them back seconds later and changes the file's times after the snapshot.
fn write_back(file: &File) {
    // SAFETY: fsync receives the descriptor this snapshot owns.
    let result = unsafe { libc::fsync(file.as_raw_fd()) };
    assert_eq!(result, 0, "{}", std::io::Error::last_os_error());
}

struct Directory(*mut libc::DIR);

impl Drop for Directory {
    fn drop(&mut self) {
        // SAFETY: this guard exclusively owns the directory stream and its descriptor.
        assert_eq!(unsafe { libc::closedir(self.0) }, 0);
    }
}
