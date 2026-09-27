use std::io;
use std::process::ExitStatus;

pub(super) struct Reaped {
    pub status: ExitStatus,
    pub failure: Option<io::Error>,
}

pub(super) fn finish<P, W>(
    mut first_failure: Option<io::Error>,
    mut poll: P,
    mut wait: W,
) -> io::Result<Reaped>
where
    P: FnMut() -> io::Result<Option<ExitStatus>>,
    W: FnMut(),
{
    loop {
        match poll() {
            Ok(Some(status)) => {
                return Ok(Reaped {
                    status,
                    failure: first_failure,
                });
            }
            Ok(None) => {}
            Err(error) if error.raw_os_error() == Some(libc::ECHILD) => return Err(error),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                first_failure.get_or_insert(error);
            }
        }
        wait();
    }
}

#[cfg(test)]
mod tests;
