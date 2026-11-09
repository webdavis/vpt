use super::*;
use std::os::unix::ffi::OsStringExt;
use vpt_application::{Kept, KeptReason};
use vpt_domain::identity::RecordingId;

#[test]
fn non_utf8_report_path_is_a_safe_error_preserving_completed_work() {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let report = RetentionReport {
        completed: vec![id.clone().into()],
        kept: vec![Kept {
            path: std::ffi::OsString::from_vec(b"/private-\xff".to_vec()).into(),
            reason: KeptReason::Untracked,
        }],
        ..Default::default()
    };
    match outcome(Ok(report), false) {
        Outcome::FailedReport { error, .. } => {
            assert_eq!(error.exit_code(), 1);
            assert_eq!(error.completed, vec![id.to_string()]);
            assert!(!error.to_json().to_string().contains("private"));
        }
        _ => panic!("unrepresentable path must return a safe error"),
    }
}

#[test]
fn failure_preserves_completed_recordings_even_without_a_new_move() {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let failure = RetentionFailure {
        report: RetentionReport {
            completed: vec![id.clone().into()],
            ..Default::default()
        },
        error: vpt_application::RetentionError::Ledger(vpt_application::ports::LedgerError::Busy),
        committed: None,
    };
    match outcome(Err(Box::new(failure)), false) {
        Outcome::FailedReport { error, .. } => {
            assert_eq!(error.completed, vec![id.to_string()]);
        }
        _ => panic!("failure must preserve its partial report"),
    }
}
