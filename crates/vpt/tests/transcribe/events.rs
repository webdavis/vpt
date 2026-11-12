use super::*;
use vpt_application::ports::NoteLedger;

#[test]
fn a_finished_transcription_sends_one_transcribed_report() {
    let (sandbox, row) = configured();
    let capture = crate::report_hook::install(&sandbox);
    let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    std::fs::write(
        sandbox.config_path(),
        config + "[report]\ncommand = ['report']\n",
    )
    .unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let note = ledger(&sandbox)
        .note_path(&row.id, vpt_application::ports::NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let events = crate::report_hook::events(&capture);
    assert_eq!(events.len(), 1, "{events:?}");
    let event = &events[0];
    assert_eq!(event["schema"], "vpt.event/1");
    assert_eq!(event["channel"], "report");
    assert_eq!(event["event"], "transcribed");
    assert_eq!(event["state"], "done");
    assert_eq!(event["recording"], row.id.as_str());
    assert_eq!(event["paths"]["note"], note.to_str().unwrap());
    assert!(
        !std::fs::read_to_string(&capture)
            .unwrap()
            .contains("canary")
    );
}
