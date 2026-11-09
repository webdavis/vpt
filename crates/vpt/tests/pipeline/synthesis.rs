use super::*;
use std::io::Write;

pub(super) fn enable(sandbox: &Sandbox) {
    let script = sandbox.path().join("agent.sh");
    fs::write(&script, "#!/bin/sh\n/bin/cat > \"$1.input\"\nprintf 'called\\n' >> \"$1.calls\"\n/bin/cat \"$1.proposal\"\n").unwrap();
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Private words","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#).unwrap();
    append(
        sandbox,
        &format!(
            "\n[synthesis]\ncommand = ['/bin/sh', {script:?}, {:?}]\n",
            sandbox.path().join("agent")
        ),
    );
}

pub(super) fn append(sandbox: &Sandbox, text: &str) {
    fs::OpenOptions::new()
        .append(true)
        .open(sandbox.config_path())
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
}

pub(super) fn calls(sandbox: &Sandbox, name: &str) -> usize {
    fs::read_to_string(sandbox.path().join(name))
        .unwrap()
        .lines()
        .count()
}

#[test]
fn pipeline_names_an_unsupported_proposal_version_without_publishing_analysis() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let record = ledger(&sandbox).recordings().unwrap().remove(0);
    enable(&sandbox);
    fs::write(
        sandbox.path().join("agent.proposal"),
        r#"{"schema":"vpt.proposal/2","private":"private provider content"}"#,
    )
    .unwrap();
    for structured in [true, false] {
        let mut command = sandbox.vpt();
        command.arg("run");
        if structured {
            command.arg("--json");
        }
        let output = support::run_with_budget(&mut command, std::time::Duration::from_secs(8));
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(stderr(&output).contains("unsupported synthesis output schema vpt.proposal/2"));
        assert!(!stderr(&output).contains("private provider content"));
        assert!(
            ledger(&sandbox)
                .accepted_proposal(&record.id)
                .unwrap()
                .is_none()
        );
        assert!(
            ledger(&sandbox)
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .is_none()
        );
        assert_eq!(calls(&sandbox, "calls"), 1);
    }
}

#[test]
fn enabled_run_synthesizes_after_transcription_and_skips_unchanged_inputs() {
    let sandbox = configured();
    enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    let result = json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert_eq!(result["synthesized"], serde_json::json!([row.id.as_str()]));
    assert_eq!(row.stages.synthesis, StageState::Succeeded);
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert_eq!(fs::read_to_string(&analysis.target).unwrap(), analysis.text);
    let transcript = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let content = vpt_adapters::note_document::Document::parse(&transcript.text).unwrap();
    assert_eq!(
        fs::read_to_string(sandbox.path().join("agent.input")).unwrap(),
        content.content().unwrap()
    );
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"],
        serde_json::json!([])
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert_eq!(
        json(&invoke(&sandbox, &[]))["synthesized"],
        serde_json::json!([])
    );
    assert_eq!(calls(&sandbox, "agent.calls"), 1);
    assert_eq!(calls(&sandbox, "calls"), 1);
}

#[test]
fn synthesis_failure_preserves_transcription_and_retries_without_the_engine() {
    let sandbox = configured();
    enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    let proposal = sandbox.path().join("agent.proposal");
    let valid = fs::read(&proposal).unwrap();
    fs::write(&proposal, "private malformed proposal").unwrap();
    let result = invoke(&sandbox, &[]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(!stderr(&result).contains("private malformed"));
    let error: serde_json::Value = serde_json::from_str(&stderr(&result)).unwrap();
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(row.stages.note, StageState::Succeeded);
    assert_eq!(row.stages.synthesis, StageState::Failed);
    fs::remove_file(sandbox.path().join("bin/engine")).unwrap();
    fs::write(proposal, valid).unwrap();
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"][0]["stage"],
        "synthesize"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let retried = json(&invoke(&sandbox, &[]));
    assert_eq!(retried["synthesized"], serde_json::json!([row.id.as_str()]));
    assert_eq!(calls(&sandbox, "agent.calls"), 2);
    assert_eq!(calls(&sandbox, "calls"), 1);
}

#[test]
fn enabling_synthesis_processes_existing_notes_and_expired_analysis_stays_expired() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    enable(&sandbox);
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert_eq!(
        json(&invoke(&sandbox, &[]))["synthesized"],
        serde_json::json!([row.id.as_str()])
    );
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    fs::remove_file(&analysis.target).unwrap();
    sql(&sandbox, "UPDATE recordings SET stage_synthesis='expired'");
    assert_eq!(
        json(&invoke(&sandbox, &[]))["synthesized"],
        serde_json::json!([])
    );
    assert!(!analysis.target.exists());
    assert_eq!(calls(&sandbox, "agent.calls"), 1);
}
