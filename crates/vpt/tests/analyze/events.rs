use super::*;

#[test]
fn a_finished_analysis_sends_one_analyzed_report() {
    let (sandbox, record) = note::configured();
    let capture = crate::report_hook::install(&sandbox);
    agent(&sandbox, "[report]\ncommand = ['report']\n");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let note = ledger(&sandbox)
        .note_path(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let events = crate::report_hook::events(&capture);
    assert_eq!(events.len(), 1, "{events:?}");
    let event = &events[0];
    assert_eq!(event["schema"], "vpt.event/1");
    assert_eq!(event["channel"], "report");
    assert_eq!(event["event"], "analyzed");
    assert_eq!(event["state"], "done");
    assert_eq!(event["recording"], record.id.as_str());
    assert_eq!(event["paths"]["note"], note.to_str().unwrap());
    assert!(
        !std::fs::read_to_string(&capture)
            .unwrap()
            .contains("private words")
    );
}
