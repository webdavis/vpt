use super::*;
use crate::ledger::contract::{record, transcript::fixture};
use vpt_application::ports::*;
use vpt_domain::{
    run::RunKind,
    uncertainty::{Class, Diagnostic, LexicalFlag},
};

#[test]
fn observations_keep_metadata_counts_and_order_without_content() {
    let ledger = MemoryLedger::new();
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(82), record(81)],
            ..Default::default()
        })
        .unwrap();
    let mut accepted = fixture();
    accepted.from_checker = false;
    accepted.transcript.segments[0].words[0].confidence = Some(0.1);
    let mut checker = accepted.transcript.engine.clone();
    checker.name = "second".into();
    checker.family = "independent".into();
    accepted.checker_engine = Some(checker);
    accepted.uncertainty.diagnostics.push(Diagnostic::Divergent);
    accepted.uncertainty.lexical.push(LexicalFlag {
        class: Class::LowConfidence,
        ranges: std::iter::once(0..1).collect(),
        record_text: "Privé".into(),
        alternative_text: None,
        confidence: Some(0.1),
    });
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Transcribe, accepted.accepted_at)
    else {
        panic!("run")
    };
    let result = ledger.commit_step(
        run.run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
            ..Default::default()
        },
        &[(
            vpt_domain::time::UtcInstant { secs: 50 },
            vpt_domain::run::HistoryFact::WorkStep {
                recording: Some(record(81).id),
                outcome: vpt_domain::run::WorkOutcome::Completed,
            },
        )],
    );
    assert!(matches!(result, HistoryWrite::Committed(_)), "{result:?}");
    let rows = ledger.observe_recordings(None).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].record.id, record(81).id);
    let metadata = rows[0].transcription.as_ref().unwrap();
    assert_eq!(metadata.engines.len(), 2);
    assert_eq!(metadata.engines[1].name, "second");
    assert_eq!(metadata.engines[0].name, "checker");
    assert_eq!(metadata.language, "fr-CA");
    assert!(!metadata.from_checker);
    assert_eq!(rows[0].flags.len(), 1);
    assert_eq!(rows[0].diagnostics.len(), 1);
    assert_eq!(rows[0].artifacts.len(), 1);
    assert_eq!(rows[0].artifacts[0].kind, ArtifactKind::Audio);
    assert!(rows[1].transcription.is_none());
    assert_eq!(
        ledger
            .observe_recordings(Some(&record(82).id))
            .unwrap()
            .len(),
        1
    );
    assert!(
        ledger
            .observe_recordings(Some(&record(83).id))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn observation_metadata_capacity_refuses_large_rows() {
    let ledger = MemoryLedger::new();
    let mut row = record(81);
    row.title = Some("x".repeat(16 * 1024 * 1024));
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row],
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        ledger.observe_recordings(None),
        Err(LedgerError::Corrupt(_))
    ));
    assert_eq!(ledger.recordings().unwrap().len(), 1);
}
