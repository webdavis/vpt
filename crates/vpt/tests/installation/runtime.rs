use crate::{
    fixture,
    support::{self, Sandbox, stderr},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, UNIX_EPOCH},
};

pub(super) fn installed_bin() -> PathBuf {
    let root = PathBuf::from(
        std::env::var_os("VPT_TEST_INSTALL_ROOT")
            .expect("set VPT_TEST_INSTALL_ROOT to the isolated installation"),
    );
    assert!(root.is_absolute());
    let bin = root.join("bin");
    assert!(bin.join("vpt").is_file());
    assert!(bin.join("vpt-macos").is_file());
    assert!(!bin.join("vpt-fake-engine").exists());
    bin
}

pub(super) fn command(sandbox: &Sandbox) -> Command {
    let template = sandbox.vpt();
    let mut command = Command::new(installed_bin().join("vpt"));
    command
        .env_clear()
        .envs(
            template
                .get_envs()
                .map(|(key, value)| (key, value.unwrap())),
        )
        .current_dir(sandbox.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

pub(super) fn invoke(sandbox: &Sandbox, args: &[&str]) -> Output {
    let budget = if args[0] == "run" {
        Duration::from_secs(8)
    } else {
        Duration::from_secs(5)
    };
    support::run_with_budget(command(sandbox).args(args), budget)
}

pub(super) fn json(output: &Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(super) fn configured(kind: &str) -> Sandbox {
    let sandbox = Sandbox::new("installed-runtime");
    let engine = sandbox.path().join("bin/external-engine");
    let calls = sandbox.path().join("calls");
    let document = sandbox.path().join("engine.json");
    let common = format!(
        "[source]\nrecordings_dir = {:?}\nread_titles = false\nquiet_period_secs = 1\n",
        sandbox.path().join("voice-memos/Recordings")
    );
    let config = if kind == "command" {
        fs::write(
            &engine,
            "#!/bin/sh\nprintf 'called\\n' >> \"$1\"\n/bin/cat \"$2\"\n",
        )
        .unwrap();
        fs::write(&document, serde_json::json!({
            "schema":"vpt.engine/1", "engine":{"name":"custom","tool":"fixture","version":"1","model":"test","family":"independent"},
            "language":"en-US", "local":true, "segments":[{"start":0.0,"end":1.0,"text":"Private words", "words":[{"text":"Private","start":0.0,"end":0.5},{"text":"words","start":0.5,"end":1.0}]}]
        }).to_string()).unwrap();
        format!(
            "{common}[engines]\nmain = 'custom'\n[engines.custom]\nkind = 'command'\nfamily = 'independent'\nlocal = true\ncommand = [{engine:?}, {calls:?}, {document:?}]\n"
        )
    } else {
        fs::write(&engine, "#!/bin/sh\nset -eu\nprintf 'called\\n' >> \"$1\"\nsource=$2\nshift 2\n/bin/mkdir -p \"$5/audio\"\n/bin/cat \"$source\" > \"$5/audio/audio.json\"\n").unwrap();
        fs::write(&document, serde_json::json!({"device":"cpu","model":"medium","transcription":{"en":{
            "text":"Private words","chunks":[{"timestamp":[0.0,1.0],"text":"Private words","words":[{"word":"Private","start":0.0,"end":0.5},{"word":"words","start":0.5,"end":1.0}]}]
        }}}).to_string()).unwrap();
        format!(
            "{common}[engines]\nmain = 'whisply'\n[engines.whisply]\ndevice = 'cpu'\nmodel = 'medium'\ncommand = [{engine:?}, {calls:?}, {document:?}]\n"
        )
    };
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o700)).unwrap();
    fixture::write_config(&sandbox, &config);
    sandbox
}

pub(super) fn add_audio(sandbox: &Sandbox) -> PathBuf {
    let source = sandbox.path().join("voice-memos/Recordings/one.m4a");
    fs::write(
        &source,
        vpt_domain::fixtures::m4a(1_787_604_456, 12, b"private audio"),
    )
    .unwrap();
    fs::File::open(&source)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(1_787_604_456))
        .unwrap();
    source
}

pub(super) fn assert_retention_requires_helper(sandbox: &Sandbox) {
    use vpt_application::ports::RecordingLedger;
    let ledger = vpt_adapters::SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let audio = ledger.recordings().unwrap().remove(0).audio_path;
    drop(ledger);
    let original = fs::read(&audio).unwrap();
    fs::File::open(&audio)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(100))
        .unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[retention]\nenabled = true\ninclude_audio = true\n[retention.hold]\naudio = '1d'\n")).unwrap();
    let result = support::run(command(sandbox).args(["retention", "run", "--json"]));
    assert_eq!(result.status.code(), Some(3), "{}", stderr(&result));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["rule"], "helper_absent");
    assert_eq!(fs::read(audio).unwrap(), original);
}
