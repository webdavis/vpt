#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "analyze/actions.rs"]
mod actions;
#[path = "analyze/actions_env.rs"]
mod actions_env;
#[path = "analyze/batch.rs"]
mod batch;
#[path = "analyze/batch_input.rs"]
mod batch_input;
#[path = "analyze/captured_at.rs"]
mod captured_at;
#[path = "analyze/events.rs"]
mod events;
#[path = "analyze/filenames.rs"]
mod filenames;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "analyze/freshness.rs"]
mod freshness;
#[path = "analyze/links.rs"]
mod links;
#[path = "analyze/metadata.rs"]
mod metadata;
#[path = "note/setup.rs"]
mod note;
#[path = "analyze/note_boundaries.rs"]
mod note_boundaries;
#[path = "analyze/note_profiles.rs"]
mod note_profiles;
#[path = "analyze/output.rs"]
mod output;
#[path = "analyze/owner_words.rs"]
mod owner_words;
#[path = "analyze/profiles.rs"]
mod profiles;
#[path = "analyze/progress.rs"]
mod progress;
#[path = "analyze/protocol.rs"]
mod protocol;
#[path = "analyze/prune.rs"]
mod prune;
#[path = "analyze/prune_vault.rs"]
mod prune_vault;
#[path = "analyze/recovery.rs"]
mod recovery;
#[path = "analyze/redo.rs"]
mod redo;
#[path = "analyze/related.rs"]
mod related;
mod report_hook;
mod support;
#[path = "analyze/tag_defaults.rs"]
mod tag_defaults;
#[path = "analyze/tag_placeholders.rs"]
mod tag_placeholders;
#[path = "analyze/tag_renames.rs"]
mod tag_renames;
#[path = "analyze/workflow.rs"]
mod workflow;
use std::{fs, time::Duration};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;

fn invoke(sandbox: &Sandbox, id: &str, dry: bool) -> std::process::Output {
    let mut command = sandbox.vpt();
    command.args(["analyze", id, "--json"]);
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
            "[analysis]\ncommand = ['/bin/sh', {script:?}, {captured:?}, '{{transcript}}', '{{id}}', '{{language}}']\n{extra}"
        ),
    );
}

#[test]
fn analyze_publishes_first_analysis_and_saves_findings_with_exact_input() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    let output = invoke(&sandbox, record.id.as_str(), false);
    let result = note::success(&output);
    assert_eq!(result["command"], "analyze");
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
        ledger.by_id(&record.id).unwrap().unwrap().stages.analysis,
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
        assert!(note.text.contains("vpt-open-flags: 1"));
    }
    assert!(!stdout(&output).contains("private words"));
    assert!(!stdout(&output).contains("private agent"));
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(events.contains("analyze"));
    assert!(!events.contains("private words"));
    assert!(!events.contains("private agent"));
}

#[test]
fn analyze_dry_run_does_not_spawn_or_create_notes_history_or_directories() {
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
fn analyze_without_a_command_refuses_before_any_mutation() {
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
