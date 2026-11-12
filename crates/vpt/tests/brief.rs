#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "brief/arguments.rs"]
mod arguments;
#[path = "brief/context.rs"]
mod context;
#[path = "brief/evidence.rs"]
mod evidence;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "brief/interrupts.rs"]
mod interrupts;
#[path = "brief/publication.rs"]
mod publication;
#[path = "brief/rendering.rs"]
mod rendering;
#[path = "brief/rendering_fixture.rs"]
mod rendering_fixture;
#[path = "brief/rendering_recovery.rs"]
mod rendering_recovery;
#[path = "brief/rendering_rewrite.rs"]
mod rendering_rewrite;
#[cfg(feature = "dev-tools")]
#[path = "brief/retention.rs"]
mod retention;
mod support;
#[path = "brief/upcoming.rs"]
mod upcoming;
use support::{Sandbox, stderr};
use vpt_adapters::SqliteLedger;
use vpt_application::ports::*;
fn invoke(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(5))
}
fn manual<'a>(extra: &'a [&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "brief",
        "--title",
        "Private planning",
        "--at",
        "2026-10-01T10:00:00-06:00",
        "--participant",
        "Private person",
        "--tag",
        "planning",
        "--tag",
        "work",
        "--duration-minutes",
        "30",
        "--json",
    ];
    args.extend_from_slice(extra);
    args
}
#[track_caller]
fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn manual_preview_and_explain_are_observational_with_an_absent_ledger() {
    let sandbox = fixture::configured("brief-preview");
    let before = fixture::snapshot(&sandbox);
    for mode in ["--dry-run", "--explain"] {
        let result = json(&invoke(&sandbox, &manual(&[mode])));
        assert_eq!(result["schema"], "vpt.brief/1");
        assert_eq!(result["occasion"]["title"], "Private planning");
        assert_eq!(result["occasion"]["duration_secs"], 1800);
        assert_eq!(result["occasion"]["participants"][0]["term"], "unconfirmed");
        assert_eq!(result["context"]["calendar"]["reason"], "disabled");
        assert_eq!(result["items"], serde_json::json!([]));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
#[test]
fn manual_brief_publishes_one_pinned_owned_file_and_rewrites_by_identity() {
    let sandbox = fixture::configured("brief-write");
    let result = json(&invoke(&sandbox, &manual(&[])));
    let id = vpt_domain::occasion::OccasionId::parse(result["occasion"]["id"].as_str().unwrap())
        .unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let before = ledger.brief(&id).unwrap().unwrap();
    assert_eq!(
        std::fs::read_to_string(&before.target).unwrap(),
        before.text
    );
    assert!(ledger.occasion(&id).unwrap().unwrap().briefed_at.is_some());
    let edited =
        before.text.replace("\ntags:", "\noperator: keep\ntags:") + "\nPrivate operator prose\n";
    std::fs::write(&before.target, &edited).unwrap();
    json(&invoke(&sandbox, &["brief", id.as_str(), "--json"]));
    let after = ledger.brief(&id).unwrap().unwrap();
    assert_eq!(after.target, before.target);
    assert_eq!(after.first_written, before.first_written);
    assert!(after.text.contains("operator: keep"));
    assert!(after.text.ends_with("\nPrivate operator prose\n"));
    assert_eq!(ledger.occasions().unwrap().len(), 1);
    assert!(ledger.pending_publications().unwrap().is_empty());
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("Private"));
    assert!(!events.contains("brief_written"));
}
