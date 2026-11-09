pub mod dam;
pub mod google;
mod time;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Unavailable,
    Io,
    Exit,
    Deadline,
    Interrupted,
    Capacity,
    Invalid,
    Network,
    Scope,
    HttpStatus,
    PageLimit,
}
impl Failure {
    pub fn detail(self) -> &'static str {
        match self {
            Self::Unavailable => "context program unavailable",
            Self::Io => "context program I/O failed",
            Self::Exit => "context program failed",
            Self::Deadline => "context collection deadline exceeded",
            Self::Interrupted => "context collection interrupted",
            Self::Capacity => "context collection exceeds capacity",
            Self::Invalid => "invalid context document",
            Self::Network => "context network request failed",
            Self::Scope => "Google token must grant only calendar.events.readonly",
            Self::HttpStatus => "context provider request failed",
            Self::PageLimit => "calendar collection page limit exceeded",
        }
    }
}
