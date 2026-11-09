use super::*;

#[test]
fn transcribe_dry_run_is_observational_and_exposes_selected_engine_posture() {
    let (sandbox, row) = configured();
    let before = fixture::snapshot(&sandbox);
    let result = json(&run(
        &sandbox,
        row.id.as_str(),
        &["--dry-run", "--language", "ja-JP"],
    ));
    assert_eq!(result["command"], "transcribe");
    assert_eq!(result["language"], "ja-JP");
    assert_eq!(result["engines"][0]["name"], "custom");
    assert_eq!(result["engines"][0]["local"], true);
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(!sandbox.path().join("calls").exists());
}

#[test]
fn command_engine_cannot_delete_archive_and_output_is_durable_and_private() {
    let (sandbox, row) = configured();
    let audio = std::fs::read(&row.audio_path).unwrap();
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    assert_eq!(result["id"], row.id.as_str());
    assert_eq!(result["engine"], "custom");
    assert_eq!(std::fs::read(&row.audio_path).unwrap(), audio);
    let ledger = ledger(&sandbox);
    let accepted = ledger.accepted_transcript(&row.id).unwrap().unwrap();
    assert_eq!(accepted.requested_language, "fr-CA");
    assert_eq!(
        accepted.uncertainty.diagnostics,
        [vpt_domain::uncertainty::Diagnostic::SingleEngine]
    );
    assert!(accepted.uncertainty.lexical.is_empty());
    assert_eq!(
        vpt_application::ports::FlagLedger::transcript_flags(&ledger, &row.id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        accepted.transcript.segments[0].words[0].text,
        "private transcript canary"
    );
    assert_eq!(
        ledger.by_id(&row.id).unwrap().unwrap().stages.transcribe,
        StageState::Succeeded
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    let root = sandbox.path().join("home/.vpt/engine-outputs");
    let target = root.join(format!("{}.custom.json", row.id));
    assert_eq!(
        std::fs::read(&target).unwrap(),
        std::fs::read(sandbox.path().join("engine.json")).unwrap()
    );
    assert_eq!(
        std::fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
    let calls = std::fs::read_to_string(sandbox.path().join("calls")).unwrap();
    assert!(!calls.contains(row.audio_path.to_str().unwrap()));
    assert_eq!(calls.lines().nth(1), Some("fr-CA"));
    assert!(
        !std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .contains("private transcript canary")
    );
}

#[test]
fn failed_retranscription_retains_prior_accepted_text_and_emits_failure() {
    let (sandbox, row) = configured();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let accepted = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
    std::fs::write(
        sandbox.path().join("bin/engine"),
        "#!/bin/sh\necho private-error-canary >&2\nexit 17\n",
    )
    .unwrap();
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("private-error-canary"));
    assert_eq!(
        ledger(&sandbox).accepted_transcript(&row.id).unwrap(),
        accepted
    );
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .transcribe,
        StageState::Failed
    );
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(events.contains("transcribe_failed"));
    assert!(!events.contains("private-error-canary"));
}

#[test]
fn same_family_is_refused_before_spawning_or_writing() {
    let (sandbox, row) = configured();
    let config = std::fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("main = 'custom'", "main = 'custom'\nchecker = 'comparison'");
    std::fs::write(sandbox.config_path(), format!("{config}\n[engines.comparison]\nkind = 'command'\nfamily = 'independent'\nlocal = true\ncommand = ['/usr/bin/false']\n")).unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(3));
    let doc: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(doc["error"]["rule"], "same_family");
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn modified_audio_is_refused_before_engine_invocation() {
    let (sandbox, row) = configured();
    let mut bytes = std::fs::read(&row.audio_path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(&row.audio_path, &bytes).unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn checker_fallback_uses_a_fresh_working_copy_after_main_deletes_its_input() {
    let (sandbox, row) = configured();
    std::fs::write(
        sandbox.path().join("bin/engine"),
        "#!/bin/sh\n/bin/rm -- \"$1\"\nexit 17\n",
    )
    .unwrap();
    let checker = sandbox.path().join("bin/checker");
    std::fs::write(
        &checker,
        "#!/bin/sh\ntest -f \"$1\" || exit 29\n/bin/cat -- \"$2\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&checker, std::fs::Permissions::from_mode(0o700)).unwrap();
    let document = sandbox.path().join("checker.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(sandbox.path().join("engine.json")).unwrap())
            .unwrap();
    value["engine"]["name"] = "comparison".into();
    value["engine"]["family"] = "second".into();
    std::fs::write(&document, serde_json::to_vec(&value).unwrap()).unwrap();
    let config = std::fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace(
            "main = 'custom'",
            "main = 'custom'\nchecker = 'comparison'\nchecker_runs = 'on_failure'",
        );
    std::fs::write(sandbox.config_path(), format!("{config}\n[engines.comparison]\nkind = 'command'\nfamily = 'second'\nlocal = true\ncommand = [{checker:?}, '{{audio}}', {document:?}]\n")).unwrap();
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    assert_eq!(result["engine"], "comparison");
    assert!(
        result["engine_diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("custom: Exited(17)"))
    );
    assert_eq!(result["diagnostics"], serde_json::json!(["single-engine"]));
    assert!(
        ledger(&sandbox)
            .accepted_transcript(&row.id)
            .unwrap()
            .unwrap()
            .from_checker
    );
    assert!(row.audio_path.exists());
}

#[test]
fn a_modified_output_preserves_previous_accepted_state_and_operator_bytes() {
    let (sandbox, row) = configured();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let accepted = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
    let target = sandbox
        .path()
        .join(format!("home/.vpt/engine-outputs/{}.custom.json", row.id));
    std::fs::write(&target, b"operator bytes").unwrap();
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(std::fs::read(target).unwrap(), b"operator bytes");
    assert_eq!(
        ledger(&sandbox).accepted_transcript(&row.id).unwrap(),
        accepted
    );
}

#[test]
fn both_engine_provenances_survive_raw_output_removal_and_note_publication() {
    let (sandbox, row) = configured();
    let document = sandbox.path().join("comparison.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(sandbox.path().join("engine.json")).unwrap())
            .unwrap();
    value["engine"]["name"] = "comparison".into();
    value["engine"]["family"] = "second".into();
    value["engine"]["model"] = "checker-model".into();
    std::fs::write(&document, serde_json::to_vec(&value).unwrap()).unwrap();
    let config = std::fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace(
            "main = 'custom'",
            "main = 'custom'\nchecker = 'comparison'\nchecker_runs = 'always'",
        );
    std::fs::write(sandbox.config_path(), format!("{config}\n[engines.comparison]\nkind = 'command'\nfamily = 'second'\nlocal = true\ncommand = ['/bin/cat', {document:?}]\n")).unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.transcript.engine.name, "custom");
    assert_eq!(
        accepted.checker_engine.as_ref().unwrap().model,
        "checker-model"
    );
    assert!(!accepted.from_checker);
    std::fs::remove_file(&row.audio_path).unwrap();
    std::fs::remove_dir_all(sandbox.path().join("home/.vpt/engine-outputs")).unwrap();
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["note", "write", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    );
    let result = json(&output);
    let note = std::fs::read_to_string(result["paths"][0].as_str().unwrap()).unwrap();
    assert!(note.contains("custom:test"));
    assert!(note.contains("comparison:checker-model"));
    assert!(note.contains("private transcript canary"));
}
