use super::*;
use std::time::{Duration, SystemTime};
use vpt_application::ports::{ArtifactKind, NewIntent, RetentionStatus};
use vpt_domain::{run::RunKind, time::UtcInstant};

fn prepared() -> (Sandbox, vpt_application::ports::BriefArtifact) {
    let sandbox = fixture::configured("brief-retention");
    fixture::write_config(
        &sandbox,
        "[retention]\nenabled=true\n[retention.hold]\nbriefs='1d'\n",
    );
    sandbox.install_fake_helper();
    let document = json(&invoke(&sandbox, &manual(&[])));
    let id = vpt_domain::occasion::OccasionId::parse(document["occasion"]["id"].as_str().unwrap())
        .unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let brief = ledger.brief(&id).unwrap().unwrap();
    drop(ledger);
    std::fs::write(
        &brief.target,
        format!("{}\nPrivate operator prose\n", brief.text),
    )
    .unwrap();
    std::fs::File::open(&brief.target)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(100))
        .unwrap();
    (sandbox, brief)
}

#[test]
fn retention_reports_brief_occasion_and_explicit_brief_regenerates_the_expired_file() {
    let (sandbox, brief) = prepared();
    let before = fixture::snapshot(&sandbox);
    let planned = json(&support::run(sandbox.vpt().args([
        "retention",
        "run",
        "--dry-run",
        "--json",
    ])));
    assert_eq!(planned["counts"]["briefs"], 1);
    assert_eq!(planned["moved"][0]["id"], brief.snapshot.id.as_str());
    assert_eq!(fixture::snapshot(&sandbox), before);
    let moved = json(&support::run(sandbox.vpt().args([
        "retention",
        "run",
        "--json",
    ])));
    assert_eq!(moved["counts"]["briefs"], 1);
    assert_eq!(
        moved["completed"],
        serde_json::json!([brief.snapshot.id.as_str()])
    );
    assert!(!brief.target.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let expired = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == ArtifactKind::Brief)
        .unwrap();
    assert_eq!(expired.status, RetentionStatus::Expired);
    assert!(expired.trashed_at.is_some());
    assert_eq!(
        ledger.brief(&brief.snapshot.id).unwrap().unwrap().snapshot,
        brief.snapshot
    );
    drop(ledger);
    json(&invoke(
        &sandbox,
        &["brief", brief.snapshot.id.as_str(), "--json"],
    ));
    assert!(brief.target.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let restored = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == ArtifactKind::Brief)
        .unwrap();
    assert_eq!(restored.status, RetentionStatus::Published);
    assert_eq!(restored.trashed_at, None);
    let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!log.contains("Private operator prose"));
    let history: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let events: Vec<_> = history
        .iter()
        .filter(|row| row["fact"]["event"]["event"] == "retention")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["fact"]["event"]["counts"]["briefs"], 1);
    assert_eq!(events[0]["fact"]["event"]["counts"]["audio"], 0);
}

#[test]
fn mutating_startup_recovers_absent_brief_by_occasion_before_deliberate_regeneration() {
    let (sandbox, brief) = prepared();
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let at = UtcInstant { secs: 200 };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Retain, at) else {
        panic!("run")
    };
    let stores = vpt_adapters::FilesystemStores::open(&[]).unwrap();
    let digest = stores.digest_bytes(&std::fs::read(&brief.target).unwrap());
    assert!(matches!(
        ledger.prepare_retention(
            run.run,
            &NewIntent {
                kind: ArtifactKind::Brief,
                owner: brief.snapshot.id.clone().into(),
                path: brief.target.clone(),
                expected: digest,
                recorded_at: at
            }
        ),
        HistoryWrite::Committed(_)
    ));
    std::fs::rename(&brief.target, sandbox.path().join("already-trashed.md")).unwrap();
    drop(ledger);
    let result = json(&invoke(
        &sandbox,
        &["brief", brief.snapshot.id.as_str(), "--json"],
    ));
    assert_eq!(
        result["retention"]["completed"],
        serde_json::json!([brief.snapshot.id.as_str()])
    );
    assert_eq!(result["retention"]["moved"], serde_json::json!([]));
    assert!(brief.target.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn pipeline_retention_reports_later_store_moves_and_keeps_expired_briefs_absent() {
    let (sandbox, brief) = prepared();
    use std::io::Write;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(sandbox.config_path())
            .unwrap(),
        "\n[source]\nrecordings_dir = {:?}",
        sandbox.path().join("voice-memos/Recordings")
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["run", "--dry-run", "--json"]));
    assert_eq!(
        plan["retention"]["moved"][0]["id"],
        brief.snapshot.id.as_str()
    );
    assert_eq!(plan["retention"]["completed"], serde_json::json!([]));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let result = json(&invoke(&sandbox, &["run", "--json"]));
    assert_eq!(result["retention"]["counts"]["briefs"], 1);
    assert_eq!(
        result["retention"]["completed"],
        serde_json::json!([brief.snapshot.id.as_str()])
    );
    assert!(!brief.target.exists());
    let again = json(&invoke(&sandbox, &["run", "--json"]));
    assert_eq!(again["retention"]["moved"], serde_json::json!([]));
    assert!(!brief.target.exists());
}
