use super::*;

fn replacement(ledger: &impl Ledger, previous: &WorkDelta) -> WorkDelta {
    let mut next = accepted(ledger);
    next.accepted_at.secs = 80;
    next.proposal.by = "second private agent".into();
    let mut next = delta(next);
    let old = &previous.notes[0].artifact;
    next.notes[0].expected_saved = Some(old.digest);
    next.notes[0].artifact.first_written = old.first_written;
    next.commit.publications[0].expected_previous = Some(old.digest);
    next
}

pub(crate) fn later_success_replaces_accepted_state_and_reports_one_recording(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let first = delta(accepted(ledger));
    committed(ledger.commit_step(run, &first, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &first.notes[0].artifact.target);
    let next = replacement(ledger, &first);
    let receipt = committed(ledger.commit_step(run, &next, &[fact(80, WorkOutcome::Completed)]));
    let ProposalChange::Accept(expected) = next.proposal.as_ref().unwrap() else {
        unreachable!()
    };
    assert_eq!(
        ledger.accepted_proposal(&record(81).id).unwrap().as_ref(),
        Some(expected.as_ref())
    );
    assert_eq!(receipt.completed, vec![record(81).id]);
    assert_eq!(
        ledger.pending_publications().unwrap(),
        next.commit.publications
    );
}

pub(crate) fn late_history_failure_preserves_prior_proposal_note_stage_and_journal(
    ledger: &impl Ledger,
) {
    let run = seed(ledger);
    let old = accepted(ledger);
    let first = delta(old.clone());
    committed(ledger.commit_step(run, &first, &[fact(70, WorkOutcome::Completed)]));
    finish(ledger, run, &first.notes[0].artifact.target);
    let before = ledger.pending_exports().unwrap();
    let bad = HistoryFact::HookFailed {
        event: vpt_domain::history::EventId::new(run.namespace(), 999_999),
        failure: vpt_domain::run::HookFailureClass::Spawn,
    };
    let result = ledger.commit_step(
        run,
        &replacement(ledger, &first),
        &[
            fact(80, WorkOutcome::Completed),
            (UtcInstant { secs: 80 }, bad),
        ],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
    assert_eq!(
        ledger.accepted_proposal(&old.recording).unwrap(),
        Some(old.clone())
    );
    assert_eq!(
        ledger.note(&old.recording, NoteStage::Analysis).unwrap(),
        Some(first.notes[0].artifact.clone())
    );
    assert_eq!(
        ledger
            .by_id(&old.recording)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Succeeded
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap(), before);
}

pub(crate) fn proposal_requires_exact_matching_history(ledger: &impl Ledger) {
    let run = seed(ledger);
    let next = delta(accepted(ledger));
    let before = ledger.pending_exports().unwrap();
    for facts in [
        vec![],
        vec![fact(69, WorkOutcome::Completed)],
        vec![fact(70, WorkOutcome::Failed)],
        vec![
            fact(70, WorkOutcome::Completed),
            fact(70, WorkOutcome::Completed),
        ],
        vec![(
            UtcInstant { secs: 70 },
            HistoryFact::WorkStep {
                recording: Some(record(82).id),
                outcome: WorkOutcome::Completed,
            },
        )],
    ] {
        assert!(matches!(
            ledger.commit_step(run, &next, &facts),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.accepted_proposal(&record(81).id).unwrap().is_none());
        assert!(
            ledger
                .note(&record(81).id, NoteStage::Analysis)
                .unwrap()
                .is_none()
        );
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
}
