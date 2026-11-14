use super::analysis::{append, calls, enable};
use super::*;

fn attempts(log: &std::path::Path) -> usize {
    fs::read_to_string(log.with_extension("attempts"))
        .unwrap_or_default()
        .lines()
        .count()
}

#[test]
fn run_resends_every_refused_action_once_only_with_the_flag() {
    let sandbox = configured();
    enable(&sandbox);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[],"actions":[{"text":"call the bank","sources":["00:00-00:01"],"owner":"me"}],"tags":[],"relations":[]}"#).unwrap();
    let script = sandbox.path().join("send.sh");
    let log = sandbox.path().join("sent");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s\\n' \"$2\" >> \"$1.attempts\"\n[ -e \"$1.fail\" ] && exit 4\nprintf '%s\\n' \"$2\" >> \"$1\"\n",
    )
    .unwrap();
    append(
        &sandbox,
        &format!(
            "\n[actions]\ncommand = ['/bin/sh', {script:?}, {log:?}, '{{text}}']\nfinal_exit_codes = [4]\n"
        ),
    );
    fs::write(log.with_extension("fail"), "").unwrap();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let first = json(&invoke(&sandbox, &[]));
    assert_eq!(
        first["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 1})
    );
    fs::remove_file(log.with_extension("fail")).unwrap();
    json(&invoke(&sandbox, &[]));
    json(&invoke(&sandbox, &["--resend-refused", "--dry-run"]));
    assert_eq!(attempts(&log), 1);
    for sent in [1, 0] {
        let result = json(&invoke(&sandbox, &["--resend-refused"]));
        assert_eq!(
            result["actions"],
            serde_json::json!({"sent": sent, "failed": 0, "refused": 0})
        );
        assert_eq!(fs::read_to_string(&log).unwrap(), "call the bank\n");
    }
    assert_eq!(calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn run_with_the_flag_and_no_actions_command_is_an_input_error() {
    let sandbox = configured();
    let output = invoke(&sandbox, &["--resend-refused"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--resend-refused requires [actions] command"));
}
