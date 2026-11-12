#[path = "upcoming/failures.rs"]
mod failures;
#[path = "upcoming/report.rs"]
mod report;
#[cfg(feature = "dev-tools")]
#[path = "upcoming/retention.rs"]
mod retention;
use super::*;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use vpt_adapters::SystemClock;

fn event(id: &str, secs: i64) -> Value {
    let mut row = support::dam::event(id, "Private event", secs, secs + 1800);
    row["labels"] = json!(["private-project"]);
    row
}
fn configured() -> Sandbox {
    let sandbox = fixture::configured("brief-upcoming");
    fixture::write_config(
        &sandbox,
        "[brief.trigger]\nenabled=true\n[context]\ntype='dam'\ncalendars=['calendar/work/']\nprojects=['work/']\n",
    );
    let script = sandbox.path().join("bin/dam");
    let calendar = sandbox.path().join("calendar.json");
    fs::write(
        &script,
        format!("#!/bin/sh\n{}", support::dam::script(&calendar)),
    )
    .unwrap();
    fs::set_permissions(script, fs::Permissions::from_mode(0o700)).unwrap();
    sandbox
}
fn reply(sandbox: &Sandbox, events: Vec<Value>) {
    fs::write(
        sandbox.path().join("calendar.json"),
        support::dam::agenda(events),
    )
    .unwrap();
}
fn reports(sandbox: &Sandbox) -> Vec<Value> {
    fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["fact"]["event"]["event"] == "brief_written")
        .collect()
}
#[test]
fn upcoming_publishes_each_identity_once_and_preserves_labels_and_rescheduled_path() {
    let sandbox = configured();
    let now = SystemClock.now().secs;
    reply(
        &sandbox,
        vec![
            event("one", now + 3600),
            event("two", now + 7200),
            event("past", now - 3600),
            event("later", now + 172800),
        ],
    );
    let result = json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]));
    assert_eq!(result["written"].as_array().unwrap().len(), 2);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let rows = ledger.occasions().unwrap();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(row.details.tags, ["private-project"]);
        assert_eq!(
            row.details.participants[0].email.as_deref(),
            Some("guest@example.org")
        );
        assert!(row.briefed_at.is_some());
        let brief = ledger.brief(&row.id).unwrap().unwrap();
        assert_eq!(brief.snapshot.occasion.tags, ["private-project"]);
        assert_eq!(fs::read_to_string(&brief.target).unwrap(), brief.text);
    }
    let events = reports(&sandbox);
    assert_eq!(events.len(), 2);
    for row in &rows {
        let event = events
            .iter()
            .map(|value| &value["fact"]["event"])
            .find(|event| event["occasion"] == row.id.as_str())
            .unwrap();
        assert!(event["recording"].is_null());
        assert_eq!(
            event["paths"]["brief"],
            row.brief_path.as_ref().unwrap().to_str().unwrap()
        );
        assert_eq!(event["counts"]["notes"], 0);
        assert!(!event.to_string().contains("private-project"));
    }
    let before: Vec<_> = rows
        .iter()
        .map(|r| ledger.brief(&r.id).unwrap().unwrap())
        .collect();
    reply(
        &sandbox,
        vec![event("one", now + 10800), event("two", now + 14400)],
    );
    assert_eq!(
        json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]))["written"],
        json!([])
    );
    assert_eq!(reports(&sandbox).len(), 2);
    let updated = ledger.occasions().unwrap();
    assert_eq!(updated.len(), 2);
    for prior in before {
        assert_eq!(ledger.brief(&prior.snapshot.id).unwrap().unwrap(), prior);
        assert!(
            updated
                .iter()
                .any(|row| row.id == prior.snapshot.id
                    && row.details.at != prior.snapshot.occasion.at)
        );
    }
}
#[test]
fn failed_discovery_records_failure_without_accepting_partial_results() {
    use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};
    let sandbox = configured();
    fs::write(
        sandbox.path().join("calendar.json"),
        "PRIVATE malformed answer",
    )
    .unwrap();
    let config = fs::read(sandbox.config_path()).unwrap();
    let output = invoke(&sandbox, &["brief", "--upcoming", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("PRIVATE"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(sandbox.config_path()).unwrap(), config);
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calendar.json")).unwrap(),
        "PRIVATE malformed answer"
    );
    assert!(!sandbox.path().join("home/.vpt/briefs").exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.occasions().unwrap().is_empty());
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(ledger.pending_exports().unwrap().is_empty());
    let history = ledger.recovery_view().unwrap();
    let run = history
        .records
        .iter()
        .find(|record| record.fact == HistoryFact::RunStarted(RunKind::Brief))
        .unwrap()
        .run;
    assert!(
        history.records.iter().any(|record| record.run == run
            && record.fact == HistoryFact::RunFinished(RunOutcome::Failed))
    );
    assert!(
        !fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .contains("PRIVATE")
    );
}
#[test]
fn disabled_context_cannot_be_used_as_successful_upcoming_discovery() {
    let sandbox = fixture::configured("brief-upcoming-none");
    fixture::write_config(&sandbox, "[brief.trigger]\nenabled=true\n");
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &["brief", "--upcoming", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
