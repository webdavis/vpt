mod support;

use support::{Sandbox, run, stderr, stdout};

#[test]
fn version_reports_the_crate_version_and_no_helper_when_the_path_is_empty() {
    let sandbox = Sandbox::new("version-json");
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "version");
    assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
    assert!(document["helper_version"].is_null());
}

#[test]
fn version_without_json_prints_one_human_line() {
    let sandbox = Sandbox::new("version-human");
    let output = run(sandbox.vpt().arg("--version"));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        format!("vpt {} (helper absent)\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn a_stdout_write_failure_exits_one() {
    let sandbox = Sandbox::new("version-write-error");
    let (writer, reader) = std::os::unix::net::UnixStream::pair().expect("local socket pair");
    drop(reader);
    let descriptor: std::os::fd::OwnedFd = writer.into();
    let output = run(sandbox.vpt().arg("--version").stdout(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("write stdout"));
}

#[test]
fn version_with_both_output_streams_broken_exits_one() {
    let sandbox = Sandbox::new("version-both-streams-broken");
    let (writer, reader) = std::os::unix::net::UnixStream::pair().expect("local socket pair");
    drop(reader);
    let err: std::os::fd::OwnedFd = writer.try_clone().expect("stderr descriptor").into();
    let out: std::os::fd::OwnedFd = writer.into();
    let output = run(sandbox.vpt().arg("--version").stdout(out).stderr(err));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}
