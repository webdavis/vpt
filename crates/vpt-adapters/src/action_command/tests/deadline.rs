use super::super::{SendOutcome, run_within};
use crate::config::{from_table, load_text};
use std::path::Path;
use std::time::{Duration, Instant};

#[test]
fn a_command_past_its_deadline_fails_even_when_every_kill_code_is_final() {
    let settings = from_table(
        &load_text(
            "config_version = 1\n[actions]\ncommand = ['/bin/sh', '-c', 'sleep 30', 'todo-tool', '{text}']\nfinal_exit_codes = [1, 9, 15, 124, 137, 143]\n",
        )
        .unwrap(),
        Path::new("/u"),
    )
    .unwrap();
    let values = [("{text}", "call the bank")];
    let started = Instant::now();
    let outcome = run_within(
        &settings,
        &settings.actions_command,
        &values,
        Duration::from_millis(200),
    );
    assert_eq!(outcome, SendOutcome::Failed);
    assert!(started.elapsed() < Duration::from_secs(10));
}
