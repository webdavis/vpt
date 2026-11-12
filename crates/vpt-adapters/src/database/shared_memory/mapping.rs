use super::{DatabaseError, SHM_REGION_BYTES, io_error};
use crate::database::invalid;
use std::{ffi::c_void, fs::File, os::fd::AsRawFd, ptr::NonNull};

pub(super) struct Region {
    pub(super) index: i32,
    base: NonNull<c_void>,
    length: usize,
    offset: usize,
}

impl Region {
    pub(super) fn map(file: &File, index: i32, writable: bool) -> Result<Self, DatabaseError> {
        let position = i64::from(index)
            .checked_mul(SHM_REGION_BYTES as i64)
            .ok_or_else(|| invalid("mapping offset"))?;
        // SAFETY: sysconf takes no pointer and reads the host's page-size constant.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return Err(invalid("host page size"));
        }
        let aligned = position / page_size * page_size;
        let offset =
            usize::try_from(position - aligned).map_err(|_| invalid("mapping alignment"))?;
        let length = offset
            .checked_add(SHM_REGION_BYTES)
            .ok_or_else(|| invalid("mapping length"))?;
        // SAFETY: the retained regular file is provisioned through this region; aligned is page-aligned.
        let protection = libc::PROT_READ | if writable { libc::PROT_WRITE } else { 0 };
        let pointer = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                protection,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                aligned,
            )
        };
        if pointer == libc::MAP_FAILED {
            return Err(io_error("map", std::io::Error::last_os_error()));
        }
        let Some(base) = NonNull::new(pointer) else {
            // SAFETY: release the successful zero-address mapping before refusing a null SQLite pointer.
            unsafe {
                libc::munmap(pointer, length);
            }
            return Err(invalid("mapping address"));
        };
        // SAFETY: base and length name the mapping created above.
        if unsafe { libc::minherit(base.as_ptr(), length, libc::VM_INHERIT_NONE) } != 0 {
            let error = std::io::Error::last_os_error();
            // SAFETY: release the mapping before refusing one a forked child would inherit.
            unsafe {
                libc::munmap(base.as_ptr(), length);
            }
            return Err(io_error("map inheritance", error));
        }
        Ok(Self {
            index,
            base,
            length,
            offset,
        })
    }

    pub(super) fn address(&self) -> NonNull<c_void> {
        // SAFETY: the nonnegative offset selects the region inside this still-owned mapping.
        unsafe { NonNull::new_unchecked(self.base.as_ptr().byte_add(self.offset)) }
    }

    pub(super) fn unmap(self) -> Result<(), DatabaseError> {
        // SAFETY: this method consumes the unique owner of the original mapping range.
        if unsafe { libc::munmap(self.base.as_ptr(), self.length) } != 0 {
            return Err(io_error("unmap", std::io::Error::last_os_error()));
        }
        std::mem::forget(self);
        Ok(())
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: this owner releases the original base and length once, after SQLite releases its use.
        unsafe {
            libc::munmap(self.base.as_ptr(), self.length);
        }
    }
}
