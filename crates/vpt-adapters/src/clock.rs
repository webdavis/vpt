use std::time::{SystemTime, UNIX_EPOCH};
use vpt_application::ports::Clock;
use vpt_domain::time::{UtcInstant, UtcOffset};

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UtcInstant {
        let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(value) => i64::try_from(value.as_secs()).unwrap_or(i64::MAX),
            Err(error) => -i64::try_from(error.duration().as_secs()).unwrap_or(i64::MAX),
        };
        UtcInstant { secs }
    }

    fn offset_at(&self, at: UtcInstant) -> UtcOffset {
        let time: libc::time_t = at.secs;
        let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
        // SAFETY: both pointers are valid for the call; local is read only after successful initialization.
        let offset = unsafe {
            if libc::localtime_r(&time, local.as_mut_ptr()).is_null() {
                0
            } else {
                local.assume_init().tm_gmtoff
            }
        };
        UtcOffset {
            secs: i32::try_from(offset).unwrap_or(0),
        }
    }
}
