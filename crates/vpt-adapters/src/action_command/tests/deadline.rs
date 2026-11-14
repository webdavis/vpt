use super::super::{ActionSend, SendOutcome, send_within};
use crate::config::{from_table, load_text};
use std::path::Path;
use std::time::{Duration, Instant};
use vpt_domain::{
    grounding::GroundedClaim,
    identity::RecordingId,
    proposal::{CheckedAction, CheckedClaim, Claim},
};

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
    let action = CheckedAction {
        claim: CheckedClaim {
            claim: Claim {
                text: "call the bank".into(),
                source_references: vec![],
            },
            grounding: GroundedClaim {
                sources: vec![],
                finding: None,
                inherited_flags: vec![],
            },
        },
        owner: None,
        due: None,
    };
    let recording = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let send = ActionSend {
        recording: &recording,
        action: &action,
        id: "2026-08-24T144736-4f3ab19c02de#1",
        note: "",
        link: "",
    };
    let started = Instant::now();
    let outcome = send_within(&settings, &send, Duration::from_millis(200));
    assert_eq!(outcome, SendOutcome::Failed);
    assert!(started.elapsed() < Duration::from_secs(10));
}
