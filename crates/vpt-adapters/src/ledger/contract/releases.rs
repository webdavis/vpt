use super::*;
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, RunKind},
    time::UtcInstant,
};
pub(crate) fn committed<T: std::fmt::Debug>(write: HistoryWrite<T>) -> T {
    match write {
        HistoryWrite::Committed(value) => value,
        other => panic!("{other:?}"),
    }
}
pub(crate) fn setup(
    ledger: &(impl RecordingLedger + RunHistory),
) -> (vpt_domain::history::RunId, NewRelease) {
    let record = record(1);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(
        RunKind::Redact,
        UtcInstant {
            secs: 1_800_000_000,
        },
    ))
    .run;
    (
        run,
        NewRelease {
            recording: record.id,
            stage: NoteStage::Transcript,
            destination: "/released/copy.md".into(),
            report_root: "/drafts".into(),
            source_digest: digest(9),
            text: "Public extract\n".into(),
            report: serde_json::json!({"schema":"vpt.redaction-report/1", "private":"Private mapping",
                "source_artifact_sha256":digest(9).hex(), "released_content_sha256":crate::ledger::brief::validate::digest(b"Public extract\n").hex()}).to_string(),
            provenance: ReleaseProvenance {
                captured_at: record.captured_at,
                captured_offset: record.captured_offset,
                engines: vec!["apple:SpeechTranscriber".into()],
                open_lexical_flags: 1,
                open_flags: 2,
                diagnostics: vec![vpt_domain::uncertainty::Diagnostic::SingleEngine],
                reviewed: false,
            },
            created_at: UtcInstant {
                secs: 1_800_000_001,
            },
        },
    )
}
pub(crate) fn release_reservation_saves_both_payloads_and_history_without_finalizing(
    ledger: &(impl RecordingLedger + RunHistory + ReleaseLedger + ReleaseHistory),
) {
    let (run, draft) = setup(ledger);
    let prepared = committed(ledger.prepare_release(run, &draft));
    assert!(prepared.release.sequence > 0);
    assert!(prepared.receipt.completed.is_empty());
    assert_eq!(prepared.receipt.records.len(), 2);
    assert!(!format!("{prepared:?}").contains("Private"));
    assert!(
        prepared
            .receipt
            .records
            .iter()
            .all(|r| matches!(r.fact, HistoryFact::EffectIntent { .. }))
    );
    assert_eq!(
        prepared.release.report_path,
        draft.report_root.join(format!(
            "{}.transcript.{}.report.json",
            draft.recording, prepared.release.sequence
        ))
    );
    assert!(ledger.releases(&draft.recording).unwrap().is_empty());
    assert_eq!(
        ledger.pending_releases().unwrap(),
        [prepared.release.sequence]
    );
    let payload = ledger
        .release_payload(prepared.release.sequence)
        .unwrap()
        .unwrap();
    assert_eq!(payload.text, draft.text);
    assert_eq!(payload.report, draft.report);
    assert_eq!(payload.release, prepared.release);
    assert!(!format!("{payload:?}").contains("Private"));
}
pub(crate) fn release_completion_requires_both_expected_digests_and_freezes_provenance(
    ledger: &(impl RecordingLedger + RunHistory + ReleaseLedger + ReleaseHistory),
) {
    let (run, mut draft) = setup(ledger);
    let prepared = committed(ledger.prepare_release(run, &draft));
    let seq = prepared.release.sequence;
    for index in 0..3 {
        let mut wrong = prepared.release.version;
        match index {
            0 => wrong.content.0[0] ^= 1,
            1 => wrong.report.0[0] ^= 1,
            _ => wrong.source.0[0] ^= 1,
        }
        assert!(matches!(
            ledger.complete_release(
                run,
                seq,
                wrong,
                UtcInstant {
                    secs: 1_800_000_002
                }
            ),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.releases(&draft.recording).unwrap().is_empty());
    }
    let receipt = committed(ledger.complete_release(
        run,
        seq,
        prepared.release.version,
        UtcInstant {
            secs: 1_800_000_002,
        },
    ));
    assert_eq!(receipt.completed, [draft.recording.clone()]);
    assert_eq!(receipt.records.len(), 3);
    assert!(ledger.pending_releases().unwrap().is_empty());
    assert!(ledger.release_payload(seq).unwrap().is_none());
    assert_eq!(ledger.releases(&draft.recording).unwrap(), [seq]);
    let frozen = ledger.release(seq).unwrap().unwrap();
    draft.provenance.open_flags = 0;
    assert_eq!(frozen.provenance, prepared.release.provenance);
    let before = ledger.pending_exports().unwrap();
    let repeated = committed(ledger.complete_release(
        run,
        seq,
        prepared.release.version,
        UtcInstant {
            secs: 1_800_000_009,
        },
    ));
    assert!(repeated.records.is_empty());
    assert_eq!(ledger.pending_exports().unwrap(), before);
    assert_eq!(ledger.release(seq).unwrap(), Some(frozen));
}
pub(crate) fn release_paths_are_reserved_and_sequences_are_unique(
    ledger: &(impl RecordingLedger + RunHistory + ReleaseLedger + ReleaseHistory),
) {
    let (run, mut draft) = setup(ledger);
    let first = committed(ledger.prepare_release(run, &draft)).release;
    assert!(matches!(
        ledger.prepare_release(run, &draft),
        HistoryWrite::Unconfirmed(_)
    ));
    committed(ledger.complete_release(run, first.sequence, first.version, draft.created_at));
    assert!(matches!(
        ledger.prepare_release(run, &draft),
        HistoryWrite::Unconfirmed(_)
    ));
    draft.destination = "/released/second.md".into();
    let second = committed(ledger.prepare_release(run, &draft)).release;
    assert!(second.sequence > first.sequence);
    assert_ne!(first.report_path, second.report_path);
    committed(ledger.complete_release(run, second.sequence, second.version, draft.created_at));
    assert_eq!(
        ledger.releases(&draft.recording).unwrap(),
        [second.sequence, first.sequence]
    );
}
macro_rules! cases {
    ($case:ident) => {
        $case!(release_reservation_saves_both_payloads_and_history_without_finalizing);
        $case!(release_completion_requires_both_expected_digests_and_freezes_provenance);
        $case!(release_paths_are_reserved_and_sequences_are_unique);
        $case!(release_reservations_refuse_other_publication_writers);
        $case!(failed_release_preparation_leaves_no_payload_history_or_sequence);
    };
}
pub(crate) use cases;

pub(crate) fn release_reservations_refuse_other_publication_writers(
    ledger: &(impl RecordingLedger + RunHistory + ReleaseLedger + ReleaseHistory + PublicationJournal),
) {
    let (run, draft) = setup(ledger);
    let row = committed(ledger.prepare_release(run, &draft)).release;
    for finalized in [false, true] {
        if finalized {
            committed(ledger.complete_release(run, row.sequence, row.version, draft.created_at));
        }
        for target in [&row.destination, &row.report_path] {
            let commit = LedgerCommit {
                publications: vec![DirtyPublication {
                    target: target.clone(),
                    expected_previous: None,
                    intended: digest(7),
                    recorded_at: draft.created_at,
                }],
                ..Default::default()
            };
            assert!(ledger.commit(&commit).is_err());
            assert!(ledger.clear_publication(target).is_err());
            let delta = WorkDelta {
                publication_clears: vec![target.clone()],
                ..Default::default()
            };
            assert!(matches!(
                ledger.commit_step(
                    run,
                    &delta,
                    &[(
                        draft.created_at,
                        HistoryFact::RunFinished(vpt_domain::run::RunOutcome::Completed)
                    )]
                ),
                HistoryWrite::Unconfirmed(_)
            ));
        }
    }
}
pub(crate) fn failed_release_preparation_leaves_no_payload_history_or_sequence(
    ledger: &(impl RecordingLedger + RunHistory + ReleaseLedger + ReleaseHistory),
) {
    let (run, draft) = setup(ledger);
    let before = ledger.pending_exports().unwrap();
    for case in 0..7 {
        let mut changed = draft.clone();
        let mut active = run;
        match case {
            0 => active = vpt_domain::history::RunId::new(run.namespace(), 999),
            1 => changed.destination = "/released/../private.md".into(),
            2 => changed.report_root = "relative".into(),
            3 => changed.provenance.captured_at.secs += 1,
            4 => changed.provenance.reviewed = true,
            5 => changed.text.push('\0'),
            _ => changed.report = changed.report.replace(&digest(9).hex(), &digest(8).hex()),
        }
        assert!(
            matches!(
                ledger.prepare_release(active, &changed),
                HistoryWrite::Unconfirmed(_)
            ),
            "case {case}"
        );
        assert!(ledger.pending_releases().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
    let prepared = committed(ledger.prepare_release(run, &draft));
    assert_eq!(prepared.release.sequence, 1);
}
