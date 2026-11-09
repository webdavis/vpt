use super::*;
use std::collections::BTreeSet;

#[test]
fn observation_refuses_recovery_without_exporting_history() {
    let result = Observation.recover(&RecoveryView::default());
    assert!(matches!(result, SinkOutcome::Stop(stop) if stop.reason == SinkError::UnsafeState));
}

#[test]
fn repeated_export_warning_is_reported_once_when_startup_already_attached_it() {
    let message = warning_text(SinkWarning::ExportUnavailable);
    let error = vpt_protocol::error::ErrorDocument::new(
        vpt_protocol::error::ErrorKind::Store,
        "startup publication failed",
    )
    .diagnostics(vec!["prior diagnostic".into(), message.into()]);
    let outcome = attach(
        Err(Box::new(error)),
        &BTreeSet::from([SinkWarning::ExportUnavailable]),
    );
    let crate::cli::output::Outcome::Failure(error) = outcome else {
        panic!("failure expected")
    };
    assert_eq!(error.diagnostics, ["prior diagnostic", message]);
}
