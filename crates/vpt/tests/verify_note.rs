#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "note/setup.rs"]
mod note;
mod support;
use std::{fs, path::Path};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;
#[path = "verify_note/analysis.rs"]
mod analysis;
#[path = "verify_note/freshness.rs"]
mod freshness;
#[path = "verify_note/output.rs"]
mod output;
#[path = "verify_note/ownership.rs"]
mod ownership;
#[path = "verify_note/workflow.rs"]
mod workflow;

fn invoke(sandbox: &Sandbox, path: &Path, id: &str) -> std::process::Output {
    support::run_with_budget(
        sandbox
            .vpt()
            .arg("verify-note")
            .arg(path)
            .args(["--recording", id, "--json"]),
        std::time::Duration::from_secs(5),
    )
}
fn success(output: &std::process::Output) -> serde_json::Value {
    note::success(output)
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[test]
fn external_notes_report_findings_without_any_file_or_ledger_changes() {
    let (sandbox, record) = note::configured();
    let path = sandbox.path().join("external.md");
    fs::write(
        &path,
        "- words [00:00-00:01]\n- Bob [00:00-00:01]\n- unsourced\n- words [00:30-00:31]\n",
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &path, record.id.as_str());
    let result = success(&output);
    assert_eq!(result["owned"], false);
    assert_eq!(result["flags"].as_array().unwrap().len(), 3);
    for (index, class, line) in [
        (0, "unsupported", 2),
        (1, "unsourced", 3),
        (2, "bad-reference", 4),
    ] {
        assert_eq!(result["flags"][index]["class"], class);
        assert_eq!(result["flags"][index]["line"], line);
    }
    assert!(!stdout(&output).contains("Bob"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn owned_note_verification_saves_current_claims_annotations_and_counts_atomically() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let old = note::saved(&sandbox, &record);
    let document = vpt_adapters::note_document::Document::parse(&old.text).unwrap();
    let edited = document
        .replace_content("- Bob [00:00-00:01]\n")
        .unwrap()
        .replace("vptOpenFlags: 0\n", "")
        + "\nOperator ending\n";
    fs::write(&old.target, &edited).unwrap();
    let accepted = ledger(&sandbox).accepted_transcript(&record.id).unwrap();
    let result = success(&invoke(&sandbox, &old.target, record.id.as_str()));
    assert_eq!(result["owned"], true);
    assert_eq!(result["flags"][0]["class"], "unsupported");
    let current = note::saved(&sandbox, &record);
    assert_eq!(
        result["flags"][0]["line"],
        current
            .text
            .lines()
            .position(|line| line.starts_with("- Bob"))
            .unwrap()
            + 1
    );
    assert!(
        current
            .text
            .contains("- Bob [00:00-00:01] [unsupported: Bob]")
    );
    assert!(current.text.contains("vptOpenFlags: 1\n"));
    assert!(current.text.ends_with("\nOperator ending\n"));
    assert_eq!(fs::read_to_string(&old.target).unwrap(), current.text);
    assert_eq!(
        ledger(&sandbox).accepted_transcript(&record.id).unwrap(),
        accepted
    );
    assert_eq!(
        ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags
            .len(),
        1
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    success(&invoke(&sandbox, &old.target, record.id.as_str()));
    assert_eq!(fs::read_to_string(&old.target).unwrap(), current.text);
}
