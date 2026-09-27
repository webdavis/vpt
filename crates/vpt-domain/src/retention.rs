use crate::time::{FileTime, UtcInstant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    seconds: u64,
}

pub fn expired(mtime: FileTime, hold: Hold, now: UtcInstant) -> bool {
    let seconds = i128::from(now.secs) - i128::from(mtime.secs);
    let age_nanos = seconds * 1_000_000_000 - i128::from(mtime.nanos);
    let hold_nanos = i128::from(hold.seconds()) * 1_000_000_000;
    hold.seconds() != 0 && age_nanos >= hold_nanos
}

impl Hold {
    pub fn never() -> Self {
        Self { seconds: 0 }
    }

    pub fn of_seconds(seconds: u64) -> Self {
        Self { seconds }
    }

    pub fn seconds(self) -> u64 {
        self.seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: UtcInstant = UtcInstant { secs: 1_000_000 };

    #[test]
    fn a_hold_of_never_expires_nothing() {
        assert!(!expired(FileTime { secs: 0, nanos: 0 }, Hold::never(), NOW));
    }

    #[test]
    fn a_file_expires_at_the_hold_boundary_and_not_one_second_before() {
        let hold = Hold::of_seconds(86_400);
        assert!(!expired(
            FileTime {
                secs: NOW.secs - 86_399,
                nanos: 0
            },
            hold,
            NOW
        ));
        assert!(expired(
            FileTime {
                secs: NOW.secs - 86_400,
                nanos: 0
            },
            hold,
            NOW
        ));
        assert!(expired(FileTime { secs: 0, nanos: 0 }, hold, NOW));
    }

    #[test]
    fn a_file_from_the_future_never_expires() {
        assert!(!expired(
            FileTime {
                secs: NOW.secs + 10,
                nanos: 0
            },
            Hold::of_seconds(1),
            NOW
        ));
    }

    #[test]
    fn a_large_hold_does_not_wrap_and_nanoseconds_delay_the_boundary() {
        assert!(!expired(
            FileTime { secs: 0, nanos: 0 },
            Hold::of_seconds(u64::MAX),
            NOW
        ));
        let hold = Hold::of_seconds(1);
        assert!(!expired(
            FileTime {
                secs: NOW.secs - 1,
                nanos: 1
            },
            hold,
            NOW
        ));
        assert!(expired(
            FileTime {
                secs: NOW.secs - 1,
                nanos: 0
            },
            hold,
            NOW
        ));
        let oldest = FileTime {
            secs: i64::MIN,
            nanos: 0,
        };
        let latest = UtcInstant { secs: i64::MAX };
        assert!(expired(oldest, Hold::of_seconds(u64::MAX), latest));
        assert!(!expired(
            FileTime { nanos: 1, ..oldest },
            Hold::of_seconds(u64::MAX),
            latest
        ));
    }
}
