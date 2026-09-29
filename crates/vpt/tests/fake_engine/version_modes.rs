use super::{Sandbox, Value, broken_output, fake, json, run, stderr, stdout};

#[test]
fn additive_version_preserves_the_selected_version_and_extra_field() {
    let sandbox = Sandbox::new("fake-version-additive");
    let output = run(fake(&sandbox)
        .arg("--version")
        .env("VPT_FAKE_VERSION", "2.0.0")
        .env("VPT_FAKE_VERSION_MODE", "additive"));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document,
        json!({"schema":"vpt.helper/1","version":"2.0.0","capabilities":[]})
    );
}

#[test]
fn private_failure_returns_exact_streams_and_exit_status() {
    let sandbox = Sandbox::new("fake-version-private");
    let output = run(fake(&sandbox)
        .arg("--version")
        .env("VPT_FAKE_VERSION_MODE", "private-failure"));
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(stdout(&output), "private-stdout-canary");
    assert_eq!(stderr(&output), "private-stderr-canary");
}

#[test]
fn version_modes_reject_extra_arguments_without_output() {
    for mode in ["", "additive", "private-failure"] {
        let sandbox = Sandbox::new("fake-version-extra-argument");
        let output = run(fake(&sandbox)
            .args(["--version", "unexpected"])
            .env("VPT_FAKE_VERSION_MODE", mode));
        assert_eq!(output.status.code(), Some(91), "mode {mode}");
        assert_eq!(stdout(&output), "");
        assert_eq!(stderr(&output), "");
    }
}

#[test]
fn unknown_version_mode_is_a_usage_error_without_output() {
    let sandbox = Sandbox::new("fake-version-unknown-mode");
    let output = run(fake(&sandbox)
        .arg("--version")
        .env("VPT_FAKE_VERSION_MODE", "unknown"));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn private_failure_reports_a_broken_output_stream() {
    for break_stdout in [true, false] {
        let sandbox = Sandbox::new("fake-version-broken-stream");
        let (writer, _peer) = broken_output();
        let mut command = fake(&sandbox);
        command
            .arg("--version")
            .env("VPT_FAKE_VERSION_MODE", "private-failure");
        if break_stdout {
            command.stdout(writer);
        } else {
            command.stderr(writer);
        }
        let output = run(&mut command);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            stdout(&output),
            if break_stdout {
                ""
            } else {
                "private-stdout-canary"
            }
        );
        assert_eq!(stderr(&output), "");
    }
}
