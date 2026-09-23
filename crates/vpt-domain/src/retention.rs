#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    seconds: u64,
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
