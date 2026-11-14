use super::files;
use crate::database::{
    AccessPolicy,
    shared_memory::{MappedRegion, SharedMemory},
};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

#[test]
fn a_forked_child_cannot_change_shared_memory_after_the_owner_unmaps() {
    let (_temp, root) = super::super::support::root();
    let files = files(&root, AccessPolicy::WritableLedger);
    let mut shared = SharedMemory::open(&files).unwrap();
    assert!(matches!(
        shared.map(0, 32768, true).unwrap(),
        MappedRegion::Writable(_)
    ));
    let (reader, writer) = std::io::pipe().unwrap();
    // SAFETY: the child calls only close, read and _exit before it ends.
    let child = unsafe { libc::fork() };
    if child == 0 {
        // SAFETY: these calls are async-signal-safe and touch only the inherited pipe.
        unsafe {
            libc::close(writer.as_raw_fd());
            let mut byte = 0u8;
            libc::read(reader.as_raw_fd(), (&raw mut byte).cast(), 1);
            libc::_exit(0);
        }
    }
    assert!(child > 0, "fork failed");
    shared.unmap(false).unwrap();
    let ctime = || {
        let metadata = std::fs::metadata(root.path().join("vpt.db-shm")).unwrap();
        (metadata.ctime(), metadata.ctime_nsec())
    };
    let settled = ctime();
    drop(writer);
    assert_eq!(
        // SAFETY: child is this test's own unreaped process.
        unsafe { libc::waitpid(child, std::ptr::null_mut(), 0) },
        child
    );
    assert_eq!(ctime(), settled);
}
