use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn changed_source_after_stage_defers_before_owned_cleanup() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("changed.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    parts.settings.deferral_page_threshold = 1;
    parts.sink.watched_audio = Some(fixture.audio.clone());
    let probe = GateRecorder::new(&fixture.recordings);
    probe.grow_after_stage.set(true);

    let report = probed(&mut parts, &probe).expect("changed source deferral");

    assert_eq!(report.deferred[0].reason, DeferralReason::ChangedDuringRead);
    assert_eq!(
        parts.sink.owned_stage_present_at_deferred_export,
        Some(true)
    );
    assert_eq!(parts.trash.attempted.borrow().len(), 1);
    assert_eq!(
        parts.trash.attempted.borrow()[0].parent(),
        Some(fixture.audio.as_path())
    );
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.archive.archived().unwrap().is_empty());
    assert_eq!(
        parts.ledger.seen(&source).unwrap().unwrap().deferral_reason,
        Some(DeferralReason::ChangedDuringRead)
    );
    let receipt = parts
        .sink
        .receipts
        .iter()
        .find(|receipt| {
            receipt.records.iter().any(|record| {
                matches!(
                    record.fact,
                    HistoryFact::WorkStep {
                        outcome: WorkOutcome::Deferred,
                        ..
                    }
                )
            })
        })
        .unwrap();
    assert_eq!(
        receipt.records.len(),
        2,
        "threshold event shares the seen/work transaction"
    );
    assert!(
        receipt
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::Event(_)))
    );
    assert_eq!(receipt.completed.len(), 0);
}

#[test]
fn invalid_staged_container_defers_and_cleans_only_owned_stage() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("stage.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let mut parts = fixture.parts();
    parts.sink.watched_audio = Some(fixture.audio.clone());
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let report = probed(&mut parts, &probe).expect("invalid staged container deferral");

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(
        parts.sink.owned_stage_present_at_deferred_export,
        Some(true)
    );
    assert_eq!(parts.trash.attempted.borrow().len(), 1);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.archive.archived().unwrap().is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert_eq!(
        parts.ledger.seen(&source).unwrap().unwrap().deferral_count,
        1
    );
    assert!(
        !parts
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::Event(_)))
    );
}

#[test]
fn unrepresentable_capture_instant_defers_invalid_staged_container() {
    let fixture = Fixture::new();
    let mut bytes = box_of(b"ftyp", b"M4A ");
    bytes.extend(box_of(b"mdat", b"owned audio"));
    bytes.extend(box_of(b"moov", &mvhd_v1(253_402_387_200, 1)));
    fixture.add_recording("future.m4a", &bytes);
    let mut parts = fixture.parts();

    let report = parts
        .ingest()
        .run(&Mode::default())
        .expect("invalid capture deferral");

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(parts.trash.moved.borrow().len(), 1);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
}

#[test]
fn byte_copy_clone_is_eligible_for_normal_publication() {
    let fixture = Fixture::new();
    fixture.add_recording("copy.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::ByteCopy);

    let report = probed(&mut parts, &probe).expect("byte copy clone");

    assert_eq!(report.ingested.len(), 1);
    assert!(report.deferred.is_empty());
    assert_eq!(probe.clones.get(), 1);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn absent_helper_keeps_private_stage_after_committed_deferral() {
    let fixture = Fixture::new();
    fixture.add_recording("absent.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let mut parts = fixture.parts();
    *parts.trash.failure.borrow_mut() = Some(TrashError::HelperAbsent);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let report = probed(&mut parts, &probe).expect("deferral survives absent helper");

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(parts.trash.attempted.borrow().len(), 1);
    let leftovers = parts.archive.staged_leftovers().unwrap();
    assert_eq!(leftovers.len(), 1);
    assert_eq!(
        std::fs::metadata(&leftovers[0])
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(
        report
            .log
            .iter()
            .any(|line| line.contains("cleanup pending"))
    );
    assert!(parts.ledger.recordings().unwrap().is_empty());
}

#[test]
fn incompatible_helper_major_refuses_after_deferral_and_keeps_safe_diagnostic() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("incompatible.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let mut parts = fixture.parts();
    *parts.trash.failure.borrow_mut() = Some(TrashError::HelperVersion { found: 7 });
    parts
        .trash
        .diagnostics
        .borrow_mut()
        .push("unknown additive helper field: capabilities".into());
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let error = probed(&mut parts, &probe).expect_err("incompatible helper refusal");

    assert_eq!(error.failure, IngestFailure::HelperVersion { found: 7 });
    assert!(error.completed.is_empty());
    assert!(
        error
            .log
            .iter()
            .any(|line| line == "unknown additive helper field: capabilities")
    );
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert_eq!(
        parts.ledger.seen(&source).unwrap().unwrap().deferral_reason,
        Some(DeferralReason::InvalidContainer)
    );
    assert!(parts.ledger.recordings().unwrap().is_empty());
}

#[test]
fn escaped_additive_helper_field_name_survives_without_private_diagnostic() {
    let fixture = Fixture::new();
    fixture.add_recording("escaped.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let mut parts = fixture.parts();
    *parts.trash.failure.borrow_mut() = Some(TrashError::HelperVersion { found: 7 });
    parts.trash.diagnostics.borrow_mut().extend([
        "unknown additive helper field: bad\\nkey".to_owned(),
        "CANARY-PRIVATE-VALUE".to_owned(),
    ]);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let error = probed(&mut parts, &probe).expect_err("incompatible helper refusal");

    assert_eq!(error.failure, IngestFailure::HelperVersion { found: 7 });
    assert!(
        error
            .log
            .iter()
            .any(|line| line == "unknown additive helper field: bad\\nkey")
    );
    assert!(!error.log.join(" ").contains("CANARY-PRIVATE-VALUE"));
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
}

#[test]
fn bounded_helper_diagnostics_preserve_all_sixteen_field_names() {
    let prefix = "unknown additive helper field: ";
    let mut expected = vec![
        prefix.to_owned(),
        format!("{prefix}a field.with-punctuation"),
        format!("{prefix}{}", "\\u{10ffff}".repeat(32)),
    ];
    expected.extend((3..16).map(|index| format!("{prefix}field_{index}")));
    let mut diagnostics = expected.clone();
    diagnostics.push(format!("{prefix}outside_budget"));

    let error = helper_diagnostic_error(diagnostics);

    let fields: Vec<_> = error
        .log
        .iter()
        .filter(|line| line.starts_with(prefix))
        .cloned()
        .collect();
    assert_eq!(fields, expected);
    assert!(!error.log.join(" ").contains("outside_budget"));
}

#[test]
fn helper_diagnostic_length_limit_preserves_boundary_and_refuses_one_over() {
    let prefix = "unknown additive helper field: ";
    let before = format!("{prefix}{}", "a".repeat(399 - prefix.len()));
    let boundary = format!("{prefix}{}", "b".repeat(400 - prefix.len()));
    let over = format!("{prefix}{}", "c".repeat(401 - prefix.len()));

    let error = helper_diagnostic_error(vec![before.clone(), boundary.clone(), over.clone()]);

    assert!(error.log.contains(&before));
    assert!(error.log.contains(&boundary));
    assert!(!error.log.contains(&over));
    assert_eq!(
        error
            .log
            .iter()
            .filter(|line| *line == "trash reported a diagnostic")
            .count(),
        1
    );
}

#[test]
fn raw_helper_diagnostics_are_replaced_without_private_content() {
    let prefix = "unknown additive helper field: ";
    let error = helper_diagnostic_error(vec![
        "CANARY-PRIVATE-VALUE".to_owned(),
        format!("{prefix}raw\nCANARY-PRIVATE-VALUE"),
        format!("{prefix}\u{1b}CANARY-PRIVATE-VALUE"),
        format!("{prefix}\u{202e}CANARY-PRIVATE-VALUE"),
    ]);

    assert!(!error.log.join(" ").contains("CANARY-PRIVATE-VALUE"));
    assert_eq!(
        error
            .log
            .iter()
            .filter(|line| *line == "trash reported a diagnostic")
            .count(),
        4
    );
}

fn helper_diagnostic_error(diagnostics: Vec<String>) -> Box<IngestError> {
    let fixture = Fixture::new();
    fixture.add_recording("diagnostic.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let mut parts = fixture.parts();
    *parts.trash.failure.borrow_mut() = Some(TrashError::HelperVersion { found: 7 });
    parts.trash.diagnostics.borrow_mut().extend(diagnostics);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let error = probed(&mut parts, &probe).expect_err("incompatible helper refusal");

    assert_eq!(error.failure, IngestFailure::HelperVersion { found: 7 });
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    error
}
