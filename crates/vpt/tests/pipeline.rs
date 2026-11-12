#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared command fixtures")]
#[path = "pipeline/actions.rs"]
mod actions;
#[path = "pipeline/alerts.rs"]
mod alerts;
#[path = "pipeline/analysis.rs"]
mod analysis;
#[path = "pipeline/command_output.rs"]
mod command_output;
#[path = "pipeline/configuration.rs"]
mod configuration;
#[path = "pipeline/diagnostics.rs"]
mod diagnostics;
#[path = "pipeline/dry_runs.rs"]
mod dry_runs;
#[path = "pipeline/eligibility.rs"]
mod eligibility;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "pipeline/freshness.rs"]
mod freshness;
#[path = "pipeline/grounding_window.rs"]
mod grounding_window;
#[path = "pipeline/history_exports.rs"]
mod history_exports;
#[path = "pipeline/history_guards.rs"]
mod history_guards;
#[path = "pipeline/interrupts.rs"]
mod interrupts;
#[path = "pipeline/later_recovery.rs"]
mod later_recovery;
#[path = "pipeline/lexical_freshness.rs"]
mod lexical_freshness;
#[path = "pipeline/mentions.rs"]
mod mentions;
#[path = "pipeline/observations.rs"]
mod observations;
#[path = "pipeline/output.rs"]
mod output;
#[path = "pipeline/state_paths.rs"]
mod state_paths;
mod support;
use std::{fs, os::unix::fs::PermissionsExt};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;

fn configured() -> Sandbox {
    let sandbox = Sandbox::new("pipeline");
    let engine = sandbox.path().join("bin/engine");
    let document = sandbox.path().join("engine.json");
    fs::write(
        &engine,
        "#!/bin/sh\nprintf 'called\\n' >> \"$1\"\n/bin/cat -- \"$2\"\n",
    )
    .unwrap();
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&document, serde_json::to_vec(&serde_json::json!({
        "schema":"vpt.engine/1", "engine":{"name":"custom","tool":"fixture","version":"1","model":"test","family":"independent"},
        "language":"en-US", "local":true, "segments":[{"start":0.0,"end":1.0,"text":"Private words", "words":[{"text":"Private","start":0.0,"end":0.5},{"text":"words","start":0.5,"end":1.0}]}]
    })).unwrap()).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\nread_titles = false\nquiet_period_secs = 1\n[engines]\nmain = 'custom'\n[engines.custom]\nkind = 'command'\nfamily = 'independent'\nlocal = true\ncommand = [{engine:?}, {:?}, {document:?}]\n",
            sandbox.path().join("voice-memos/Recordings"),
            sandbox.path().join("calls")
        ),
    );
    sandbox
}

fn add(sandbox: &Sandbox, name: &str, seconds: i64) -> std::path::PathBuf {
    let path = sandbox.path().join("voice-memos/Recordings").join(name);
    fs::write(
        &path,
        vpt_domain::fixtures::m4a(seconds, 12, name.as_bytes()),
    )
    .unwrap();
    fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_787_604_456))
        .unwrap();
    path
}
fn invoke(sandbox: &Sandbox, extra: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["run", "--json"]).args(extra),
        std::time::Duration::from_secs(8),
    )
}
fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn dry_run_with_absent_state_is_observational_and_reports_ingestion_candidates() {
    let sandbox = configured();
    let source = add(&sandbox, "one.m4a", 1_787_604_456);
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(result["command"], "run");
    assert_eq!(result["dry_run"], true);
    assert_eq!(result["would_ingest"][0]["path"], source.to_str().unwrap());
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(!sandbox.path().join("calls").exists());
}

#[test]
fn run_ingests_transcribes_and_publishes_once_then_skips_successful_stages() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    add(&sandbox, "two.m4a", 1_787_604_476);
    let first = json(&invoke(&sandbox, &[]));
    assert_eq!(first["ingested"].as_array().unwrap().len(), 2);
    assert_eq!(first["transcribed"].as_array().unwrap().len(), 2);
    assert_eq!(first["notes"].as_array().unwrap().len(), 2);
    for row in ledger(&sandbox).recordings().unwrap() {
        assert_eq!(row.stages.transcribe, StageState::Succeeded);
        assert_eq!(row.stages.note, StageState::Succeeded);
        let note = ledger(&sandbox)
            .note(&row.id, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
    }
    let prior = ledger(&sandbox)
        .recordings()
        .unwrap()
        .into_iter()
        .map(|row| {
            ledger(&sandbox)
                .note(&row.id, NoteStage::Transcript)
                .unwrap()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let second = json(&invoke(&sandbox, &[]));
    for old in prior {
        let new = ledger(&sandbox)
            .note(&old.recording, NoteStage::Transcript)
            .unwrap()
            .unwrap();
        assert_eq!(old.text, new.text);
    }
    assert_eq!(second["ingested"], serde_json::json!([]));
    assert_eq!(second["transcribed"], serde_json::json!([]));
    assert_eq!(second["notes"], serde_json::json!([]));
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn pending_note_resumes_without_audio_or_an_engine_and_expired_notes_stay_expired() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    fs::remove_file(&row.audio_path).unwrap();
    fs::remove_file(sandbox.path().join("bin/engine")).unwrap();
    sql(&sandbox, "UPDATE recordings SET stage_note='pending'");
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(plan["plan"][0]["stage"], "note");
    assert_eq!(fixture::snapshot(&sandbox), before);
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["transcribed"], serde_json::json!([]));
    assert_eq!(result["notes"].as_array().unwrap().len(), 1);
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    fs::remove_file(&note.target).unwrap();
    sql(&sandbox, "UPDATE recordings SET stage_note='expired'");
    assert_eq!(json(&invoke(&sandbox, &[]))["notes"], serde_json::json!([]));
    assert!(!note.target.exists());
}

#[test]
fn engine_failure_preserves_ingested_progress_and_next_run_retries_from_transcription() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let engine = sandbox.path().join("bin/engine");
    let original = fs::read(&engine).unwrap();
    fs::write(&engine, "#!/bin/sh\necho Private-error >&2\nexit 17\n").unwrap();
    let failed = invoke(&sandbox, &[]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stdout.is_empty());
    assert!(!stderr(&failed).contains("Private-error"));
    let error: serde_json::Value = serde_json::from_str(&stderr(&failed)).unwrap();
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(row.stages.transcribe, StageState::Failed);
    fs::write(engine, original).unwrap();
    let recovered = json(&invoke(&sandbox, &[]));
    assert_eq!(recovered["ingested"], serde_json::json!([]));
    assert_eq!(
        recovered["transcribed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(recovered["notes"].as_array().unwrap().len(), 1);
}

#[path = "pipeline/boundaries.rs"]
mod boundaries;

#[path = "pipeline/recovery.rs"]
mod recovery;

#[test]
fn pipeline_publishes_and_saves_the_immediate_continuation_link() {
    let sandbox = configured();
    add(&sandbox, "earlier.m4a", 1_787_604_456);
    add(&sandbox, "later.m4a", 1_787_604_576);
    json(&invoke(&sandbox, &[]));
    let ledger = ledger(&sandbox);
    let mut rows = ledger.recordings().unwrap();
    rows.sort_by_key(|row| row.captured_at);
    let previous = ledger
        .note(&rows[0].id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let current = ledger
        .note(&rows[1].id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let links = vpt_domain::note::managed::Regions::parse(&current.text)
        .unwrap()
        .links();
    assert!(links.contains("- Continues:"));
    assert!(links.contains(previous.target.file_name().unwrap().to_str().unwrap()));
    let relations = ledger.relations(&rows[1].id).unwrap();
    assert_eq!(relations.len(), 1);
    assert_eq!(
        relations[0].target,
        vpt_domain::relations::Target::Recording(rows[0].id.clone())
    );
    let before = fixture::snapshot(&sandbox);
    json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn a_late_arriving_predecessor_updates_successful_note_links_without_retranscribing() {
    let sandbox = configured();
    add(&sandbox, "earlier.m4a", 1_787_604_456);
    add(&sandbox, "later.m4a", 1_787_604_576);
    json(&invoke(&sandbox, &[]));
    let ledger = ledger(&sandbox);
    let mut rows = ledger.recordings().unwrap();
    rows.sort_by_key(|row| row.captured_at);
    let current = ledger
        .note(&rows[1].id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    add(&sandbox, "middle.m4a", 1_787_604_516);
    let report = json(&invoke(&sandbox, &[]));
    let middle = ledger
        .recordings()
        .unwrap()
        .into_iter()
        .find(|row| row.captured_at.secs == 1_787_604_516)
        .unwrap();
    let relations = ledger.relations(&rows[1].id).unwrap();
    assert_eq!(
        relations[0].target,
        vpt_domain::relations::Target::Recording(middle.id.clone())
    );
    assert_eq!(
        ledger
            .note(&rows[1].id, NoteStage::Transcript)
            .unwrap()
            .unwrap()
            .content_digest,
        current.content_digest
    );
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    assert_eq!(report["transcribed"].as_array().unwrap().len(), 1);
}

#[path = "pipeline/expiration.rs"]
mod expiration;
#[path = "pipeline/retention.rs"]
mod retention;
#[path = "pipeline/sharing.rs"]
mod sharing;
