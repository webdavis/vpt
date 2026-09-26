use super::{EXCLUSIVE, files};
use crate::RootDir;
use crate::database::{
    AccessPolicy, FileRole,
    faults::Fault,
    locks::{LockKind, set_record_lock},
    shared_memory::{MappedRegion, SharedMemory},
};

pub(super) fn run(scenario: &str, root: RootDir) {
    let files = files(&root, AccessPolicy::WritableLedger);
    let mut first = SharedMemory::open(&files).unwrap();
    let fd = files.open(FileRole::SharedMemory, false).unwrap();
    match scenario {
        "shm_missing" => {
            assert!(matches!(
                first.map(0, 32768, false).unwrap(),
                MappedRegion::Missing
            ));
            assert_eq!(fd.metadata().unwrap().len(), 3);
        }
        "shm_regions" => {
            let mut second = SharedMemory::open(&files).unwrap();
            for index in [0, 1, 3] {
                let MappedRegion::Writable(p) = first.map(index, 32768, true).unwrap() else {
                    panic!("writable map")
                };
                let MappedRegion::Writable(q) = second.map(index, 32768, false).unwrap() else {
                    panic!("shared map")
                };
                // SAFETY: both owners retain the full mapped regions during this access.
                unsafe {
                    p.cast::<u8>()
                        .as_ptr()
                        .add(32767)
                        .write_volatile(index as u8 + 10);
                    assert_eq!(
                        q.cast::<u8>().as_ptr().add(32767).read_volatile(),
                        index as u8 + 10
                    );
                }
            }
            assert_eq!(fd.metadata().unwrap().len(), 131072);
            drop(second);
        }
        "shm_invalid_map" => {
            for (index, size) in [
                (-1, 32768),
                (0, -1),
                (0, 0),
                (0, 4096),
                (0, 32767),
                (0, 32769),
            ] {
                assert!(matches!(
                    first.map(index, size, true),
                    Err(crate::database::DatabaseError::Io {
                        operation: "shared memory region",
                        kind: std::io::ErrorKind::InvalidInput
                    })
                ));
            }
            assert_eq!(fd.metadata().unwrap().len(), 3);
        }
        "shm_map_failure" => {
            first.map(0, 32768, true).unwrap();
            first.lock(0, 1, EXCLUSIVE).unwrap();
            files.faults.arm(Fault::Map);
            assert!(first.map(1, 32768, true).is_err());
            assert!(files.faults.consumed.get());
            first.unmap(false).unwrap();
            set_record_lock(&fd, 120, 9, LockKind::Exclusive).unwrap();
            set_record_lock(&fd, 120, 9, LockKind::Unlock).unwrap();
            let mut next = SharedMemory::open(&files).unwrap();
            next.map(0, 32768, true).unwrap();
            next.unmap(false).unwrap();
            return;
        }
        "shm_readonly_map" | "shm_readonly_policy" | "shm_readonly_extension" => {
            first.map(0, 32768, true).unwrap();
            let before = super::super::support::snapshot(root.path());
            let mut readonly =
                SharedMemory::open(&super::files(&root, AccessPolicy::ObservationalLedger))
                    .unwrap();
            assert!(matches!(
                readonly.map(0, 32768, false).unwrap(),
                MappedRegion::ReadOnly(_)
            ));
            if scenario == "shm_readonly_policy" {
                drop(readonly);
                assert_eq!(super::super::support::snapshot(root.path()), before);
                return;
            }
            assert!(
                matches!(readonly.map(1, 32768, true), Err(crate::database::DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::ReadOnly)
            );
            if scenario == "shm_readonly_extension" {
                drop(readonly);
                assert_eq!(super::super::support::snapshot(root.path()), before);
                return;
            }
            readonly.unmap(true).unwrap();
            assert_eq!(super::super::support::snapshot(root.path()), before);
        }
        "shm_root_switch" => {
            let held = root.path().with_file_name("held");
            std::fs::rename(root.path(), &held).unwrap();
            std::fs::create_dir(root.path()).unwrap();
            std::fs::write(root.path().join("vpt.db-shm"), b"replacement").unwrap();
            first.map(0, 32768, true).unwrap();
            first.unmap(true).unwrap();
            assert!(!held.join("vpt.db-shm").exists());
            assert_eq!(
                std::fs::read(root.path().join("vpt.db-shm")).unwrap(),
                b"replacement"
            );
            return;
        }
        "shm_delete_active" => {
            let second = SharedMemory::open(&files).unwrap();
            first.unmap(true).unwrap();
            assert!(root.path().join("vpt.db-shm").exists());
            second.unmap(true).unwrap();
            assert!(!root.path().join("vpt.db-shm").exists());
            return;
        }
        "shm_protection" => {
            first.map(0, 32768, true).unwrap();
            let mut readonly =
                SharedMemory::open(&super::files(&root, AccessPolicy::ObservationalLedger))
                    .unwrap();
            let MappedRegion::ReadOnly(p) = readonly.map(0, 32768, false).unwrap() else {
                panic!("readonly map")
            };
            super::super::children::scenarios::ready();
            // SAFETY: deliberate protection probe is confined to this core-disabled supervised child.
            unsafe {
                p.cast::<u8>().as_ptr().write_volatile(42);
            }
            panic!("observational mapping accepted a write");
        }
        _ => panic!("unknown map fixture"),
    }
    drop(first);
}
