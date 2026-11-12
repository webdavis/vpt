use super::speakers::{assert_no_leak, whisply};
use super::*;
use serde_json::{Value, json};
use std::fs;

fn word(text: &str, start: f64, speaker: &str) -> Value {
    json!({"word": text, "start": start, "end": start + 1.0, "speaker": speaker})
}

fn transcript(sandbox: &Sandbox, row: &vpt_application::ports::RecordingRecord) -> (String, Value) {
    let note = fs::read_to_string(notes::target(sandbox, row)).unwrap();
    let output = support::run(sandbox.vpt().args([
        "show",
        row.id.as_str(),
        "--stage",
        "transcript",
        "--json",
    ]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let shown: Value = serde_json::from_str(&stdout(&output)).unwrap();
    (note, shown["segments"].clone())
}

fn speakers(segments: &Value) -> Vec<Value> {
    segments
        .as_array()
        .unwrap()
        .iter()
        .map(|segment| segment.get("speaker").cloned().unwrap_or(Value::Null))
        .collect()
}

#[test]
fn whisply_speakers_label_each_turn_of_the_note_and_the_segments() {
    let (sandbox, row) = configured();
    let words = [
        word("bonjour", 0.0, "SPEAKER_01"),
        word("salut", 1.0, "SPEAKER_00"),
        word("merci", 2.0, "SPEAKER_01"),
    ];
    whisply(&sandbox, &words, "[engines]\nmain='whisply'", true);
    let output = run(&sandbox, row.id.as_str(), &[]);
    json(&output);
    let (note, segments) = transcript(&sandbox, &row);
    assert!(
        note.contains(
            "[00:00] Speaker 1: bonjour\n\n[00:01] Speaker 2: salut\n\n[00:02] Speaker 1: merci\n"
        ),
        "{note}"
    );
    assert_eq!(
        speakers(&segments),
        [json!("Speaker 1"), json!("Speaker 2"), json!("Speaker 1")]
    );
    assert_eq!(segments[1]["text"], "salut");
    assert_no_leak(&sandbox, &[&output]);
}

#[test]
fn one_speaker_reads_as_before() {
    let (sandbox, row) = configured();
    let words = [
        word("bonjour", 0.0, "SPEAKER_00"),
        word("salut", 1.0, "UNKNOWN"),
        word("merci", 2.0, "SPEAKER_00"),
    ];
    whisply(&sandbox, &words, "[engines]\nmain='whisply'", true);
    json(&run(&sandbox, row.id.as_str(), &[]));
    let (note, segments) = transcript(&sandbox, &row);
    assert!(!note.contains("Speaker"), "{note}");
    assert!(speakers(&segments).iter().all(Value::is_null));
}

#[test]
fn a_whisply_checker_labels_the_main_engine_transcript_by_time() {
    let (sandbox, row) = configured();
    let document = sandbox.path().join("engine.json");
    fs::write(&document, json!({
        "schema":"vpt.engine/1", "engine":{"name":"custom", "tool":"fixture", "version":"1", "model":"test", "family":"independent"},
        "language":"fr-CA", "local":true, "segments":[
            {"start":0.0,"end":1.0,"text":"Bonjour.", "words":[{"text":"Bonjour.","start":0.0,"end":1.0}]},
            {"start":1.0,"end":3.0,"text":"Salut, merci.", "words":[{"text":"Salut,","start":1.0,"end":2.0},{"text":"merci.","start":2.0,"end":3.0}]}]
    }).to_string()).unwrap();
    let main = format!(
        "[engines]\nmain='custom'\nchecker='whisply'\n[engines.custom]\nkind='command'\nfamily='independent'\nlocal=true\ncommand=[{:?}, '{{audio}}', '{{language}}', '{{out_dir}}', {:?}, {document:?}]",
        sandbox.path().join("bin/engine"),
        sandbox.path().join("calls"),
    );
    let words = [
        word("bonjour", 0.0, "SPEAKER_00"),
        word("salut", 1.0, "SPEAKER_01"),
        word("merci", 2.0, "SPEAKER_01"),
    ];
    whisply(&sandbox, &words, &main, true);
    let output = run(&sandbox, row.id.as_str(), &[]);
    json(&output);
    let (note, segments) = transcript(&sandbox, &row);
    assert!(
        note.contains("[00:00] Speaker 1: Bonjour.\n\n[00:01] Speaker 2: Salut, merci.\n"),
        "{note}"
    );
    assert_eq!(
        speakers(&segments),
        [json!("Speaker 1"), json!("Speaker 2")]
    );
    assert_no_leak(&sandbox, &[&output]);
}
