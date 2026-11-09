#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "redact/fixture.rs"]
mod recording;
mod support;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use support::{Sandbox, stderr};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;
fn invoke(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(5))
}
fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
fn configured(name: &str) -> (Sandbox, vpt_domain::identity::RecordingId) {
    let sandbox = fixture::configured(name);
    let id = recording::seed(&sandbox);
    json(&invoke(&sandbox, &["confirm", "--term", "Alice", "--json"]));
    fs::write(
        sandbox.path().join("home/.vpt/transcripts/Bob.md"),
        "# Bob\n",
    )
    .unwrap();
    (sandbox, id)
}
#[test]
fn redact_masks_live_terms_and_names_publishes_both_files_and_preserves_source() {
    let (sandbox, id) = configured("redact-write");
    let ledger = ledger(&sandbox);
    let note = ledger.note(&id, NoteStage::Transcript).unwrap().unwrap();
    let source = fs::read(&note.target).unwrap();
    let result = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    assert_eq!(result["command"], "redact");
    let written = Path::new(result["written"].as_str().unwrap());
    assert_eq!(
        written.file_name().unwrap(),
        format!("{}-transcript-{}.md", id.capture_date(), &id.hash12()[..8]).as_str()
    );
    let copy = fs::read_to_string(written).unwrap();
    for private in [
        "Alice",
        "Bob",
        "person@sample.test",
        "1234",
        "vpt:",
        "timecodes:",
    ] {
        assert!(!copy.contains(private), "{private}: {copy}");
    }
    assert!(copy.contains("&#91;term 1&#93;"));
    assert_eq!(result["masks"]["term"], 1);
    assert_eq!(result["masks"]["note"], 1);
    assert_eq!(result["masks"]["email"], 1);
    assert_eq!(result["masks"]["number"], 1);
    let report = Path::new(result["report"].as_str().unwrap());
    assert_eq!(
        fs::metadata(report).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    assert!(fs::read_to_string(report).unwrap().contains("Alice"));
    assert_eq!(fs::read(&note.target).unwrap(), source);
    let sequences = ledger.releases(&id).unwrap();
    assert_eq!(sequences.len(), 1);
    let release = ledger.release(sequences[0]).unwrap().unwrap();
    assert_eq!(release.destination, written);
    assert_eq!(release.report_path, report);
    assert!(!release.provenance.reviewed);
    assert!(ledger.pending_releases().unwrap().is_empty());
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    for private in ["Alice", "Bob", "person@sample.test", "Private mask"] {
        assert!(!events.contains(private));
    }
}
#[test]
fn redact_refuses_duplicate_under_an_unrelated_filename_without_another_release() {
    let (sandbox, id) = configured("redact-duplicate");
    let destination = sandbox.path().join("shared");
    let args = [
        "redact",
        id.as_str(),
        "--to",
        destination.to_str().unwrap(),
        "--json",
    ];
    let first = json(&invoke(&sandbox, &args));
    let renamed = destination.join("export.txt");
    fs::rename(first["written"].as_str().unwrap(), &renamed).unwrap();
    let result = invoke(&sandbox, &args);
    assert_eq!(result.status.code(), Some(3), "{}", stderr(&result));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["rule"], "duplicate");
    assert!(stderr(&result).contains("export.txt"));
    assert_eq!(ledger(&sandbox).releases(&id).unwrap().len(), 1);
}
#[test]
fn redact_refuses_private_destinations_and_header_residue_before_reservation() {
    let (sandbox, id) = configured("redact-refused");
    let private = sandbox.path().join("home/.vpt/transcripts/nested");
    for options in [
        vec!["--to", private.to_str().unwrap()],
        vec!["--title", "Alice"],
    ] {
        let mut args = vec!["redact", id.as_str(), "--json"];
        args.extend(options);
        let result = invoke(&sandbox, &args);
        assert_eq!(result.status.code(), Some(3), "{}", stderr(&result));
        assert!(result.stdout.is_empty());
        assert!(!stderr(&result).contains("Alice"));
    }
    assert!(!private.exists());
    assert!(ledger(&sandbox).releases(&id).unwrap().is_empty());
    assert!(ledger(&sandbox).pending_releases().unwrap().is_empty());
}
#[test]
fn redact_rejects_unsupported_options_and_private_values_stay_out_of_usage_errors() {
    let sandbox = fixture::configured("redact-usage");
    let id = "2026-08-24T144736-4f3ab19c02de";
    for extra in [
        vec!["--stage", "brief"],
        vec!["--stage", "audio"],
        vec!["--dry-run"],
        vec!["--title", "Private title", "--title", "Private other"],
        vec!["--to"],
    ] {
        let before = fixture::snapshot(&sandbox);
        let mut args = vec!["redact", id, "--json"];
        args.extend(extra);
        let result = invoke(&sandbox, &args);
        assert_eq!(result.status.code(), Some(2), "{}", stderr(&result));
        assert!(!stderr(&result).contains("Private"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[path = "redact/analysis.rs"]
mod analysis;
#[path = "redact/workflow.rs"]
mod workflow;
