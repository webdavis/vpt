#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "occasions/recovery.rs"]
mod recovery;
mod support;
use support::{Sandbox, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    occasion::{OccasionDetails, OccasionKey, time::OccasionTime},
    run::{HistoryFact, RunKind},
    time::UtcInstant,
};

fn invoke(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(5))
}
fn seed(sandbox: &Sandbox) -> OccasionRecord {
    seed_named(sandbox, "Private provider id")
}
fn seed_named(sandbox: &Sandbox, key: &str) -> OccasionRecord {
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir_all(&state).unwrap();
    let ledger = SqliteLedger::open(&RootDir::open(&state).unwrap()).unwrap();
    let details = OccasionDetails {
        at: OccasionTime::parse("2026-09-30T23:00:00.123456789012-06:00").unwrap(),
        title: "Private\noccasion".into(),
        duration_secs: Some(900),
        participants: vec![],
        tags: vec!["planning".into()],
    };
    let at = UtcInstant { secs: 100 };
    let change =
        vpt_adapters::prepare_occasion(&ledger, OccasionKey::dam(key).unwrap(), details, at)
            .unwrap();
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Brief, at) else {
        panic!("run");
    };
    let record = change.occasion.clone();
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                occasion: Some(change),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::OccasionSaved {
                    occasion: record.id.clone()
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    record
}

#[test]
fn occasions_lists_unbriefed_saved_data_without_creating_or_changing_state() {
    let sandbox = fixture::configured("occasions");
    let before = fixture::snapshot(&sandbox);
    let empty = invoke(&sandbox, &["occasions", "--json"]);
    assert_eq!(empty.status.code(), Some(0), "{}", stderr(&empty));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap()["occasions"],
        serde_json::json!([])
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let record = seed(&sandbox);
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &["occasions", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["command"], "occasions");
    let rows = json["occasions"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], record.id.as_str());
    assert_eq!(rows[0]["at"], record.details.at.as_str());
    assert_eq!(rows[0]["title"], record.details.title);
    assert_eq!(rows[0]["source"], "dam");
    assert!(rows[0]["brief_path"].is_null());
    assert!(rows[0]["briefed_at"].is_null());
    assert!(!stdout(&output).contains("provider id"));
    let human = invoke(&sandbox, &["occasions"]);
    assert_eq!(human.status.code(), Some(0), "{}", stderr(&human));
    assert!(stdout(&human).contains("Private\\noccasion"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn occasion_path_reads_saved_pin_even_when_file_is_absent() {
    let sandbox = fixture::configured("occasion-path");
    let record = seed(&sandbox);
    let pin = sandbox.path().join("briefs/original.md");
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection
        .execute(
            "UPDATE occasions SET brief_path = ?1 WHERE id = ?2",
            rusqlite::params![pin.to_str().unwrap(), record.id.as_str()],
        )
        .unwrap();
    fixture::preserve_wal(&connection);
    drop(connection);
    let before = fixture::snapshot(&sandbox);
    let output = invoke(
        &sandbox,
        &[
            "path",
            "--occasion",
            record.id.as_str(),
            "--stage",
            "brief",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["path"], pin.to_str().unwrap());
    assert!(!pin.exists());
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn occasion_path_usage_and_missing_pin_are_observational() {
    let sandbox = fixture::configured("occasion-path-errors");
    let record = seed(&sandbox);
    let before = fixture::snapshot(&sandbox);
    for args in [
        vec!["path", "--occasion", record.id.as_str(), "--stage", "brief"],
        vec![
            "path",
            "--occasion",
            record.id.as_str(),
            "--stage",
            "analysis",
        ],
        vec!["path", "--occasion", "invalid", "--stage", "brief"],
        vec![
            "path",
            record.id.as_str(),
            "--occasion",
            record.id.as_str(),
            "--stage",
            "brief",
        ],
        vec!["path", "--occasion", "--stage", "brief"],
        vec!["occasions", "--dry-run"],
    ] {
        let output = invoke(&sandbox, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty());
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
