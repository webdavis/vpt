fn main() {
    if std::env::args().nth(1).as_deref() == Some("reap-descendant") {
        reap_descendant();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("terminal-exec") {
        use std::os::unix::process::CommandExt;
        // SAFETY: stdin is the owned pseudoterminal slave in a new session.
        if unsafe { libc::ioctl(0, libc::TIOCSCTTY as libc::c_ulong, 0) } == -1 {
            eprintln!("terminal fixture: {}", std::io::Error::last_os_error());
            std::process::exit(1);
        }
        let mut args = std::env::args_os().skip(2);
        let path = args.next().expect("terminal fixture command");
        let error = std::process::Command::new(path).args(args).exec();
        eprintln!("terminal fixture: {error}");
        std::process::exit(1);
    }
    eprintln!("vpt-fake-engine: no subcommand yet");
    std::process::exit(2);
}

fn reap_descendant() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;

    extern "C" fn caught_term(_: libc::c_int) {}

    // SAFETY: the handler is async-signal-safe and this fixture owns its signal mask.
    unsafe {
        libc::signal(
            libc::SIGTERM,
            caught_term as *const () as libc::sighandler_t,
        );
        let mut mask = std::mem::zeroed();
        assert_eq!(libc::sigemptyset(&mut mask), 0);
        assert_eq!(libc::sigaddset(&mut mask, libc::SIGTERM), 0);
        assert_eq!(
            libc::sigprocmask(libc::SIG_UNBLOCK, &mask, std::ptr::null_mut()),
            0
        );
    }
    let mut args = std::env::args_os().skip(2);
    let ready = args.next().expect("ready path");
    let reaped = args.next().expect("reaped path");
    let mut child = std::process::Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("owned descendant");
    let _input = child.stdin.take().expect("retained child input");
    let sentinel = if std::env::var_os("UNOWNED_SENTINEL").is_none() {
        "absent"
    } else {
        "present"
    };
    std::fs::write(
        &ready,
        format!(
            "{} {} {} {} {}",
            std::process::id(),
            child.id(),
            sentinel,
            std::env::var("HOME").expect("owned home"),
            std::env::var("PATH").expect("owned path")
        ),
    )
    .expect("ready record");
    let status = child.wait().expect("reap descendant");
    std::fs::write(
        &reaped,
        format!("{} {}", child.id(), status.signal().unwrap_or(0)),
    )
    .expect("reaping record");
}
