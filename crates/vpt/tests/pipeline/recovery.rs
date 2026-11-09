use super::*;
use vpt_adapters::RootDir;
use vpt_domain::{run::RunKind, time::UtcInstant};

#[test]
fn recovered_retention_hook_warning_survives_a_later_source_refusal() {
    let sandbox = configured();
    sandbox.install_fake_helper();
    let mut row = fixture::record(&sandbox, 1, StageStates::fresh());
    fs::create_dir_all(row.audio_path.parent().unwrap()).unwrap();
    fs::write(&row.audio_path, b"previously archived").unwrap();
    row.digest = vpt_adapters::digest_open(&mut fs::File::open(&row.audio_path).unwrap()).unwrap();
    fixture::seed(&sandbox, vec![row.clone()]);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
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
    drop(ledger);
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        format!("{config}\n[report]\ncommand = ['/usr/bin/false']\n"),
    )
    .unwrap();
    fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    let output = invoke(&sandbox, &[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(!row.audio_path.exists());
    assert!(
        error["error"]["diagnostics"]
            .as_array()
            .is_some_and(|items| items
                .iter()
                .any(|item| item == "configured notification command failed")),
        "{error}"
    );
}
