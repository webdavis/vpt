use super::*;

fn moved() -> (WorkDelta, Vec<(UtcInstant, HistoryFact)>) {
    let mut delta = delta();
    let change = &mut delta.notes[0];
    change.expected_saved = Some(artifact().digest);
    change.relocated_from = Some(artifact().target);
    change.artifact.target = "/notes/renamed.md".into();
    delta.commit.publications[0].target = change.artifact.target.clone();
    delta.commit.publications[0].expected_previous = Some(hash(b"operator edited bytes"));
    let mut events = facts().to_vec();
    events.push((
        change.artifact.updated_at,
        HistoryFact::NoteMoved {
            recording: change.artifact.recording.clone(),
            from: artifact().target,
            to: change.artifact.target.clone(),
        },
    ));
    (delta, events)
}

pub(crate) fn relocation_commits_pin_bytes_and_one_history_fact_atomically(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &facts()));
    ledger.clear_publication(&artifact().target).unwrap();
    let (change, facts) = moved();
    let mut failing = facts.clone();
    failing.push((
        artifact().updated_at,
        HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &change, &failing),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.note_at(&artifact().target).unwrap(),
        Some(artifact())
    );
    assert_eq!(
        ledger
            .note_path(&artifact().recording, NoteStage::Transcript)
            .unwrap(),
        Some(artifact().target)
    );
    assert_eq!(
        ledger.note_at(&change.notes[0].artifact.target).unwrap(),
        None
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
    committed(ledger.commit_step(run, &change, &facts));
    assert_eq!(ledger.note_at(&artifact().target).unwrap(), None);
    assert_eq!(
        ledger.note_at(&change.notes[0].artifact.target).unwrap(),
        Some(change.notes[0].artifact.clone())
    );
    assert_eq!(
        ledger
            .note_path(&artifact().recording, NoteStage::Transcript)
            .unwrap(),
        Some(change.notes[0].artifact.target.clone())
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        change.commit.publications
    );
    assert_eq!(
        ledger
            .pending_exports()
            .unwrap()
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::NoteMoved { .. }))
            .count(),
        1
    );
}

pub(crate) fn relocation_refuses_missing_history_changed_pins_and_reservations(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal + RetentionJournal),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &facts()));
    let (change, events) = moved();
    assert!(matches!(
        ledger.commit_step(run, &change, &events),
        HistoryWrite::Unconfirmed(_)
    ));
    ledger.clear_publication(&artifact().target).unwrap();
    for variant in 0..5 {
        let (mut change, mut events) = moved();
        match variant {
            0 => {
                events.pop();
            }
            1 => change.notes[0].relocated_from = Some("/notes/wrong.md".into()),
            2 => change.notes[0].expected_saved = Some(hash(b"stale")),
            3 => events.push(events[1].clone()),
            _ => {
                change.notes[0].artifact.target = "/foreign/renamed.md".into();
                change.commit.publications[0].target = change.notes[0].artifact.target.clone();
                let HistoryFact::NoteMoved { to, .. } = &mut events[1].1 else {
                    unreachable!()
                };
                *to = change.notes[0].artifact.target.clone();
            }
        }
        assert!(
            matches!(
                ledger.commit_step(run, &change, &events),
                HistoryWrite::Unconfirmed(_)
            ),
            "variant {variant}"
        );
    }
    for path in [artifact().target, change.notes[0].artifact.target.clone()] {
        let id = ledger
            .record_intent(&NewIntent {
                kind: ArtifactKind::Audio,
                owner: (artifact().recording).into(),
                path,
                expected: artifact().digest,
                recorded_at: artifact().updated_at,
            })
            .unwrap();
        assert!(matches!(
            ledger.commit_step(run, &change, &events),
            HistoryWrite::Unconfirmed(_)
        ));
        ledger.complete_intent(id, UtcInstant { secs: 60 }).unwrap();
    }
    assert_eq!(
        ledger.note_at(&artifact().target).unwrap(),
        Some(artifact())
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}
