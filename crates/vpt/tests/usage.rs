mod support;

use support::{Sandbox, run, stderr, stdout};

#[test]
fn an_unknown_verb_prints_usage_to_stderr_and_exits_2() {
    let sandbox = Sandbox::new("usage-unknown");
    let output = run(sandbox.vpt().arg("transcode"));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let err = stderr(&output);
    assert!(err.starts_with("vpt: unknown verb transcode\n"), "{err}");
    assert!(err.contains("usage: vpt"), "{err}");
}

#[test]
fn unknown_input_with_broken_stderr_exits_one() {
    let sandbox = Sandbox::new("usage-broken-stderr");
    let (writer, reader) = std::os::unix::net::UnixStream::pair().expect("local socket pair");
    drop(reader);
    let descriptor: std::os::fd::OwnedFd = writer.into();
    let output = run(sandbox.vpt().arg("transcode").stderr(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}

#[test]
fn unimplemented_verb_with_broken_stderr_exits_one() {
    let sandbox = Sandbox::new("unimplemented-broken-stderr");
    let (writer, reader) = std::os::unix::net::UnixStream::pair().expect("local socket pair");
    drop(reader);
    let descriptor: std::os::fd::OwnedFd = writer.into();
    let output = run(sandbox.vpt().arg("doctor").stderr(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}
