use super::*;

pub(crate) fn verification_windows_survive_empty_results_and_artifact_replacement(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    analysis(ledger, run);
    let id = record(81).id;
    let original = ledger
        .review_snapshot(&id)
        .unwrap()
        .unwrap()
        .verified_claims;
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].grounding_window_secs, 0);
    let mut selected = change(ledger);
    selected.findings.clear();
    for window in [0, u32::MAX] {
        selected.grounding_window_secs = window;
        committed(save(ledger, run, selected.clone()));
        let snapshot = ledger.review_snapshot(&id).unwrap().unwrap();
        assert!(snapshot.stale_artifacts.is_empty());
        let transcript = snapshot
            .verified_claims
            .iter()
            .find(|claims| claims.artifact == NoteStage::Transcript)
            .unwrap();
        assert_eq!(transcript.grounding_window_secs, window);
        assert!(snapshot.verified_claims.contains(&original[0]));
        assert_eq!(ledger.note_inputs(&id).unwrap().unwrap().review, snapshot);
    }
}
