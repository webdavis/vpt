#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "synthesize/events.rs"]
mod events;
#[path = "synthesize/filenames.rs"]
mod filenames;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "synthesize/freshness.rs"]
mod freshness;
#[path = "synthesize/links.rs"]
mod links;
#[path = "synthesize/metadata.rs"]
mod metadata;
#[path = "note/setup.rs"]
mod note;
#[path = "synthesize/note_boundaries.rs"]
mod note_boundaries;
#[path = "synthesize/note_profiles.rs"]
mod note_profiles;
#[path = "synthesize/output.rs"]
mod output;
#[path = "synthesize/profiles.rs"]
mod profiles;
#[path = "synthesize/progress.rs"]
mod progress;
#[path = "synthesize/protocol.rs"]
mod protocol;
#[path = "synthesize/recovery.rs"]
mod recovery;
#[path = "synthesize/redo.rs"]
mod redo;
#[path = "synthesize/related.rs"]
mod related;
mod report_hook;
mod support;
#[path = "synthesize/tag_defaults.rs"]
mod tag_defaults;
#[path = "synthesize/tag_placeholders.rs"]
mod tag_placeholders;
#[path = "synthesize/tag_renames.rs"]
mod tag_renames;
#[path = "synthesize/workflow.rs"]
mod workflow;
use std::{fs, time::Duration};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;

fn invoke(sandbox: &Sandbox, id: &str, dry: bool) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.args(["synthesize", id, "--json"]);
    if dry {
        command.arg("--dry-run");
    }
    support::run_with_budget(&mut command, Duration::from_secs(5))
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
fn agent(sandbox: &Sandbox, extra: &str) {
    let script = sandbox.path().join("agent.sh");
    fs::write(&script, "#!/bin/sh\n/bin/cat > \"$1\"\nprintf '%s\\n' \"$2\" \"$3\" \"$4\" > \"$1.args\"\n/bin/cat \"$1.proposal\"\n").unwrap();
    let captured = sandbox.path().join("captured");
    fs::write(captured.with_extension("proposal"), r#"{"schema":"vpt.proposal/1","by":"private agent","summary":[{"text":"private words","sources":["00:00-00:01"]}],"actions":[{"text":"a private task","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    fixture::write_config(
        sandbox,
        &format!(
            "[synthesis]\ncommand = ['/bin/sh', {script:?}, {captured:?}, '{{transcript}}', '{{id}}', '{{language}}']\n{extra}"
        ),
    );
}

#[test]
fn synthesize_publishes_first_analysis_and_saves_findings_with_exact_input() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let output = invoke(&sandbox, record.id.as_str(), false);
    let result = note::success(&output);
    assert_eq!(result["command"], "synthesize");
    assert_eq!(result["verify"]["unsourced"], 1);
    let ledger = ledger(&sandbox);
    let transcript = ledger
        .note(&record.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let analysis = ledger
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let proposal = ledger.accepted_proposal(&record.id).unwrap().unwrap();
    let document = vpt_adapters::note_document::Document::parse(&transcript.text).unwrap();
    assert_eq!(
        fs::read_to_string(sandbox.path().join("captured")).unwrap(),
        document.content().unwrap()
    );
    assert_eq!(proposal.input_digest, transcript.content_digest);
    assert_eq!(
        fs::read_to_string(sandbox.path().join("captured.args")).unwrap(),
        format!("{}\n{}\nen-US\n", transcript.target.display(), record.id)
    );
    assert!(analysis.text.contains("- a private task [unsourced]"));
    assert!(analysis.text.contains("- Transcript:"));
    assert!(transcript.text.contains("- Analysis:"));
    assert_eq!(
        ledger.by_id(&record.id).unwrap().unwrap().stages.synthesis,
        StageState::Succeeded
    );
    assert_eq!(
        ledger
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags
            .len(),
        1
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    for note in [transcript, analysis] {
        assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
        assert!(note.text.contains("vptOpenFlags: 1"));
    }
    assert!(!stdout(&output).contains("private words"));
    assert!(!stdout(&output).contains("private agent"));
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(events.contains("synthesize"));
    assert!(!events.contains("private words"));
    assert!(!events.contains("private agent"));
}

#[test]
fn synthesize_dry_run_does_not_spawn_or_create_notes_history_or_directories() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let before = fixture::snapshot(&sandbox);
    let result = note::success(&invoke(&sandbox, record.id.as_str(), true));
    assert_eq!(result["dry_run"], true);
    assert_eq!(result["id"], record.id.as_str());
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(!sandbox.path().join("captured").exists());
}

#[test]
fn synthesize_without_a_command_refuses_before_any_mutation() {
    let (sandbox, record) = note::configured();
    let before = fixture::snapshot(&sandbox);
    for dry in [false, true] {
        let output = invoke(&sandbox, record.id.as_str(), dry);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(error["error"]["kind"], "config");
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
