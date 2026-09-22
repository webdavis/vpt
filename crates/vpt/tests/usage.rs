mod support;

use support::{Sandbox, run, stderr, stdout};

#[test]
fn an_unknown_verb_under_json_is_an_error_document_on_stderr() {
    let sandbox = Sandbox::new("usage-json");
    for args in [["--json", "transcode"], ["transcode", "--json"]] {
        let output = run(sandbox.vpt().args(args));
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(stdout(&output), "");
        let document: serde_json::Value =
            serde_json::from_str(&stderr(&output)).expect("json on stderr");
        assert_eq!(document["schema"], "vpt.error/1");
        assert_eq!(document["error"]["kind"], "usage");
        assert_eq!(document["error"]["message"], "unknown verb transcode");
    }
}

#[test]
fn json_usage_with_broken_stderr_exits_one() {
    let sandbox = Sandbox::new("usage-json-broken-stderr");
    for verb in ["transcode", "doctor"] {
        let (writer, reader) = std::os::unix::net::UnixStream::pair().expect("local socket pair");
        drop(reader);
        let descriptor: std::os::fd::OwnedFd = writer.into();
        let output = run(sandbox.vpt().args([verb, "--json"]).stderr(descriptor));
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout(&output), "");
    }
}

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
