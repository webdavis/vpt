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

#[test]
fn sending_a_recordings_actions_sends_one_actions_sent_report_with_counts_only() {
    let (sandbox, record) = note::configured();
    let capture = crate::report_hook::install(&sandbox);
    let log = crate::actions::configure_with(&sandbox, "[report]\ncommand = ['report']\n");
    std::fs::write(log.with_extension("fail"), "1").unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    std::fs::remove_file(log.with_extension("fail")).unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let note = ledger(&sandbox)
        .note_path(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let sends: Vec<_> = crate::report_hook::events(&capture)
        .into_iter()
        .filter(|event| event["event"] == "actions_sent")
        .collect();
    assert_eq!(sends.len(), 2, "{sends:?}");
    for (event, state, detail) in [
        (&sends[0], "failed", "0 sent, 4 failed, 0 refused"),
        (&sends[1], "done", "4 sent, 0 failed, 0 refused"),
    ] {
        assert_eq!(event["channel"], "report");
        assert_eq!(event["state"], state);
        assert_eq!(event["detail"], detail);
        assert_eq!(event["recording"], record.id.as_str());
        assert_eq!(event["paths"]["note"], note.to_str().unwrap());
    }
    assert!(
        !std::fs::read_to_string(&capture)
            .unwrap()
            .contains("secret")
    );
}
