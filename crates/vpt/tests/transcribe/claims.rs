use super::speakers::whisply;
use super::*;
use serde_json::{Value, json};
use std::fs;
use vpt_application::ports::{NoteLedger, NoteStage};

fn labeled(sandbox: &Sandbox, row: &vpt_application::ports::RecordingRecord) {
    let words = [
        json!({"word":"bonjour","start":0.0,"end":1.0,"speaker":"SPEAKER_01"}),
        json!({"word":"salut","start":1.0,"end":2.0,"speaker":"SPEAKER_00"}),
        json!({"word":"merci","start":2.0,"end":3.0,"speaker":"SPEAKER_01"}),
    ];
    whisply(sandbox, &words, "[engines]\nmain='whisply'", true);
    json(&run(sandbox, row.id.as_str(), &[]));
}

fn analyze(sandbox: &Sandbox, id: &str, proposal: Value) -> Value {
    let script = sandbox.path().join("agent.sh");
    let path = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    fs::write(&path, proposal.to_string()).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        format!("{config}\n[analysis]\ncommand=['/bin/sh', {script:?}, {path:?}]\n"),
    )
    .unwrap();
    json(&support::run(sandbox.vpt().args(["analyze", id, "--json"])));
    json(&support::run(
        sandbox
            .vpt()
            .args(["show", id, "--stage", "analysis", "--json"]),
    ))
}

fn line(text: &str, sources: &str) -> Value {
    json!({"text": text, "sources": [sources]})
}

#[test]
fn a_claim_may_name_the_speaker_of_a_segment_it_cites_and_no_other() {
    let (sandbox, row) = configured();
    labeled(&sandbox, &row);
    let mut action = line("Speaker 2 will send the report", "00:01-00:02");
    action["due"] = json!({"date": "2026-11-19", "sources": ["00:01-00:02"]});
    let shown = analyze(
        &sandbox,
        row.id.as_str(),
        json!({"schema": "vpt.proposal/1", "by": "fixture", "summary": [
            line("Speaker 2 says salut", "00:01-00:02"),
            json!({"text": "Speaker 3 says merci", "sources": ["00:00-00:01", "00:01-00:02"]}),
            line("Speaker 2 says merci", "00:02-00:03"),
            line("Speaker 2 approved 2 hires", "00:01-00:02"),
            line("Speaker approved", "00:01-00:02"),
        ], "actions": [action], "tags": [], "relations": []}),
    );
    let findings: Vec<_> = shown["summary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["finding"].clone())
        .collect();
    assert_eq!(
        findings,
        [
            json!(null),
            json!("unsupported"),
            json!("unsupported"),
            json!("unsupported"),
            json!("unsupported")
        ],
        "{shown}"
    );
    assert_eq!(shown["actions"][0]["finding"], json!(null), "{shown}");
    assert_eq!(
        shown["actions"][0]["due"]["finding"],
        json!("unsupported"),
        "a label's digit is not a spoken time: {shown}"
    );
    let note = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let verified = json(&support::run(
        sandbox.vpt().arg("verify-note").arg(&note.target).args([
            "--recording",
            row.id.as_str(),
            "--json",
        ]),
    ));
    let classes: Vec<_> = verified["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag["class"].clone())
        .collect();
    assert_eq!(classes, vec![json!("unsupported"); 4], "{verified}");
}
