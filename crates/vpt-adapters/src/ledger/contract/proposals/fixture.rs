use super::*;

pub(crate) fn seed(ledger: &impl Ledger) -> RunId {
    let run = notes::seed(ledger);
    let mut transcript = transcript::fixture();
    transcript.duration = 2.0;
    transcript.uncertainty.lexical.push(LexicalFlag {
        class: Class::ProperNoun,
        ranges: std::iter::once(0..1).collect(),
        record_text: "Privé".into(),
        alternative_text: Some("Privée".into()),
        confidence: Some(0.91),
    });
    let content = vpt_domain::note::transcript_content(
        &transcript.transcript,
        &crate::ledger::flags::prepare(&transcript).unwrap(),
        false,
    )
    .unwrap();
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(transcript))),
            ..Default::default()
        },
        &[fact(50, WorkOutcome::Completed)],
    ));
    let mut delta = notes::delta();
    let note = &mut delta.notes[0].artifact;
    replace_content(note, &content);
    delta.commit.publications[0].intended = note.digest;
    committed(ledger.commit_step(run, &delta, &[fact(50, WorkOutcome::Completed)]));
    finish(ledger, run, &delta.notes[0].artifact.target);
    run
}

pub(crate) fn accepted(ledger: &impl Ledger) -> AcceptedProposal {
    let id = record(81).id;
    let transcript = ledger.accepted_transcript(&id).unwrap().unwrap();
    let flags = ledger.transcript_flags(&id).unwrap();
    let proposal = Proposal {
        by: "private agent".into(),
        summary: vec![Claim {
            text: "Privé".into(),
            source_references: vec!["00:00-00:01".into()],
        }],
        actions: vec![
            Claim {
                text: "a private task".into(),
                source_references: vec![],
            }
            .into(),
        ],
        tags: vec![Tag {
            tag: "private-tag".into(),
            reason: "private reason".into(),
        }],
        relations: vec![Related {
            target: "private target".into(),
            reason: "private relation".into(),
        }],
    };
    let proposal = Grounder::new(
        transcript.transcript.segments.iter().flat_map(|s| &s.words),
        &flags,
        transcript.duration,
        0,
    )
    .unwrap()
    .proposal(proposal)
    .unwrap();
    AcceptedProposal {
        vocabulary_publication: None,
        recording: id.clone(),
        input_digest: ledger
            .note(&id, NoteStage::Transcript)
            .unwrap()
            .unwrap()
            .content_digest,
        basis: ledger
            .grounding_basis(&id)
            .unwrap()
            .expect("accepted words and flags supply a grounding basis"),
        grounding_window_secs: 0,
        proposal,
        accepted_at: UtcInstant { secs: 70 },
    }
}

pub(crate) fn delta(accepted: AcceptedProposal) -> WorkDelta {
    let content = vpt_domain::note::analysis_content(&accepted.proposal).unwrap();
    let mut note = notes::artifact();
    note.stage = NoteStage::Analysis;
    note.target = "/analysis/pinned.md".into();
    let record = record(81);
    note.text = vpt_domain::note::document::create(&vpt_domain::note::document::RecordingNote {
        stage: NoteStage::Analysis,
        recording: &record.id,
        filename: "pinned.md",
        captured_at: record.captured_at,
        offset: record.captured_offset,
        duration_secs: record.duration_secs,
        language: "en-US",
        engines: &[],
        readability: false,
        open_flags: 0,
        diagnostics: 0,
        tags: &note.managed_tags,
        suggested_tags: &[],
        profile: vpt_domain::note::document::Profile::Portable,
        links: "- Transcript: [pinned](../notes/pinned.md)\n",
        content: &content,
    })
    .unwrap();
    note.updated_at = accepted.accepted_at;
    note.first_written = accepted.accepted_at;
    replace_content(&mut note, &content);
    WorkDelta {
        proposal: Some(ProposalChange::Accept(Box::new(accepted))),
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: note.target.clone(),
                expected_previous: None,
                intended: note.digest,
                recorded_at: note.updated_at,
            }],
            ..Default::default()
        },
        notes: vec![NoteChange {
            relocated_from: None,
            artifact: note,
            expected_saved: None,
        }],
        ..Default::default()
    }
}

pub(crate) fn replace_content(note: &mut NoteArtifact, content: &str) {
    let parsed = crate::note_document::Document::parse(&note.text).unwrap();
    let old = parsed.content().unwrap().to_owned();
    note.text = note.text.replace(&old, content);
    note.digest = notes::hash(note.text.as_bytes());
    note.content_digest = notes::hash(content.as_bytes());
}

pub(crate) fn fact(at: i64, outcome: WorkOutcome) -> (UtcInstant, HistoryFact) {
    (
        UtcInstant { secs: at },
        HistoryFact::WorkStep {
            recording: Some(record(81).id),
            outcome,
        },
    )
}

pub(crate) fn finish(ledger: &impl Ledger, run: RunId, target: &std::path::Path) {
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            publication_clears: vec![target.into()],
            ..Default::default()
        },
        &[fact(71, WorkOutcome::Completed)],
    ));
}
