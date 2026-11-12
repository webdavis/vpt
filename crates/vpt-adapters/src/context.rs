pub mod dam;
pub mod google;
mod time;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Unavailable,
    Io,
    Exit,
    Usage,
    Deadline,
    Interrupted,
    Capacity,
    Stale,
    UnknownRemote,
    Invalid,
    Network,
    Scope,
    HttpStatus,
    PageLimit,
}
impl Failure {
    pub fn detail(self) -> &'static str {
        match self {
            Self::Unavailable => "dam was not found; install dam or set context.dam.command",
            Self::Io => "context program I/O failed",
            Self::Exit => "context program failed",
            Self::Usage => "dam rejected the command line; dam agenda needs a newer dam",
            Self::Deadline => "context collection deadline exceeded",
            Self::Interrupted => "context collection interrupted",
            Self::Capacity => "context collection exceeds capacity",
            Self::UnknownRemote => {
                "context.dam.max_age names a remote that dam does not know or that its token may not read; use a name from dam remote list"
            }
            Self::Stale => "calendar data is stale: dam refused it under context.dam.max_age",
            Self::Invalid => "invalid context document",
            Self::Network => "context network request failed",
            Self::Scope => "Google token must grant only calendar.events.readonly",
            Self::HttpStatus => "context provider request failed",
            Self::PageLimit => "calendar collection page limit exceeded",
        }
    }
}
