use super::*;
use crate::ledger::contract::engine_outputs::committed;
use vpt_domain::time::UtcInstant;

fn change() -> FlagResolutionChange {
    FlagResolutionChange {
        recording: accepted().recording,
        flag: crate::lexical_flag_id(&accepted().recording, &accepted().uncertainty.lexical[0]),
        expected: Resolution::Open,
        resolution: Resolution::Corrected {
            text: "Private correction canary".into(),
            at: UtcInstant { secs: 60 },
        },
    }
}
fn facts() -> Vec<(UtcInstant, HistoryFact)> {
    vec![
        (
            UtcInstant { secs: 60 },
            HistoryFact::WorkStep {
                recording: Some(accepted().recording),
                outcome: WorkOutcome::Completed,
            },
        ),
        (
            UtcInstant { secs: 60 },
            HistoryFact::FlagResolved {
                recording: accepted().recording,
                flag: change().flag,
                resolution: vpt_domain::review::ResolutionKind::Corrected,
            },
        ),
    ]
}

pub(crate) fn corrections_commit_with_history_without_changing_accepted_words(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    committed(save(ledger, run, accepted()));
    let delta = WorkDelta {
        resolution: Some(change()),
        ..Default::default()
    };
    let receipt = committed(ledger.commit_step(run, &delta, &facts()));
    let flags = ledger.transcript_flags(&accepted().recording).unwrap();
    assert_eq!(
        flags
            .iter()
            .find(|flag| flag.identifier == change().flag)
            .unwrap()
            .resolution,
        change().resolution
    );
    assert_eq!(
        ledger.accepted_transcript(&accepted().recording).unwrap(),
        Some(accepted())
    );
    assert_eq!(receipt.completed, vec![accepted().recording]);
    assert!(
        !format!("{:?} {:?}", receipt, ledger.pending_exports().unwrap())
            .contains("Private correction canary")
    );
    committed(save(ledger, run, accepted()));
    let flags = ledger.transcript_flags(&accepted().recording).unwrap();
    assert_eq!(
        flags
            .iter()
            .find(|flag| flag.identifier == change().flag)
            .unwrap()
            .resolution,
        change().resolution
    );
}

pub(crate) fn resolution_failures_leave_flags_history_and_accepted_state_unchanged(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    check_resolution_failures(ledger, 0..5);
}

pub(crate) fn resolution_history_failures_leave_flags_history_and_accepted_state_unchanged(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    check_resolution_failures(ledger, 5..11);
}

fn check_resolution_failures(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
    variants: std::ops::Range<usize>,
) {
    let run = seed(ledger);
    committed(save(ledger, run, accepted()));
    let before = ledger.transcript_flags(&accepted().recording).unwrap();
    for variant in variants {
        let mut change = change();
        let mut events = facts();
        match variant {
            0 => change.flag = super::super::digest(99),
            1 => change.recording = super::super::record(82).id,
            2 => {
                change.expected = Resolution::Dismissed {
                    at: UtcInstant { secs: 59 },
                }
            }
            3 => change.resolution = Resolution::Open,
            4 => {
                change.resolution = Resolution::Corrected {
                    text: " ".into(),
                    at: UtcInstant { secs: 60 },
                }
            }
            5 => events.clear(),
            6 => {
                events[0].1 = HistoryFact::WorkStep {
                    recording: Some(accepted().recording),
                    outcome: WorkOutcome::Failed,
                }
            }
            7 => {
                events.pop();
            }
            8 => events.push(events[1].clone()),
            9 => events[1].0.secs += 1,
            _ => events.push((
                UtcInstant { secs: 60 },
                HistoryFact::HookFailed {
                    event: vpt_domain::history::EventId::new(run.namespace(), 999),
                    failure: vpt_domain::run::HookFailureClass::Spawn,
                },
            )),
        }
        let result = ledger.commit_step(
            run,
            &WorkDelta {
                resolution: Some(change),
                ..Default::default()
            },
            &events,
        );
        assert!(
            matches!(result, HistoryWrite::Unconfirmed(_)),
            "variant {variant}: {result:?}"
        );
        assert_eq!(
            ledger.transcript_flags(&accepted().recording).unwrap(),
            before
        );
        assert_eq!(
            ledger.accepted_transcript(&accepted().recording).unwrap(),
            Some(accepted())
        );
        assert_eq!(ledger.pending_exports().unwrap().len(), 2);
    }
}

pub(crate) fn confirmation_and_dismissal_retain_explicit_state(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    committed(save(ledger, run, accepted()));
    let mut previous = Resolution::Open;
    for resolution in [
        Resolution::Confirmed {
            at: UtcInstant { secs: 60 },
        },
        Resolution::Dismissed {
            at: UtcInstant { secs: 70 },
        },
    ] {
        let mut change = change();
        change.expected = previous;
        change.resolution = resolution.clone();
        let mut facts = facts();
        for (at, fact) in &mut facts {
            *at = resolution.at().unwrap();
            if let HistoryFact::FlagResolved {
                resolution: kind, ..
            } = fact
            {
                *kind = resolution.kind().unwrap();
            }
        }
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                resolution: Some(change.clone()),
                ..Default::default()
            },
            &facts,
        ));
        assert_eq!(
            ledger
                .transcript_flags(&accepted().recording)
                .unwrap()
                .iter()
                .find(|flag| flag.identifier == change.flag)
                .unwrap()
                .resolution,
            resolution
        );
        assert_eq!(
            ledger.accepted_transcript(&accepted().recording).unwrap(),
            Some(accepted())
        );
        previous = resolution;
    }
}

pub(crate) fn resolved_flags_and_rendered_note_commit_or_roll_back_together(
    ledger: &(
         impl RecordingLedger
         + RunHistory
         + TranscriptLedger
         + FlagLedger
         + ReviewLedger
         + NoteLedger
         + PublicationJournal
         + VocabularyLedger
     ),
) {
    use crate::ledger::contract::notes;
    let run = seed(ledger);
    committed(save(ledger, run, accepted()));
    committed(ledger.commit_step(run, &notes::delta(), &notes::facts()));
    let old = notes::artifact();
    ledger.clear_publication(&old.target).unwrap();
    let mut snapshot = ledger
        .review_snapshot(&accepted().recording)
        .unwrap()
        .unwrap();
    let prepared = vpt_application::review::resolve::prepare(
        &mut snapshot,
        &accepted().recording,
        change().flag,
        &vpt_domain::review::ReviewAction::Correct("Private correction canary".into()),
        UtcInstant { secs: 60 },
    )
    .unwrap();
    assert_eq!(
        prepared.known_term.as_deref(),
        Some("Private correction canary")
    );
    let mut terms = crate::ledger::contract::known_terms::publication();
    terms.bytes = format!("{}\n", prepared.known_term.as_ref().unwrap()).into_bytes();
    terms.entry.intended = notes::hash(&terms.bytes);
    terms.entry.recorded_at = UtcInstant { secs: 60 };
    let content = vpt_domain::note::transcript_content(
        &snapshot.accepted.unwrap().transcript,
        &snapshot.flags,
        false,
    )
    .unwrap();
    let mut updated = old.clone();
    updated.text = updated.text.replace("Private café transcript", &content);
    updated.updated_at = UtcInstant { secs: 60 };
    updated.digest = notes::hash(updated.text.as_bytes());
    updated.content_digest = notes::hash(
        crate::note_document::Document::parse(&updated.text)
            .unwrap()
            .content()
            .unwrap()
            .as_bytes(),
    );
    let delta = WorkDelta {
        vocabulary: Some(terms.clone()),
        resolution: prepared.change,
        notes: vec![NoteChange {
            artifact: updated.clone(),
            expected_saved: Some(old.digest),
            relocated_from: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: old.target.clone(),
                expected_previous: Some(old.digest),
                intended: updated.digest,
                recorded_at: updated.updated_at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let mut events = facts();
    events.push((
        terms.entry.recorded_at,
        HistoryFact::EffectIntent {
            kind: vpt_domain::run::EffectKind::Publication,
            reference: vpt_domain::run::EffectReference::PublicationTarget(
                terms.entry.target.clone(),
            ),
        },
    ));
    let mut failed = events.clone();
    failed.push((
        updated.updated_at,
        HistoryFact::HookFailed {
            event: vpt_domain::history::EventId::new(run.namespace(), 999),
            failure: vpt_domain::run::HookFailureClass::Spawn,
        },
    ));
    assert!(matches!(
        ledger.commit_step(run, &delta, &failed),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.note_at(&old.target).unwrap(), Some(old.clone()));
    assert!(
        ledger
            .transcript_flags(&accepted().recording)
            .unwrap()
            .iter()
            .all(|flag| flag.resolution == Resolution::Open)
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(
        ledger
            .vocabulary_publication(&terms.entry.target)
            .unwrap()
            .is_none()
    );
    let receipt = committed(ledger.commit_step(run, &delta, &events));
    assert_eq!(
        ledger.vocabulary_publication(&terms.entry.target).unwrap(),
        Some(terms)
    );
    assert_eq!(receipt.completed, vec![accepted().recording]);
    assert_eq!(ledger.note_at(&old.target).unwrap(), Some(updated.clone()));
    assert!(
        updated
            .text
            .contains("[corrected: Private correction canary]")
    );
    assert_ne!(updated.content_digest, old.content_digest);
    assert_eq!(
        ledger.accepted_transcript(&accepted().recording).unwrap(),
        Some(accepted())
    );
}
