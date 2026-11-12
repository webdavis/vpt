use super::{engine_outputs::committed, record};
use sha2::{Digest, Sha256};
mod analysis;
mod publication;
mod relocation;
pub(crate) use analysis::*;
pub(crate) use relocation::*;
mod refusals;
pub(crate) use publication::note_stage_completion_commits_with_publication_history;
pub(crate) use refusals::*;
use vpt_application::ports::*;
use vpt_domain::{
    digest::Sha256Digest,
    run::{HistoryFact, RunKind, WorkOutcome},
    time::{UtcInstant, UtcOffset},
};

pub(crate) fn artifact() -> NoteArtifact {
    let recording = record(81).id;
    let text = format!(
        "---\nvptSchema: 1\nvptRecording: {recording}\nvptStage: transcript\ntags: [managed, manual]\n---\n# Pinned\n<!-- vpt:links start -->\n<!-- vpt:links end -->\n<!-- vpt:content start -->\nPrivate café transcript\n<!-- vpt:content end -->\nOperator prose\n"
    );
    let content = crate::note_document::Document::parse(&text)
        .unwrap()
        .content()
        .unwrap()
        .to_owned();
    NoteArtifact {
        recording,
        stage: NoteStage::Transcript,
        target: "/notes/pinned.md".into(),
        digest: hash(text.as_bytes()),
        content_digest: hash(content.as_bytes()),
        text,
        managed_tags: vec!["managed".into()],
        first_written: UtcInstant { secs: 40 },
        write_offset: UtcOffset { secs: -21_600 },
        updated_at: UtcInstant { secs: 50 },
    }
}

pub(crate) fn hash(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}

pub(crate) fn delta() -> WorkDelta {
    let artifact = artifact();
    WorkDelta {
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: artifact.target.clone(),
                expected_previous: None,
                intended: artifact.digest,
                recorded_at: artifact.updated_at,
            }],
            ..Default::default()
        },
        notes: vec![NoteChange {
            relocated_from: None,
            artifact,
            expected_saved: None,
        }],
        ..Default::default()
    }
}

pub(crate) fn facts() -> [(UtcInstant, HistoryFact); 1] {
    [(
        artifact().updated_at,
        HistoryFact::WorkStep {
            recording: Some(record(81).id),
            outcome: WorkOutcome::Completed,
        },
    )]
}

pub(crate) fn seed(ledger: &(impl RecordingLedger + RunHistory)) -> vpt_domain::history::RunId {
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(81)],
            ..Default::default()
        })
        .unwrap();
    committed(ledger.begin_run(RunKind::Publish, UtcInstant { secs: 49 })).run
}

pub(crate) fn note_bytes_paths_and_history_commit_together(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    let receipt = committed(ledger.commit_step(run, &delta(), &facts()));
    let note = artifact();
    assert_eq!(
        ledger.note(&note.recording, note.stage).unwrap(),
        Some(note.clone())
    );
    assert_eq!(ledger.note_at(&note.target).unwrap(), Some(note.clone()));
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta().commit.publications
    );
    assert_eq!(receipt.completed, vec![note.recording]);
    let debug = format!("{:?} {:?}", delta(), ledger.pending_exports().unwrap());
    assert!(!debug.contains("Private café"));
    assert!(!debug.contains("Operator prose"));
}

pub(crate) fn missing_publication_or_history_refuses_note_state(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    for variant in 0..4 {
        let mut delta = delta();
        let mut facts = facts();
        match variant {
            0 => delta.commit.publications.clear(),
            1 => delta.commit.publications[0].intended = super::digest(9),
            2 => {
                facts[0].1 = HistoryFact::WorkStep {
                    recording: Some(record(80).id),
                    outcome: WorkOutcome::Completed,
                }
            }
            _ => {
                facts[0].1 = HistoryFact::WorkStep {
                    recording: Some(record(81).id),
                    outcome: WorkOutcome::Failed,
                }
            }
        }
        assert!(matches!(
            ledger.commit_step(run, &delta, &facts),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.note_at(&artifact().target).unwrap(), None);
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}

pub(crate) fn pinned_path_first_write_and_pending_bytes_cannot_be_replaced(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &facts()));
    let mut change = delta();
    change.notes[0].expected_saved = Some(artifact().digest);
    change.commit.publications[0].expected_previous = Some(artifact().digest);
    assert!(matches!(
        ledger.commit_step(run, &change, &facts()),
        HistoryWrite::Unconfirmed(_)
    ));
    ledger.clear_publication(&artifact().target).unwrap();
    for variant in 0..3 {
        let mut invalid = change.clone();
        match variant {
            0 => invalid.notes[0].expected_saved = None,
            1 => {
                invalid.notes[0].artifact.target = "/notes/renamed.md".into();
                invalid.commit.publications[0].target = "/notes/renamed.md".into();
            }
            _ => invalid.notes[0].artifact.first_written.secs += 1,
        }
        assert!(matches!(
            ledger.commit_step(run, &invalid, &facts()),
            HistoryWrite::Unconfirmed(_)
        ));
    }
    assert_eq!(
        ledger.note_at(&artifact().target).unwrap(),
        Some(artifact())
    );
}

pub(crate) fn rewrites_save_operator_bytes_and_only_content_changes_its_digest(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &facts()));
    ledger.clear_publication(&artifact().target).unwrap();
    let mut change = delta();
    change.notes[0].expected_saved = Some(artifact().digest);
    let note = &mut change.notes[0].artifact;
    note.text.push_str("More operator prose\n");
    note.digest = hash(note.text.as_bytes());
    change.commit.publications[0].intended = note.digest;
    change.commit.publications[0].expected_previous = Some(hash(b"operator edited input"));
    committed(ledger.commit_step(run, &change, &facts()));
    let saved = ledger.note_at(&artifact().target).unwrap().unwrap();
    assert_eq!(saved, change.notes[0].artifact);
    assert_eq!(saved.content_digest, artifact().content_digest);
    assert_ne!(saved.digest, artifact().digest);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(content_changes_leave_expired_analysis_expired);
        $case!(content_changes_leave_disabled_analysis_disabled);
        $case!(only_content_changes_make_succeeded_analysis_pending);
        $case!(relocation_commits_pin_bytes_and_one_history_fact_atomically);
        $case!(relocation_refuses_missing_history_changed_pins_and_reservations);
        $case!(note_stage_completion_commits_with_publication_history);
        $case!(note_bytes_paths_and_history_commit_together);
        $case!(missing_publication_or_history_refuses_note_state);
        $case!(pinned_path_first_write_and_pending_bytes_cannot_be_replaced);
        $case!(rewrites_save_operator_bytes_and_only_content_changes_its_digest);
        $case!(invalid_note_documents_refuse_without_state_or_history);
        $case!(foreign_targets_and_pending_retention_refuse_note_state);
        $case!(transcript_and_analysis_commit_with_one_recording_receipt);
    };
}
pub(crate) use cases;

pub(crate) fn transcript_and_analysis_commit_with_one_recording_receipt(
    ledger: &(impl RecordingLedger + RunHistory + NoteLedger + PublicationJournal),
) {
    let run = seed(ledger);
    let mut both = delta();
    let mut analysis = artifact();
    analysis.stage = NoteStage::Analysis;
    analysis.target = "/analysis/pinned.md".into();
    analysis.text = analysis
        .text
        .replace("vptStage: transcript", "vptStage: analysis");
    analysis.digest = hash(analysis.text.as_bytes());
    both.commit.publications.push(DirtyPublication {
        target: analysis.target.clone(),
        expected_previous: None,
        intended: analysis.digest,
        recorded_at: analysis.updated_at,
    });
    both.notes.push(NoteChange {
        relocated_from: None,
        artifact: analysis.clone(),
        expected_saved: None,
    });
    let receipt = committed(ledger.commit_step(run, &both, &facts()));
    assert_eq!(receipt.completed, vec![analysis.recording.clone()]);
    assert_eq!(
        ledger
            .note(&analysis.recording, NoteStage::Analysis)
            .unwrap(),
        Some(analysis)
    );
    assert_eq!(
        ledger
            .note(&artifact().recording, NoteStage::Transcript)
            .unwrap(),
        Some(artifact())
    );
    assert_eq!(ledger.pending_publications().unwrap().len(), 2);
}
