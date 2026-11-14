use super::*;
use crate::actions::{configure_with, propose, sent};

fn resend(sandbox: &Sandbox, id: &str, extra: &[&str]) -> std::process::Output {
    let mut command = sandbox.vpt();
    command
        .args(["analyze", id, "--resend-refused", "--json"])
        .args(extra);
    support::run_with_budget(&mut command, Duration::from_secs(5))
}

fn refused() -> (Sandbox, String, std::path::PathBuf) {
    let (sandbox, mut rows) = note::configured_saying(&[7], "um private words tomorrow");
    let id = rows.remove(0).id.as_str().to_owned();
    let log = configure_with(&sandbox, "final_exit_codes = [4]\n");
    fs::write(log.with_extension("fail"), "4").unwrap();
    let result = note::success(&invoke(&sandbox, &id, false));
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 4})
    );
    (sandbox, id, log)
}

fn attempts(log: &std::path::Path) -> usize {
    sent(&log.with_extension("attempts")).len()
}

#[test]
fn the_flag_sends_refused_actions_once_without_running_the_agent_again() {
    let (sandbox, id, log) = refused();
    fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&invoke(&sandbox, &id, false));
    assert!(sent(&log).is_empty());
    propose(
        &sandbox,
        r#"[{"text":"a reworded task","sources":[],"owner":"me"}]"#,
    );
    let result = note::success(&resend(&sandbox, &id, &[]));
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 4, "failed": 0, "refused": 0})
    );
    assert_eq!(result["warnings"], serde_json::json!([]));
    let texts: Vec<_> = sent(&log)
        .into_iter()
        .map(|args| args.last().unwrap().clone())
        .collect();
    assert!(texts.contains(&"- send the secret invoice".to_owned()));
    assert!(!texts.iter().any(|text| text == "a reworded task"));
    let again = note::success(&resend(&sandbox, &id, &[]));
    assert_eq!(
        again["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 0})
    );
    assert_eq!(sent(&log).len(), 4);
}

#[test]
fn a_command_that_still_refuses_leaves_the_actions_refused() {
    let (sandbox, id, log) = refused();
    let result = note::success(&resend(&sandbox, &id, &[]));
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 0, "failed": 0, "refused": 4})
    );
    assert_eq!(attempts(&log), 8);
    fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&invoke(&sandbox, &id, false));
    assert_eq!(attempts(&log), 8);
}

#[test]
fn a_dry_run_with_the_flag_sends_nothing_and_keeps_the_actions_refused() {
    let (sandbox, id, log) = refused();
    fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&resend(&sandbox, &id, &["--dry-run"]));
    note::success(&invoke(&sandbox, &id, false));
    assert_eq!(attempts(&log), 4);
}

#[test]
fn the_flag_without_an_actions_command_is_an_input_error() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    for extra in [&[][..], &["--dry-run"]] {
        let output = resend(&sandbox, record.id.as_str(), extra);
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains("--resend-refused requires [actions] command"));
    }
}

#[test]
fn the_flag_takes_one_recording() {
    let (sandbox, _) = note::configured();
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["analyze", "--all", "--resend-refused", "--json"]),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--resend-refused"));
}
