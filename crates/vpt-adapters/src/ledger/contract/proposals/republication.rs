use super::*;
use vpt_domain::review::{Resolution, ResolutionKind};

pub(crate) fn analysis_republication_keeps_changed_transcript_content_pending(
    ledger: &impl Ledger,
) {
    check(ledger, false);
}
pub(crate) fn analysis_republication_keeps_changed_lexical_basis_pending(ledger: &impl Ledger) {
    check(ledger, true);
}
fn check(ledger: &impl Ledger, lexical: bool) {
    let run = seed(ledger);
    let first = delta(accepted(ledger));
    committed(ledger.commit_step(run, &first, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &first.notes[0].artifact.target);
    let at = UtcInstant { secs: 80 };
    if lexical {
        let flag = ledger.transcript_flags(&record(81).id).unwrap().remove(0);
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                resolution: Some(FlagResolutionChange {
                    recording: record(81).id,
                    flag: flag.identifier,
                    expected: Resolution::Open,
                    resolution: Resolution::Confirmed { at },
                }),
                ..Default::default()
            },
            &[
                fact(80, WorkOutcome::Completed),
                (
                    at,
                    HistoryFact::FlagResolved {
                        recording: record(81).id,
                        flag: flag.identifier,
                        resolution: ResolutionKind::Confirmed,
                    },
                ),
            ],
        ));
    } else {
        let mut source = ledger
            .note(&record(81).id, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        let old = source.digest;
        replace_content(&mut source, "## Transcript\n\n[00:00] changed content\n");
        source.updated_at = at;
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                notes: vec![NoteChange {
                    artifact: source.clone(),
                    expected_saved: Some(old),
                    relocated_from: None,
                }],
                commit: LedgerCommit {
                    publications: vec![DirtyPublication {
                        target: source.target.clone(),
                        expected_previous: Some(old),
                        intended: source.digest,
                        recorded_at: at,
                    }],
                    ..Default::default()
                },
                ..Default::default()
            },
            &[fact(80, WorkOutcome::Completed)],
        ));
        finish(ledger, run, &source.target);
    }
    let old = &first.notes[0].artifact;
    let mut note = old.clone();
    note.text.push_str("\nOperator ending\n");
    note.digest = notes::hash(note.text.as_bytes());
    note.updated_at = at;
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            notes: vec![NoteChange {
                artifact: note.clone(),
                expected_saved: Some(old.digest),
                relocated_from: None,
            }],
            commit: LedgerCommit {
                publications: vec![DirtyPublication {
                    target: note.target.clone(),
                    expected_previous: Some(old.digest),
                    intended: note.digest,
                    recorded_at: at,
                }],
                ..Default::default()
            },
            ..Default::default()
        },
        &[fact(80, WorkOutcome::Completed)],
    ));
    finish(ledger, run, &note.target);
    assert_eq!(
        ledger
            .by_id(&record(81).id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Pending
    );
}

pub(crate) fn analysis_metadata_publication_preserves_a_failed_synthesis_attempt(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let first = delta(accepted(ledger));
    committed(ledger.commit_step(run, &first, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &first.notes[0].artifact.target);
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            proposal: Some(ProposalChange::Fail {
                recording: record(81).id,
                at: UtcInstant { secs: 80 },
            }),
            ..Default::default()
        },
        &[fact(80, WorkOutcome::Failed)],
    ));
    let mut next = first.clone();
    next.proposal = None;
    next.notes[0].expected_saved = Some(first.notes[0].artifact.digest);
    next.notes[0].artifact.updated_at.secs = 90;
    next.commit.publications[0].expected_previous = Some(first.notes[0].artifact.digest);
    next.commit.publications[0].recorded_at.secs = 90;
    committed(ledger.commit_step(run, &next, &[fact(90, WorkOutcome::Completed)]));
    finish(ledger, run, &next.notes[0].artifact.target);
    assert_eq!(
        ledger
            .by_id(&record(81).id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Failed
    );
}
