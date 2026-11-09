use super::*;

pub(crate) fn checker_metadata_must_match_the_published_output(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    for variant in 0..4 {
        let mut changed = delta();
        let mut checker = artifact();
        checker.engine = "comparison".into();
        checker.target = format!("/outputs/{}.comparison.json", checker.recording).into();
        let mut value: serde_json::Value = serde_json::from_slice(&checker.bytes).unwrap();
        value["engine"]["name"] = "comparison".into();
        checker.bytes = serde_json::to_vec(&value).unwrap();
        checker.digest = vpt_domain::digest::Sha256Digest(Sha256::digest(&checker.bytes).into());
        changed.engine_outputs.push(checker.clone());
        changed.commit.publications.push(DirtyPublication {
            target: checker.target.clone(),
            expected_previous: None,
            intended: checker.digest,
            recorded_at: checker.created_at,
        });
        let TranscriptionChange::Accept(accepted) = changed.transcription.as_mut().unwrap() else {
            unreachable!()
        };
        accepted.from_checker = false;
        let mut metadata = accepted.transcript.engine.clone();
        metadata.name = "comparison".into();
        match variant {
            0 => metadata.model = "wrong-model".into(),
            1 => metadata.name = "wrong-name".into(),
            2 => {
                changed.engine_outputs.pop();
                changed.commit.publications.pop();
            }
            _ => (),
        }
        accepted.checker_engine = (variant != 3).then_some(metadata);
        assert!(
            matches!(
                ledger.commit_step(run, &changed, &fact()),
                HistoryWrite::Unconfirmed(_)
            ),
            "variant {variant}"
        );
        assert!(
            ledger
                .accepted_transcript(&fixture().recording)
                .unwrap()
                .is_none()
        );
        assert!(ledger.engine_output(&checker.target).unwrap().is_none());
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}
