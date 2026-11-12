use super::*;

#[test]
fn summary_command_receives_the_local_capture_time_without_changing_its_input() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let config = fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("'{language}'", "'Captured {captured_at}.'");
    fs::write(sandbox.config_path(), config).unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let args = fs::read_to_string(sandbox.path().join("captured.args")).unwrap();
    assert_eq!(
        args.lines().nth(2).unwrap(),
        format!(
            "Captured {}.",
            record.captured_at.rfc3339_with(record.captured_offset)
        )
    );
    let transcript = ledger(&sandbox)
        .note(&record.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let document = vpt_adapters::note_document::Document::parse(&transcript.text).unwrap();
    assert_eq!(
        fs::read_to_string(sandbox.path().join("captured")).unwrap(),
        document.content().unwrap()
    );
}
