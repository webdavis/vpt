use super::*;
use std::io::Write;

fn configure(sandbox: &Sandbox, enabled: bool, include_audio: bool) {
    writeln!(
        fs::OpenOptions::new().append(true).open(sandbox.config_path()).unwrap(),
        "\n[retention]\nenabled = {enabled}\ninclude_audio = {include_audio}\n[retention.hold]\naudio = '1d'"
    ).unwrap();
}

fn old_audio(sandbox: &Sandbox) -> RecordingRecord {
    add(sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(sandbox, &[]));
    let row = ledger(sandbox).recordings().unwrap().remove(0);
    fs::File::open(&row.audio_path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    row
}

#[test]
fn enabled_pipeline_retention_has_an_observational_plan_and_preserves_expiration() {
    let sandbox = configured();
    let row = old_audio(&sandbox);
    sandbox.install_fake_helper();
    configure(&sandbox, true, true);
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(
        plan["retention"]["moved"][0]["path"],
        row.audio_path.to_str().unwrap()
    );
    assert_eq!(plan["retention"]["counts"]["audio"], 1);
    assert_eq!(plan["retention"]["completed"], serde_json::json!([]));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let human = support::run_with_budget(
        sandbox.vpt().args(["run", "--dry-run"]),
        std::time::Duration::from_secs(8),
    );
    assert!(human.status.success(), "{}", stderr(&human));
    assert!(stdout(&human).contains("Would move 1 file(s)"));
    assert!(stdout(&human).contains(row.audio_path.to_str().unwrap()));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["retention"]["moved"], plan["retention"]["moved"]);
    assert_eq!(
        result["retention"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(!row.audio_path.exists());
    assert!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
    assert!(
        ledger(&sandbox)
            .note(&row.id, NoteStage::Transcript)
            .unwrap()
            .unwrap()
            .target
            .exists()
    );
    let again = json(&invoke(&sandbox, &[]));
    assert_eq!(again["retention"]["moved"], serde_json::json!([]));
    assert_eq!(again["transcribed"], serde_json::json!([]));
    assert!(!row.audio_path.exists());
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn pipeline_retention_requires_both_retention_and_audio_opt_in() {
    for (enabled, include_audio) in [(false, true), (true, false)] {
        let sandbox = configured();
        let row = old_audio(&sandbox);
        configure(&sandbox, enabled, include_audio);
        let result = json(&invoke(&sandbox, &[]));
        assert_eq!(result["retention"]["moved"], serde_json::json!([]));
        assert!(row.audio_path.exists());
        assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
    }
}

#[test]
fn retention_failure_preserves_new_pipeline_work_and_pending_move_for_recovery() {
    let sandbox = configured();
    let old = old_audio(&sandbox);
    configure(&sandbox, true, true);
    add(&sandbox, "two.m4a", 1_787_604_476);
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["rule"], "helper_absent");
    let new = ledger(&sandbox)
        .recordings()
        .unwrap()
        .into_iter()
        .find(|row| row.id != old.id)
        .unwrap();
    assert!(
        error["error"]["completed"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(new.id.as_str()))
    );
    assert_eq!(new.stages.transcribe, StageState::Succeeded);
    assert_eq!(new.stages.note, StageState::Succeeded);
    assert!(old.audio_path.exists());
    assert_eq!(ledger(&sandbox).pending_intents().unwrap().len(), 1);
    sandbox.install_fake_helper();
    let recovered = json(&invoke(&sandbox, &[]));
    assert_eq!(recovered["retention"]["moved"].as_array().unwrap().len(), 2);
    assert_eq!(recovered["retention"]["counts"]["audio"], 2);
    assert_eq!(
        recovered["retention"]["completed"],
        serde_json::json!([old.id.as_str(), new.id.as_str()])
    );
    assert_eq!(recovered["transcribed"], serde_json::json!([]));
    assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
}

#[test]
fn retention_output_failure_keeps_actual_completions_and_dry_run_claims_none() {
    let sandbox = configured();
    let row = old_audio(&sandbox);
    configure(&sandbox, true, true);
    sandbox.install_fake_helper();
    for dry_run in [true, false] {
        let before = fixture::snapshot(&sandbox);
        let mut command = sandbox.vpt();
        command.args(["run", "--json"]);
        if dry_run {
            command.arg("--dry-run");
        }
        let output =
            support::run_with_broken_stdout(&mut command, std::time::Duration::from_secs(8));
        assert_eq!(output.status.code(), Some(1));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["message"], "standard output write failed");
        if dry_run {
            assert_eq!(error["error"]["completed"], serde_json::json!([]));
            assert_eq!(fixture::snapshot(&sandbox), before);
        } else {
            assert_eq!(
                error["error"]["completed"],
                serde_json::json!([row.id.as_str()])
            );
            assert!(!row.audio_path.exists());
        }
    }
}
