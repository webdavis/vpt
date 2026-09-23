use super::super::{DatabaseError, invalid, io_error, sqlite_error};
use super::run;
use rusqlite::ffi::*;
use std::ffi::c_void;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;

fn range(length: i32, offset: i64) -> Result<(usize, u64), DatabaseError> {
    if length < 0 || offset < 0 || offset.checked_add(i64::from(length)).is_none() {
        return Err(invalid("file range"));
    }
    Ok((length as usize, offset as u64))
}

pub(super) unsafe extern "C" fn read(
    file: *mut sqlite3_file,
    buffer: *mut c_void,
    length: i32,
    offset: i64,
) -> i32 {
    // SAFETY: run validates file state; SQLite owns length writable bytes at buffer.
    unsafe {
        run(file, SQLITE_IOERR_READ, |file, _| {
            let (length, mut offset) = range(length, offset)?;
            if buffer.is_null() {
                return Err(invalid("read buffer"));
            }
            let mut bytes = std::slice::from_raw_parts_mut(buffer.cast::<u8>(), length);
            while !bytes.is_empty() {
                match file.locks.file().read_at(bytes, offset) {
                    Ok(0) => {
                        bytes.fill(0);
                        return Ok(SQLITE_IOERR_SHORT_READ);
                    }
                    Ok(count) => {
                        offset += count as u64;
                        bytes = &mut bytes[count..];
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(io_error("read", error)),
                }
            }
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn write(
    file: *mut sqlite3_file,
    buffer: *const c_void,
    length: i32,
    offset: i64,
) -> i32 {
    // SAFETY: run validates file state; SQLite owns length readable bytes at buffer.
    unsafe {
        run(file, SQLITE_IOERR_WRITE, |file, state| {
            let (length, mut offset) = range(length, offset)?;
            if buffer.is_null() {
                return Err(invalid("write buffer"));
            }
            if !file.writable {
                return Err(sqlite_error(SQLITE_READONLY));
            }
            let prefix = state.files.faults.write_prefix()?;
            let mut bytes = std::slice::from_raw_parts(
                buffer.cast::<u8>(),
                prefix.map_or(length, |prefix| prefix.min(length)),
            );
            while !bytes.is_empty() {
                match file.locks.file().write_at(bytes, offset) {
                    Ok(0) => return Err(io_error("write", ErrorKind::WriteZero.into())),
                    Ok(count) => {
                        offset += count as u64;
                        bytes = &bytes[count..];
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(io_error("write", error)),
                }
            }
            if prefix.is_some() {
                return Err(io_error(
                    "write",
                    std::io::Error::from_raw_os_error(libc::EIO),
                ));
            }
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn truncate(file: *mut sqlite3_file, length: i64) -> i32 {
    // SAFETY: run validates the initialized file state.
    unsafe {
        run(file, SQLITE_IOERR_TRUNCATE, |file, state| {
            let length = u64::try_from(length).map_err(|_| invalid("truncate length"))?;
            if !file.writable {
                return Err(sqlite_error(SQLITE_READONLY));
            }
            state.files.faults.check("truncate")?;
            file.locks
                .file()
                .set_len(length)
                .map_err(|error| io_error("truncate", error))?;
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn size(file: *mut sqlite3_file, output: *mut i64) -> i32 {
    // SAFETY: run validates file state; SQLite supplies a writable i64 output.
    unsafe {
        run(file, SQLITE_IOERR_FSTAT, |file, _| {
            if output.is_null() {
                return Err(invalid("size output"));
            }
            let metadata = file
                .locks
                .file()
                .metadata()
                .map_err(|error| io_error("file metadata", error))?;
            *output = i64::try_from(metadata.len()).map_err(|_| invalid("file size"))?;
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn sync(file: *mut sqlite3_file, flags: i32) -> i32 {
    // SAFETY: run validates file state; fcntl receives its retained descriptor.
    unsafe {
        run(file, SQLITE_IOERR_FSYNC, |file, state| {
            if !matches!(
                flags & !SQLITE_SYNC_DATAONLY,
                SQLITE_SYNC_NORMAL | SQLITE_SYNC_FULL
            ) {
                return Err(invalid("sync flags"));
            }
            if !file.writable {
                return Err(sqlite_error(SQLITE_READONLY));
            }
            let synchronization = state.files.faults.check("file sync").and_then(|()| {
                let file = file.locks.file();
                let result = if flags & 0x0f == SQLITE_SYNC_FULL {
                    if libc::fcntl(file.as_raw_fd(), libc::F_FULLFSYNC) == -1 {
                        Err(std::io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                } else if flags & SQLITE_SYNC_DATAONLY != 0 {
                    file.sync_data()
                } else {
                    file.sync_all()
                };
                result.map_err(|error| io_error("file sync", error))
            });
            synchronization?;
            Ok(SQLITE_OK)
        })
    }
}
