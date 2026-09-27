use super::{Sandbox, Value, fake, json, run, stderr, stdout};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::time::Duration;

#[test]
fn bounded_exchange_fixture_preserves_arguments_input_and_separate_streams() {
    let sandbox = Sandbox::new("bounded-exchange-control");
    let input = sandbox.path().join("stdin");
    let text = "owned wrapper input\n";
    std::fs::write(&input, text).expect("owned stdin");
    let arguments = ["one two", "", "$(literal);*", "{artifact}"];
    let output = run(fake(&sandbox)
        .arg("process-exchange")
        .args(arguments)
        .env("VPT_FAKE_INPUT_LEN", text.len().to_string())
        .env("VPT_FAKE_EXPIRE_INPUT_ON_PENDING", "1")
        .stdin(std::fs::File::open(input).expect("owned input")));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "owned stderr marker\n");
    let document: Value = serde_json::from_slice(&output.stdout).expect("exchange JSON");
    assert_eq!(document["argv"], json!(arguments));
    assert_eq!(document["stdin"], text);
    assert_eq!(
        document["environment"]["HOME"],
        json!(sandbox.path().join("home"))
    );
}

#[test]
fn bounded_exchange_fixture_fails_when_its_retained_input_never_arrives() {
    let sandbox = Sandbox::new("bounded-exchange-expiry");
    let (reader, _retained_writer) = UnixStream::pair().expect("owned input channel");
    let output = run(fake(&sandbox)
        .arg("process-exchange")
        .env("VPT_FAKE_INPUT_LEN", "1")
        .env("VPT_FAKE_EXPIRE_INPUT_ON_PENDING", "1")
        .stdin(OwnedFd::from(reader)));
    assert_eq!(output.status.code(), Some(75));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "fixture input pending\nfixture input deadline\n"
    );
}

#[test]
fn public_environment_wrapper_delivers_exact_arguments_and_input_with_owned_environment() {
    let sandbox = Sandbox::new("public-process-wrapper");
    let environment = sandbox.vpt();
    let text = "owned wrapper input\n";
    let mut values = environment
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_str().expect("environment key").to_owned(),
                value.expect("set").to_str().expect("owned path").to_owned(),
            )
        })
        .collect::<Vec<_>>();
    values.push(("VPT_FAKE_INPUT_LEN".into(), text.len().to_string()));
    let overrides = values
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    let arguments = ["one two", "one", "two", "", "$(literal);*'\"", "{artifact}"];
    let mut argv = vec![
        env!("CARGO_BIN_EXE_vpt-fake-engine").to_owned(),
        "process-exchange".into(),
    ];
    argv.extend(arguments.iter().map(|arg| (*arg).to_owned()));
    let outcome = vpt_adapters::run_with_env(
        &argv,
        text.as_bytes(),
        Duration::from_millis(500),
        vpt_adapters::OUTPUT_LIMIT,
        &overrides,
    )
    .expect("public wrapper exchange");
    assert_eq!(outcome.status, vpt_adapters::Status::Exited(0));
    assert_eq!(outcome.stderr, b"owned stderr marker\n");
    let document: Value = serde_json::from_slice(&outcome.stdout).expect("exchange document");
    assert_eq!(document["argv"], json!(arguments));
    assert_eq!(document["stdin"], text);
    for (key, value) in values.iter().filter(|(key, _)| key != "VPT_FAKE_INPUT_LEN") {
        assert_eq!(document["environment"][key], *value, "owned {key}");
    }
    assert!(outcome.group > 1);
    // SAFETY: a zeroed status is valid output storage for waitid.
    let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    // SAFETY: WNOWAIT only observes this completed wrapper's former child number.
    assert_eq!(
        unsafe {
            libc::waitid(
                libc::P_PID,
                outcome.group as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    // SAFETY: signal zero only observes the completed wrapper's former group number.
    assert_eq!(unsafe { libc::killpg(outcome.group, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

#[test]
fn public_runner_classifies_a_missing_owned_executable() {
    let sandbox = Sandbox::new("public-process-missing");
    let missing = sandbox
        .path()
        .join("missing-executable")
        .to_str()
        .expect("owned path")
        .to_owned();
    assert_eq!(
        vpt_adapters::run(std::slice::from_ref(&missing), b"", Duration::ZERO, 0)
            .expect_err("missing program"),
        vpt_adapters::SpawnError::NotFound(missing)
    );
}
