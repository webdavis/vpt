use super::analysis::{append, calls, enable};
use super::*;

#[test]
fn run_retries_a_failed_action_send_once_the_command_works_and_never_resends() {
    let sandbox = configured();
    enable(&sandbox);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[],"actions":[{"text":"call the bank","sources":["00:00-00:01"],"owner":"me"}],"tags":[],"relations":[]}"#).unwrap();
    let script = sandbox.path().join("send.sh");
    let log = sandbox.path().join("sent");
    fs::write(
        &script,
        "#!/bin/sh\n[ -e \"$1.fail\" ] && exit 3\nprintf '%s\\n' \"$2\" >> \"$1\"\n",
    )
    .unwrap();
    append(
        &sandbox,
        &format!("\n[actions]\ncommand = ['/bin/sh', {script:?}, {log:?}, '{{text}}']\n"),
    );
    fs::write(log.with_extension("fail"), "").unwrap();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let first = json(&invoke(&sandbox, &[]));
    assert_eq!(first["analyzed"].as_array().unwrap().len(), 1);
    assert_eq!(
        first["warnings"],
        serde_json::json!(["configured action command failed"])
    );
    assert_eq!(
        first["actions"],
        serde_json::json!({"sent": 0, "failed": 1, "refused": 0})
    );
    assert!(!log.exists());
    fs::remove_file(log.with_extension("fail")).unwrap();
    json(&invoke(&sandbox, &["--dry-run"]));
    assert!(!log.exists());
    for sent in [1, 0] {
        let result = json(&invoke(&sandbox, &[]));
        assert_eq!(result["warnings"], serde_json::json!([]));
        assert_eq!(
            result["actions"],
            serde_json::json!({"sent": sent, "failed": 0, "refused": 0})
        );
        assert_eq!(fs::read_to_string(&log).unwrap(), "call the bank\n");
    }
    assert_eq!(calls(&sandbox, "agent.calls"), 1);
}
