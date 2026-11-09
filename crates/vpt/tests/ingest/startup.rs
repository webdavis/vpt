#[path = "startup/engine_outputs.rs"]
mod engine_outputs;
#[path = "startup/notes.rs"]
mod notes;
use super::*;

use vpt_adapters::{RootDir, SqliteLedger, digest_open};
use vpt_application::ports::{
    ArtifactKind, DirtyPublication, HistoryWrite, LedgerCommit, NewIntent, PublicationJournal,
    RecordingLedger, RetentionHistory, RetentionJournal, RunHistory, StageStates,
};
use vpt_domain::run::RunKind;
use vpt_domain::time::UtcInstant;

pub(super) fn pending_retention(sandbox: &Sandbox) -> vpt_application::ports::RecordingRecord {
    let mut row = fixture::record(sandbox, 1, StageStates::fresh());
    std::fs::create_dir_all(row.audio_path.parent().unwrap()).unwrap();
    std::fs::write(&row.audio_path, b"previously archived").unwrap();
    row.digest = digest_open(&mut std::fs::File::open(&row.audio_path).unwrap()).unwrap();
    fixture::seed(sandbox, vec![row.clone()]);
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let at = UtcInstant { secs: 100 };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Retain, at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.prepare_retention(
            run.run,
            &NewIntent {
                kind: ArtifactKind::Audio,
                owner: (row.id.clone()).into(),
                path: row.audio_path.clone(),
                expected: row.digest,
                recorded_at: at,
            }
        ),
        HistoryWrite::Committed(_)
    ));
    row
}

#[test]
fn ingest_reconciles_pending_retention_even_when_retention_is_disabled() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let row = pending_retention(&sandbox);
    add(&sandbox, "new.m4a");
    let doc = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(doc["ingested"].as_array().unwrap().len(), 1);
    assert!(!row.audio_path.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert!(
        ledger
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
}

#[test]
fn ingestion_dry_run_leaves_pending_retention_and_logs_untouched() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    pending_retention(&sandbox);
    add(&sandbox, "new.m4a");
    let before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn unavailable_source_still_recovers_retention_and_reports_completed_work() {
    assert_recovered_before_source_failure(false);
}

#[test]
fn source_failure_after_recovery_preserves_export_and_hook_warnings_once() {
    assert_recovered_before_source_failure(true);
}

fn assert_recovered_before_source_failure(fail_delivery: bool) {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let row = pending_retention(&sandbox);
    if fail_delivery {
        std::fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
        std::fs::write(
            sandbox.config_path(),
            format!("{config}\n[alert]\ncommand = ['/usr/bin/false']\n"),
        )
        .unwrap();
    }
    std::fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(!row.audio_path.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert!(
        ledger
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
    if fail_delivery {
        let warnings = error["error"]["diagnostics"].as_array().unwrap();
        assert_eq!(
            warnings
                .iter()
                .filter(|value| *value == "event log export is unavailable")
                .count(),
            1
        );
        assert_eq!(
            warnings
                .iter()
                .filter(|value| *value == "configured notification command failed")
                .count(),
            1
        );
    } else {
        let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        assert_eq!(
            log.lines()
                .filter(|line| line.contains("\"event\":\"ingest_failed\""))
                .count(),
            1
        );
        assert!(!log.contains("previously archived"));
    }
}

#[test]
fn changed_publication_target_blocks_ingestion_and_preserves_operator_bytes() {
    let sandbox = configured();
    fixture::dirty(&sandbox, fixture::record(&sandbox, 1, StageStates::fresh()));
    add(&sandbox, "new.m4a");
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let doc: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(doc["error"]["rule"], "publication_target_modified");
    assert_eq!(
        std::fs::read(sandbox.path().join("home/.vpt/transcripts/operator.md")).unwrap(),
        b"operator prose"
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(ledger.recordings().unwrap().len(), 1);
}

#[test]
fn already_published_bytes_complete_the_journal_before_ingestion() {
    let sandbox = configured();
    fixture::seed(&sandbox, vec![]);
    let target = sandbox.path().join("home/.vpt/transcripts/recovered.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, b"intended bytes").unwrap();
    let digest = digest_open(&mut std::fs::File::open(&target).unwrap()).unwrap();
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    ledger
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: target.clone(),
                intended: digest,
                expected_previous: None,
                recorded_at: UtcInstant { secs: 100 },
            }],
            ..Default::default()
        })
        .unwrap();
    drop(ledger);
    add(&sandbox, "new.m4a");
    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(
        ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(
                record.fact,
                vpt_domain::run::HistoryFact::EffectIntent {
                    kind: vpt_domain::run::EffectKind::Publication,
                    ..
                }
            ))
    );
    assert_eq!(std::fs::read(target).unwrap(), b"intended bytes");
}
