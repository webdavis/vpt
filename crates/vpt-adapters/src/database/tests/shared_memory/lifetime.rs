use super::{EXCLUSIVE, SHARED, UNLOCK_EXCLUSIVE, UNLOCK_SHARED, busy, files};
use crate::database::{
    AccessPolicy, FileRole,
    locks::{LockKind, set_record_lock},
    shared_memory::{MappedRegion, SharedMemory},
};
use crate::{Access, RootDir};
use std::path::Path;

pub(super) fn race(root: RootDir) {
    use std::sync::{Barrier, mpsc};
    let fd = files(&root, AccessPolicy::WritableLedger)
        .open(FileRole::SharedMemory, true)
        .unwrap();
    fd.set_len(65536).unwrap();
    let start = Barrier::new(3);
    let (send, receive) = mpsc::channel();
    let (closed_send, closed_receive) = mpsc::channel();
    std::thread::scope(|scope| {
        let mut release = Vec::new();
        for id in 0..2 {
            let root = &root;
            let start = &start;
            let send = send.clone();
            let closed = closed_send.clone();
            let (finish, wait) = mpsc::channel();
            release.push(finish);
            scope.spawn(move || {
                let files = files(root, AccessPolicy::WritableLedger);
                start.wait();
                let owned = SharedMemory::open(&files);
                match &owned {
                    Ok(_) => send.send((id, true)).unwrap(),
                    Err(crate::database::DatabaseError::Sqlite(
                        rusqlite::Error::SqliteFailure(e, _),
                    )) if e.code == rusqlite::ErrorCode::DatabaseBusy => {
                        send.send((id, false)).unwrap()
                    }
                    Err(error) => panic!("unexpected initialization failure: {error:?}"),
                }
                wait.recv().unwrap();
                drop(owned);
                closed.send(id).unwrap();
            });
        }
        start.wait();
        let mut opened = [false; 2];
        for _ in 0..2 {
            let (id, success) = receive.recv().unwrap();
            opened[id] = success;
        }
        assert!(opened.iter().any(|success| *success));
        assert_eq!(fd.metadata().unwrap().len(), 3);
        for (id, finish) in release.into_iter().enumerate() {
            if opened.iter().any(|success| *success) {
                busy(set_record_lock(&fd, 128, 1, LockKind::Exclusive));
            }
            finish.send(()).unwrap();
            assert_eq!(closed_receive.recv().unwrap(), id);
            opened[id] = false;
        }
    });
    set_record_lock(&fd, 128, 1, LockKind::Exclusive).unwrap();
}

pub(super) fn run(scenario: &str, root: RootDir) {
    let files = files(&root, AccessPolicy::WritableLedger);
    let fd = files.open(FileRole::SharedMemory, true).unwrap();
    fd.set_len(65536).unwrap();
    if scenario == "shm_busy_initializer" {
        set_record_lock(&fd, 128, 1, LockKind::Exclusive).unwrap();
        let result = SharedMemory::open(&files);
        assert!(
            matches!(result, Err(crate::database::DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::DatabaseBusy)
        );
        assert_eq!(fd.metadata().unwrap().len(), 65536);
        return;
    }
    if matches!(
        scenario,
        "shm_readonly_initialize" | "shm_uninitialized_map"
    ) {
        let ro = super::files(&root, AccessPolicy::ObservationalLedger);
        let mut reader = SharedMemory::open(&ro).unwrap();
        assert!(matches!(
            reader.map(0, 32768, false).unwrap(),
            MappedRegion::ReadOnlyUninitialized
        ));
        if scenario == "shm_uninitialized_map" {
            assert_eq!(fd.metadata().unwrap().len(), 65536);
            return;
        }
        reader.lock(3, 1, SHARED).unwrap();
        busy(set_record_lock(&fd, 123, 1, LockKind::Exclusive));
        let mut writer = SharedMemory::open(&files).unwrap();
        writer.map(0, 32768, true).unwrap();
        assert!(matches!(
            reader.map(0, 32768, false).unwrap(),
            MappedRegion::ReadOnly(_)
        ));
        reader.lock(3, 1, UNLOCK_SHARED).unwrap();
        drop(reader);
        drop(writer);
        return;
    }
    let mut first = SharedMemory::open(&files).unwrap();
    if scenario == "shm_invalid_slot_arguments" {
        for (offset, count, flags) in [
            (-1, 1, SHARED),
            (8, 1, SHARED),
            (0, 0, SHARED),
            (0, i32::MAX, EXCLUSIVE),
            (0, 2, SHARED),
            (0, 1, 0),
            (0, 1, 15),
        ] {
            assert!(matches!(
                first.lock(offset, count, flags),
                Err(crate::database::DatabaseError::Io {
                    operation: "shared memory lock",
                    kind: std::io::ErrorKind::InvalidInput
                })
            ));
        }
        set_record_lock(&fd, 120, 8, LockKind::Exclusive).unwrap();
        return;
    }
    if scenario == "shm_initialize" {
        assert_eq!(fd.metadata().unwrap().len(), 3);
        drop(first);
        set_record_lock(&fd, 128, 1, LockKind::Exclusive).unwrap();
        return;
    }
    fd.set_len(32768).unwrap();
    let mut second = SharedMemory::open(&files).unwrap();
    match scenario {
        "shm_lifetime" => {
            assert_eq!(fd.metadata().unwrap().len(), 32768);
            busy(set_record_lock(&fd, 128, 1, LockKind::Exclusive));
            drop(first);
            busy(set_record_lock(&fd, 128, 1, LockKind::Exclusive));
            drop(second);
            set_record_lock(&fd, 128, 1, LockKind::Exclusive).unwrap();
            return;
        }
        "shm_close" => {
            first.lock(0, 1, EXCLUSIVE).unwrap();
            let unrelated = root
                .open_file(Path::new("vpt.db-shm"), Access::ReadWrite)
                .unwrap();
            drop(unrelated);
            second.unmap(false).unwrap();
            busy(set_record_lock(&fd, 120, 1, LockKind::Exclusive));
            first.unmap(false).unwrap();
            set_record_lock(&fd, 120, 1, LockKind::Exclusive).unwrap();
            return;
        }
        "shm_slots" => {
            for slot in 0..8 {
                first.lock(slot, 1, SHARED).unwrap();
                second.lock(slot, 1, SHARED).unwrap();
                busy(set_record_lock(
                    &fd,
                    120 + i64::from(slot),
                    1,
                    LockKind::Exclusive,
                ));
                second.lock(slot, 1, UNLOCK_SHARED).unwrap();
                busy(second.lock(slot, 1, EXCLUSIVE));
                first.lock(slot, 1, UNLOCK_SHARED).unwrap();
                second.lock(slot, 1, EXCLUSIVE).unwrap();
                busy(first.lock(slot, 1, SHARED));
                second.lock(slot, 1, UNLOCK_EXCLUSIVE).unwrap();
            }
            first.lock(1, 7, EXCLUSIVE).unwrap();
            busy(second.lock(7, 1, SHARED));
            first.lock(1, 7, UNLOCK_EXCLUSIVE).unwrap();
        }
        "shm_invalid_locks" => {
            first.lock(3, 1, SHARED).unwrap();
            for (offset, count, flags) in [
                (-1, 1, SHARED),
                (8, 1, SHARED),
                (0, 0, SHARED),
                (0, i32::MAX, EXCLUSIVE),
                (0, 2, SHARED),
                (0, 1, 0),
                (0, 1, 15),
                (3, 1, EXCLUSIVE),
                (3, 1, UNLOCK_EXCLUSIVE),
            ] {
                assert!(matches!(
                    first.lock(offset, count, flags),
                    Err(crate::database::DatabaseError::Io {
                        kind: std::io::ErrorKind::InvalidInput,
                        ..
                    })
                ));
                busy(second.lock(3, 1, EXCLUSIVE));
            }
            first.lock(3, 1, SHARED).unwrap();
            first.lock(3, 1, UNLOCK_SHARED).unwrap();
        }
        _ => panic!("unknown lifetime fixture"),
    }
    drop(second);
    drop(first);
}
