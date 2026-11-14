use super::RootDir;
use std::os::fd::AsRawFd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameSupport {
    pub swap: bool,
    pub exclusive: bool,
    pub hard_links: bool,
}

impl RenameSupport {
    /// Whether a file can take a name only while that name is free, without copying its bytes.
    pub fn places_exclusively(self) -> bool {
        self.exclusive || self.hard_links
    }
}

#[repr(C)]
struct Reply {
    _length: u32,
    attribute: libc::vol_capabilities_attr_t,
}

impl RootDir {
    /// A volume that cannot be asked reports no support, so vpt refuses to publish notes there.
    pub fn rename_support(&self) -> RenameSupport {
        let mut request = libc::attrlist {
            bitmapcount: libc::ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: 0,
            volattr: libc::ATTR_VOL_INFO | libc::ATTR_VOL_CAPABILITIES,
            dirattr: 0,
            fileattr: 0,
            forkattr: 0,
        };
        let mut reply = Reply {
            _length: 0,
            attribute: libc::vol_capabilities_attr_t {
                capabilities: [0; 4],
                valid: [0; 4],
            },
        };
        // SAFETY: request and reply are live, writable locals and the size passed is reply's own;
        // every field of the reply is a plain integer, so any bytes the kernel writes are valid;
        // the retained descriptor outlives the call.
        let result = unsafe {
            libc::fgetattrlist(
                self.file.as_raw_fd(),
                (&raw mut request).cast(),
                (&raw mut reply).cast(),
                size_of::<Reply>(),
                0,
            )
        };
        let set = |index: usize, bit: libc::attrgroup_t| {
            let caps = &reply.attribute;
            result == 0 && caps.capabilities[index] & caps.valid[index] & bit != 0
        };
        RenameSupport {
            swap: set(
                libc::VOL_CAPABILITIES_INTERFACES,
                libc::VOL_CAP_INT_RENAME_SWAP,
            ),
            exclusive: set(
                libc::VOL_CAPABILITIES_INTERFACES,
                libc::VOL_CAP_INT_RENAME_EXCL,
            ),
            hard_links: set(libc::VOL_CAPABILITIES_FORMAT, libc::VOL_CAP_FMT_HARDLINKS),
        }
    }
}
