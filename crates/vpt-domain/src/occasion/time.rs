mod parse;
use crate::time::UtcInstant;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccasionTime {
    text: String,
    seconds: i64,
    leap: bool,
    fraction: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    Invalid,
    CapacityExceeded,
}

impl OccasionTime {
    pub fn parse(text: &str) -> Result<Self, TimeError> {
        parse::parse(text)
    }
    pub fn as_str(&self) -> &str {
        &self.text
    }
    pub fn local_timestamp(&self) -> String {
        self.text[..19].replace(':', "")
    }
    pub fn date(&self) -> &str {
        &self.text[..10]
    }
    /// The preceding Unix second for a leap second; use `cmp_instant` for exact ordering.
    pub fn unix_second(&self) -> UtcInstant {
        UtcInstant { secs: self.seconds }
    }
    pub fn cmp_instant(&self, other: &Self) -> Ordering {
        (self.seconds, self.leap, &self.fraction).cmp(&(other.seconds, other.leap, &other.fraction))
    }
}

#[cfg(test)]
mod tests;
