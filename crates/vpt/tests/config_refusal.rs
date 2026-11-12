#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared command fixtures")]
#[path = "config_refusal/failures.rs"]
mod failures;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "config_refusal/fixture.rs"]
mod refusal;
mod support;

use std::{fs, time::Duration};
use support::stderr;
use vpt_application::ports::{RecordingLedger, RunHistory};
use vpt_domain::{
    history::{EventDetail, EventKind, SafeFailureClass},
    run::{HistoryFact, RunKind, RunOutcome},
};

fn check(verb: &str, kind: RunKind) {
    let case = refusal::Fixture::new();
    if verb == "upcoming" {
        case.enable_upcoming();
    }
    let before = case.ledger().recovery_view().unwrap().records.len();
    let args = case.args(verb);
    let output = support::run_with_budget(
        case.sandbox.vpt().args(args).arg("--json"),
        if verb == "run" {
            Duration::from_secs(8)
        } else {
            Duration::from_secs(5)
        },
    );
    assert_eq!(output.status.code(), Some(2), "{verb}: {}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "config");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("stores.drafts:"),
        "{error}"
    );
    assert_eq!(error["error"]["completed"], serde_json::json!([]));
    assert_eq!(
        fs::read_to_string(case.sandbox.path().join("engine-calls")).unwrap(),
        "called\n"
    );
    assert!(!case.parent.join("drafts").exists());
    let ledger = case.ledger();
    assert_eq!(ledger.recordings().unwrap().len(), 1);
    let history = ledger.recovery_view().unwrap();
    let new = &history.records[before..];
    let events: Vec<_> = new
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::Event(event) if event.kind == EventKind::ConfigRefused => {
                Some((record.run, event))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        events.len(),
        1,
        "{verb}: missing configuration refusal history"
    );
    assert_eq!(
        events[0].1.detail,
        EventDetail::Failure(SafeFailureClass::ConfigurationRefused)
    );
    assert!(events[0].1.paths.is_empty());
    let run = events[0].0;
    assert!(
        new.iter()
            .any(|record| record.run == run && record.fact == HistoryFact::RunStarted(kind))
    );
    assert_eq!(
        new.iter()
            .filter(|record| record.run == run
                && record.fact == HistoryFact::RunFinished(RunOutcome::Failed))
            .count(),
        1
    );
    assert!(ledger.pending_exports().unwrap().is_empty());
    let alerts = fs::read_to_string(case.sandbox.path().join("alerts.jsonl")).unwrap();
    let alerts: Vec<serde_json::Value> = alerts
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["schema"], "vpt.event/1");
    assert_eq!(alerts[0]["channel"], "alert");
    assert_eq!(alerts[0]["event"], "config_refused");
    assert!(!case.sandbox.path().join("report-called").exists());
    let log = fs::read_to_string(case.sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!log.contains("Private words"));
}

macro_rules! refusal_test {
    ($name:ident, $verb:literal, $kind:ident) => {
        #[test]
        fn $name() {
            check($verb, RunKind::$kind);
        }
    };
}
refusal_test!(pipeline, "run", Pipeline);
refusal_test!(transcribe, "transcribe", Transcribe);
refusal_test!(note, "note", Note);
refusal_test!(analyze, "analyze", Analyze);
refusal_test!(confirm, "confirm", Confirm);
refusal_test!(review, "review", Review);
refusal_test!(manual_brief, "brief", Brief);
refusal_test!(upcoming_brief, "upcoming", Brief);
refusal_test!(redact, "redact", Redact);
