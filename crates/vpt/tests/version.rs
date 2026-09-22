mod support;

use support::{Sandbox, broken_output, run, stderr, stdout};

#[test]
fn a_json_stdout_write_failure_is_one_store_document_on_stderr() {
    let sandbox = Sandbox::new("version-json-write-error");
    let (descriptor, _peer) = broken_output();
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json"])
        .stdout(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let document: serde_json::Value =
        serde_json::from_str(&stderr(&output)).expect("json on stderr");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], "store");
    assert_eq!(document["error"]["message"], "standard output write failed");
}

#[test]
fn json_version_with_both_output_streams_broken_exits_one() {
    let sandbox = Sandbox::new("version-json-both-streams-broken");
    let (out, _peer) = broken_output();
    let err = out.try_clone().expect("stderr descriptor");
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json"])
        .stdout(out)
        .stderr(err));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

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
    let (descriptor, _peer) = broken_output();
    let output = run(sandbox.vpt().arg("--version").stdout(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "vpt: standard output write failed\n");
}

#[test]
fn version_with_both_output_streams_broken_exits_one() {
    let sandbox = Sandbox::new("version-both-streams-broken");
    let (out, _peer) = broken_output();
    let err = out.try_clone().expect("stderr descriptor");
    let output = run(sandbox.vpt().arg("--version").stdout(out).stderr(err));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}
