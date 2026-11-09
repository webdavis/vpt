#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "redact/fixture.rs"]
mod recording;
mod support;
use std::{fs, path::Path};
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
    let id = recording::seed_flags(&sandbox, true);
    (sandbox, id)
}
fn allow(sandbox: &Sandbox) {
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(sandbox.config_path())
        .unwrap()
        .write_all(b"\n[handoff]\nallow_private = true\n")
        .unwrap();
}
fn handoff(sandbox: &Sandbox, id: &str, stage: &str) -> std::process::Output {
    invoke(sandbox, &["handoff", id, "--stage", stage, "--json"])
}
#[test]
fn released_handoff_is_observational_and_uses_frozen_uncertainty_and_exact_body() {
    let (sandbox, id) = configured("handoff-release");
    let release = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--title", "Public title", "--json"],
    ));
    let path = Path::new(release["written"].as_str().unwrap());
    let bytes = fs::read_to_string(path).unwrap();
    let body = bytes.split_once("\n---\n").unwrap().1;
    let saved = ledger(&sandbox)
        .release(release["sequence"].as_i64().unwrap())
        .unwrap()
        .unwrap();
    let before = fixture::snapshot(&sandbox);
    let first = json(&handoff(&sandbox, id.as_str(), "released"));
    assert_eq!(first["schema"], "vpt.handoff/1");
    assert_eq!(first["kind"], "released");
    assert_eq!(first["identity"]["recording"], id.as_str());
    assert_eq!(
        first["identity"]["content_sha256"],
        saved.version.content.hex()
    );
    assert_eq!(
        first["provenance"]["open_flags"],
        saved.provenance.open_flags
    );
    assert_eq!(first["provenance"]["reviewed"], false);
    assert_eq!(first["provenance"]["redacted"], true);
    let content = first["content"].as_str().unwrap();
    assert!(content.starts_with("# vpt released copy, "));
    assert!(content.ends_with(body));
    assert!(content.contains("Open flags: 1"));
    assert!(!content.contains("Public title"));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let flag = ledger(&sandbox)
        .review_snapshot(&id)
        .unwrap()
        .unwrap()
        .flags
        .into_iter()
        .find(|f| matches!(&f.content, vpt_domain::review::FlagContent::Lexical(_)))
        .unwrap();
    json(&invoke(
        &sandbox,
        &[
            "review",
            id.as_str(),
            "--resolve",
            &flag.identifier.hex(),
            "--confirm",
            "--json",
        ],
    ));
    let second = json(&handoff(&sandbox, id.as_str(), "released"));
    for key in ["identity", "title", "content", "provenance"] {
        assert_eq!(first[key], second[key]);
    }
}
#[test]
fn private_handoff_requires_opt_in_and_preserves_current_operator_prose() {
    let (sandbox, id) = configured("handoff-private");
    let before = fixture::snapshot(&sandbox);
    let denied = handoff(&sandbox, id.as_str(), "transcript");
    assert_eq!(denied.status.code(), Some(3));
    let error: serde_json::Value = serde_json::from_slice(&denied.stderr).unwrap();
    assert_eq!(error["error"]["rule"], "private_handoff");
    assert_eq!(fixture::snapshot(&sandbox), before);
    allow(&sandbox);
    let note = ledger(&sandbox)
        .note(&id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let edited = note.text.replace("tags:", "privateHeader: secret\ntags:")
        + "\nOperator prose must survive.\n";
    fs::write(&note.target, &edited).unwrap();
    let before = fixture::snapshot(&sandbox);
    let result = json(&handoff(&sandbox, id.as_str(), "transcript"));
    assert_eq!(result["kind"], "transcript");
    assert_eq!(result["provenance"]["redacted"], false);
    assert_eq!(result["provenance"]["reviewed"], false);
    assert!(
        result["content"]
            .as_str()
            .unwrap()
            .ends_with("\nOperator prose must survive.\n")
    );
    assert!(
        !result["content"]
            .as_str()
            .unwrap()
            .contains("privateHeader")
    );
    assert_ne!(result["identity"]["content_sha256"], note.digest.hex());
    assert_eq!(fixture::snapshot(&sandbox), before);
}
#[test]
fn released_selection_uses_sequence_skips_missing_files_and_refuses_changed_newest() {
    let (sandbox, id) = configured("handoff-selection");
    let first = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    let second = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--title", "Another copy", "--json"],
    ));
    let selected = json(&handoff(&sandbox, id.as_str(), "released"));
    assert!(
        selected["identity"]["note"].as_str().unwrap().ends_with(
            Path::new(second["written"].as_str().unwrap())
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
    fs::write(second["written"].as_str().unwrap(), "altered").unwrap();
    let before = fixture::snapshot(&sandbox);
    let changed = handoff(&sandbox, id.as_str(), "released");
    assert_eq!(changed.status.code(), Some(3));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&changed.stderr).unwrap()["error"]["rule"],
        "artifact_changed"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    fs::remove_file(second["written"].as_str().unwrap()).unwrap();
    let fallback = json(&handoff(&sandbox, id.as_str(), "released"));
    assert!(
        fallback["identity"]["note"].as_str().unwrap().ends_with(
            Path::new(first["written"].as_str().unwrap())
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
}
#[test]
fn missing_and_unsupported_handoffs_are_observational_input_errors() {
    let (sandbox, id) = configured("handoff-missing");
    let before = fixture::snapshot(&sandbox);
    for args in [
        vec!["handoff", id.as_str(), "--stage", "released", "--json"],
        vec!["handoff", id.as_str(), "--stage", "audio", "--json"],
        vec!["handoff", id.as_str(), "--json"],
        vec![
            "handoff",
            id.as_str(),
            "--stage",
            "released",
            "--dry-run",
            "--json",
        ],
    ] {
        let result = invoke(&sandbox, &args);
        assert_eq!(result.status.code(), Some(2), "{}", stderr(&result));
        assert!(result.stdout.is_empty());
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[path = "handoff/boundaries.rs"]
mod boundaries;

#[path = "handoff/analysis.rs"]
mod analysis;
#[path = "handoff/availability.rs"]
mod availability;
#[path = "handoff/independence.rs"]
mod independence;
