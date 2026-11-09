use super::*;
#[path = "expiration/eligibility.rs"]
mod eligibility;
#[path = "expiration/report.rs"]
mod report;
#[path = "expiration/report_failures.rs"]
mod report_failures;

pub(super) fn prepared() -> (Sandbox, RecordingRecord, Vec<RetentionArtifact>) {
    let sandbox = configured();
    synthesis::enable(&sandbox);
    synthesis::append(&sandbox, "\n[share]\nautomatic=true\n");
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let ledger = ledger(&sandbox);
    let row = ledger.recordings().unwrap().remove(0);
    let artifacts = ledger.retention_artifacts().unwrap();
    assert_eq!(artifacts.len(), 8);
    for artifact in &artifacts {
        fs::File::open(&artifact.path)
            .unwrap()
            .set_modified(std::time::SystemTime::now())
            .unwrap();
    }
    drop(ledger);
    sandbox.install_fake_helper();
    synthesis::append(
        &sandbox,
        "\n[retention]\nenabled=true\ninclude_audio=true\n[retention.hold]\naudio='1d'\nengine_outputs='1d'\ntranscripts='1d'\nanalysis='1d'\ndrafts='1d'\nreleased='1d'\n",
    );
    (sandbox, row, artifacts)
}

pub(super) fn age(artifacts: impl Iterator<Item = impl AsRef<std::path::Path>>) {
    for path in artifacts {
        fs::File::open(path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH)
            .unwrap();
    }
}

#[test]
fn real_expiration_stays_terminal_and_explicit_note_restores_saved_render_inputs() {
    let (sandbox, row, artifacts) = prepared();
    let inputs = ledger(&sandbox).note_inputs(&row.id).unwrap().unwrap();
    age(artifacts.iter().map(|a| &a.path));
    let retained = json(&invoke(&sandbox, &[]));
    assert_eq!(retained["retention"]["moved"].as_array().unwrap().len(), 8);
    assert!(artifacts.iter().all(|a| !a.path.exists()));
    let expired = ledger(&sandbox).by_id(&row.id).unwrap().unwrap();
    assert_eq!(expired.stages.note, StageState::Expired);
    assert_eq!(expired.stages.synthesis, StageState::Expired);
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(plan["plan"], serde_json::json!([]));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let again = json(&invoke(&sandbox, &[]));
    for key in [
        "ingested",
        "transcribed",
        "notes",
        "synthesized",
        "redacted",
    ] {
        assert_eq!(again[key], serde_json::json!([]), "{key}");
    }
    assert!(artifacts.iter().all(|a| !a.path.exists()));
    json(&support::run_with_budget(
        sandbox
            .vpt()
            .args(["note", "write", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(8),
    ));
    let restored = ledger(&sandbox).note_inputs(&row.id).unwrap().unwrap();
    assert_eq!(restored.record.stages.note, StageState::Succeeded);
    assert_eq!(restored.record.stages.synthesis, StageState::Succeeded);
    for note in &inputs.notes {
        let current = restored
            .notes
            .iter()
            .find(|n| n.stage == note.stage)
            .unwrap();
        assert_eq!(current.content_digest, note.content_digest);
        assert_eq!(
            vpt_adapters::note_document::Document::parse(&current.text)
                .unwrap()
                .content()
                .unwrap(),
            vpt_adapters::note_document::Document::parse(&note.text)
                .unwrap()
                .content()
                .unwrap()
        );
        assert_eq!(fs::read_to_string(&current.target).unwrap(), current.text);
    }
    for artifact in ledger(&sandbox).retention_artifacts().unwrap() {
        if matches!(
            artifact.kind,
            ArtifactKind::Transcript | ArtifactKind::Analysis
        ) {
            assert_eq!(artifact.status, RetentionStatus::Published);
            assert_eq!(artifact.trashed_at, None);
        } else {
            assert!(!artifact.path.exists());
            assert_eq!(artifact.status, RetentionStatus::Expired);
        }
    }
    let automatic = json(&invoke(&sandbox, &[]));
    assert_eq!(automatic["transcribed"], serde_json::json!([]));
    assert_eq!(automatic["synthesized"], serde_json::json!([]));
    assert_eq!(automatic["redacted"], serde_json::json!([]));
    assert!(
        artifacts
            .iter()
            .filter(|a| !matches!(a.kind, ArtifactKind::Transcript | ArtifactKind::Analysis))
            .all(|a| !a.path.exists())
    );
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn automatic_sharing_keeps_expired_copies_absent_while_source_notes_are_live() {
    let (sandbox, row, artifacts) = prepared();
    let copies: Vec<_> = artifacts
        .iter()
        .filter(|a| {
            matches!(
                a.kind,
                ArtifactKind::Release | ArtifactKind::RedactionReport
            )
        })
        .collect();
    age(copies.iter().map(|a| &a.path));
    let retained = json(&invoke(&sandbox, &[]));
    assert_eq!(retained["retention"]["moved"].as_array().unwrap().len(), 4);
    let sequences = ledger(&sandbox).releases(&row.id).unwrap();
    let again = json(&invoke(&sandbox, &[]));
    assert_eq!(again["redacted"], serde_json::json!([]));
    assert!(copies.iter().all(|a| !a.path.exists()));
    assert_eq!(ledger(&sandbox).releases(&row.id).unwrap(), sequences);
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Succeeded
    );
}

fn pending(sandbox: &Sandbox, artifact: &RetentionArtifact) {
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let at = vpt_domain::time::UtcInstant { secs: 200 };
    let HistoryWrite::Committed(run) = ledger.begin_run(vpt_domain::run::RunKind::Retain, at)
    else {
        panic!("run")
    };
    assert!(matches!(
        ledger.prepare_retention(
            run.run,
            &NewIntent {
                kind: artifact.kind,
                owner: artifact.owner.clone(),
                path: artifact.path.clone(),
                expected: artifact.digest,
                recorded_at: at,
            }
        ),
        HistoryWrite::Committed(_)
    ));
}

#[test]
fn pending_absent_notes_are_expired_before_automatic_work_selects_stages() {
    let (sandbox, row, artifacts) = prepared();
    let notes: Vec<_> = artifacts
        .iter()
        .filter(|a| matches!(a.kind, ArtifactKind::Transcript | ArtifactKind::Analysis))
        .collect();
    for note in &notes {
        pending(&sandbox, note);
        fs::rename(
            &note.path,
            sandbox.path().join(format!("moved-{:?}", note.kind)),
        )
        .unwrap();
    }
    let recovered = json(&invoke(&sandbox, &[]));
    assert_eq!(
        recovered["retention"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(recovered["retention"]["moved"], serde_json::json!([]));
    assert_eq!(recovered["notes"], serde_json::json!([]));
    assert_eq!(recovered["synthesized"], serde_json::json!([]));
    assert!(notes.iter().all(|a| !a.path.exists()));
    assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
    assert!(!sandbox.fake_log().exists());
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn changed_pending_note_blocks_automatic_and_explicit_regeneration() {
    let (sandbox, row, artifacts) = prepared();
    let note = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Transcript)
        .unwrap();
    pending(&sandbox, note);
    let changed = format!(
        "{}\nOperator replacement\n",
        fs::read_to_string(&note.path).unwrap()
    );
    fs::write(&note.path, &changed).unwrap();
    for args in [
        vec!["run", "--json"],
        vec!["note", "write", row.id.as_str(), "--json"],
    ] {
        let output =
            support::run_with_budget(sandbox.vpt().args(args), std::time::Duration::from_secs(8));
        assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["rule"], "retention_target_modified");
        assert_eq!(fs::read_to_string(&note.path).unwrap(), changed);
        assert_eq!(ledger(&sandbox).pending_intents().unwrap().len(), 1);
    }
    assert!(!sandbox.fake_log().exists());
    assert_eq!(synthesis::calls(&sandbox, "calls"), 1);
    assert_eq!(synthesis::calls(&sandbox, "agent.calls"), 1);
}
