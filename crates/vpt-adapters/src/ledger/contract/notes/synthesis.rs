use super::*;

pub(crate) fn only_content_changes_make_succeeded_synthesis_pending(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    check(ledger, StageState::Succeeded);
}

pub(crate) fn content_changes_leave_expired_synthesis_expired(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    check(ledger, StageState::Expired);
}

pub(crate) fn content_changes_leave_disabled_synthesis_disabled(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    check(ledger, StageState::Disabled);
}

fn check(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
    stage: StageState,
) {
    let seed = 81;
    let mut row = record(seed);
    row.stages.synthesis = stage;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Note, UtcInstant { secs: 49 })).run;
    let mut note = artifact();
    note.text = note.text.replace(note.recording.as_str(), row.id.as_str());
    note.recording = row.id.clone();
    note.target = format!("/notes/{seed}.md").into();
    let mut previous = None;
    for phase in 0..3 {
        match phase {
            1 => note.text.push_str("Operator ending\n"),
            2 => {
                note.text = note
                    .text
                    .replace("Private café transcript", "Different content")
            }
            _ => (),
        }
        note.digest = hash(note.text.as_bytes());
        note.content_digest = hash(
            crate::note_document::Document::parse(&note.text)
                .unwrap()
                .content()
                .unwrap()
                .as_bytes(),
        );
        let delta = WorkDelta {
            notes: vec![NoteChange {
                artifact: note.clone(),
                expected_saved: previous,
                relocated_from: None,
            }],
            commit: LedgerCommit {
                publications: vec![DirtyPublication {
                    target: note.target.clone(),
                    expected_previous: previous,
                    intended: note.digest,
                    recorded_at: note.updated_at,
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        if phase == 2 {
            let failed = [
                (
                    note.updated_at,
                    HistoryFact::WorkStep {
                        recording: Some(row.id.clone()),
                        outcome: WorkOutcome::Completed,
                    },
                ),
                (
                    note.updated_at,
                    HistoryFact::HookFailed {
                        event: vpt_domain::history::EventId::new(run.namespace(), 999),
                        failure: vpt_domain::run::HookFailureClass::Spawn,
                    },
                ),
            ];
            assert!(matches!(
                ledger.commit_step(run, &delta, &failed),
                HistoryWrite::Unconfirmed(_)
            ));
            assert_eq!(
                ledger.by_id(&row.id).unwrap().unwrap().stages.synthesis,
                stage
            );
            assert!(ledger.pending_publications().unwrap().is_empty());
        }
        committed(ledger.commit_step(
            run,
            &delta,
            &[(
                note.updated_at,
                HistoryFact::WorkStep {
                    recording: Some(row.id.clone()),
                    outcome: WorkOutcome::Completed,
                },
            )],
        ));
        let expected = if phase == 2 && stage == StageState::Succeeded {
            StageState::Pending
        } else {
            stage
        };
        assert_eq!(
            ledger.by_id(&row.id).unwrap().unwrap().stages.synthesis,
            expected
        );
        ledger.clear_publication(&note.target).unwrap();
        previous = Some(note.digest);
    }
}
