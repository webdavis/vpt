use super::*;

pub(crate) fn invalid_note_documents_refuse_without_state_or_history(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    for variant in 0..9 {
        let mut delta = delta();
        let note = &mut delta.notes[0].artifact;
        match variant {
            0 => note.digest = super::super::digest(99),
            1 => note.content_digest = super::super::digest(99),
            2 => {
                note.text = note.text.replace("vpt:links end", "vpt:links start");
                note.digest = hash(note.text.as_bytes());
            }
            3 => {
                note.text = note.text.replace("vptSchema: 1", "vptSchema: 2");
                note.digest = hash(note.text.as_bytes());
            }
            4 => note.managed_tags = vec!["managed".into(); 257],
            5 => note.target = "/notes/../foreign.md".into(),
            6 => note.write_offset.secs = 86_400,
            7 => note.managed_tags = vec!["not-in-note".into()],
            _ => note.stage = NoteStage::Analysis,
        }
        delta.commit.publications[0].intended = note.digest;
        let error = ledger.commit_step(run, &delta, &facts());
        assert!(
            matches!(error, HistoryWrite::Unconfirmed(_)),
            "{variant}: {error:?}"
        );
        assert!(!format!("{error:?}").contains("Private café"));
        assert_eq!(ledger.note_at(&artifact().target).unwrap(), None);
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}

pub(crate) fn foreign_targets_and_pending_retention_refuse_note_state(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal + RetentionJournal),
) {
    let run = seed(ledger);
    let target = artifact().target;
    ledger
        .record_intent(&NewIntent {
            kind: ArtifactKind::Audio,
            owner: (record(81).id).into(),
            path: target.clone(),
            expected: artifact().digest,
            recorded_at: UtcInstant { secs: 49 },
        })
        .unwrap();
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    ledger
        .complete_intent(
            ledger.pending_intents().unwrap()[0].id,
            UtcInstant { secs: 50 },
        )
        .unwrap();
    committed(ledger.commit_step(run, &delta(), &facts()));
    ledger.clear_publication(&target).unwrap();
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(82)],
            ..Default::default()
        })
        .unwrap();
    let mut changed = delta();
    let note = &mut changed.notes[0].artifact;
    note.text = note
        .text
        .replace(note.recording.as_str(), record(82).id.as_str());
    note.recording = record(82).id;
    note.digest = hash(note.text.as_bytes());
    changed.commit.publications[0].intended = note.digest;
    let facts = [(
        note.updated_at,
        HistoryFact::WorkStep {
            recording: Some(note.recording.clone()),
            outcome: WorkOutcome::Completed,
        },
    )];
    assert!(matches!(
        ledger.commit_step(run, &changed, &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.note_at(&target).unwrap(), Some(artifact()));
    assert!(
        ledger
            .note(&record(82).id, NoteStage::Transcript)
            .unwrap()
            .is_none()
    );
}
