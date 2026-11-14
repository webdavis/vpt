use super::*;
use crate::actions::{configure, sent, spoken};
use std::os::unix::process::ExitStatusExt;

#[test]
fn a_send_vpt_dies_before_saving_is_sent_again_under_the_same_id() {
    let (sandbox, record) = spoken();
    let log = configure(&sandbox);
    fs::write(
        sandbox.path().join("todo-tool.sh"),
        "#!/bin/sh\nlog=\"$1\"\nshift\nprintf '%s\\037' \"$@\" >> \"$log\"\nprintf '\\n' >> \"$log\"\n[ -e \"$log.crashed\" ] || { : > \"$log.crashed\"; kill -9 \"$PPID\"; }\n",
    )
    .unwrap();
    let crashed = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(crashed.status.signal(), Some(9), "{}", stderr(&crashed));
    assert_eq!(sent(&log).len(), 1);
    let result = note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        result["actions"],
        serde_json::json!({"sent": 4, "failed": 0, "refused": 0})
    );
    let ids: Vec<_> = sent(&log).into_iter().map(|args| args[2].clone()).collect();
    assert_eq!(ids.len(), 5);
    assert_eq!(ids[1], ids[0]);
}
