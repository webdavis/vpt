use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

/// Runs the command with its standard output on a pseudo-terminal and standard error on a pipe,
/// returning what the terminal showed with its line endings turned back into `\n`.
pub fn on_a_terminal(command: &mut Command, budget: Duration) -> Output {
    let (mut master, slave) = pty();
    let reader = std::thread::spawn(move || {
        let mut shown = Vec::new();
        let mut chunk = [0u8; 4096];
        // Reading the master fails with EIO once every copy of the slave has closed.
        while let Ok(read @ 1..) = master.read(&mut chunk) {
            shown.extend_from_slice(&chunk[..read]);
        }
        shown
    });
    command.stdout(Stdio::from(slave));
    let mut output = super::run_with_budget(command, budget);
    command.stdout(Stdio::null());
    let shown = reader.join().expect("terminal reader");
    output.stdout = String::from_utf8(shown)
        .expect("terminal text")
        .replace("\r\n", "\n")
        .into_bytes();
    output
}

fn pty() -> (File, OwnedFd) {
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty writes two descriptors it opened, which are owned here from now on.
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0, "openpty: {}", std::io::Error::last_os_error());
    // SAFETY: both descriptors are open and nothing else owns them.
    unsafe { (File::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}
