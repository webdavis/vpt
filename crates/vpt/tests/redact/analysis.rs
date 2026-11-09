use super::*;
fn synthesized() -> (Sandbox, vpt_domain::identity::RecordingId) {
    let sandbox = fixture::configured("redact-analysis");
    let id = recording::seed_flags(&sandbox, true);
    let script = sandbox.path().join("agent.sh");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    let proposal = sandbox.path().join("proposal.json");
    fs::write(&proposal, r#"{"schema":"vpt.proposal/1","by":"private agent","summary":[{"text":"Meet Alice with Bob via person@sample.test for 1234","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#).unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[synthesis]\ncommand=['/bin/sh', {script:?}, {proposal:?}]\n"),
    );
    json(&invoke(&sandbox, &["synthesize", id.as_str(), "--json"]));
    (sandbox, id)
}
#[test]
fn redact_analysis_omits_whole_claims_and_explicit_mark_retains_the_accepted_claim() {
    let (sandbox, id) = synthesized();
    let original = ledger(&sandbox)
        .note(&id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let first = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--stage", "analysis", "--json"],
    ));
    assert_eq!(first["omitted"], 1);
    assert!(
        !fs::read_to_string(first["written"].as_str().unwrap())
            .unwrap()
            .contains("Meet Alice")
    );
    assert!(first["written"].as_str().unwrap().contains("-analysis-"));
    fixture::write_config(&sandbox, "[share.redact]\nflagged_spans='mark'\n");
    let second = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--stage", "analysis", "--json"],
    ));
    assert_eq!(second["marked"], 1);
    let marked = fs::read_to_string(second["written"].as_str().unwrap()).unwrap();
    assert!(marked.contains("Meet Alice"));
    assert!(marked.contains("unverified"));
    assert!(!marked.contains("5678"));
    assert_eq!(fs::read_to_string(original.target).unwrap(), original.text);
}
#[test]
fn redact_analysis_rechecks_stale_findings_after_the_source_flag_is_resolved() {
    let (sandbox, id) = synthesized();
    let ledger = ledger(&sandbox);
    let flag = ledger
        .transcript_flags(&id)
        .unwrap()
        .into_iter()
        .find(|f| matches!(f.content, vpt_domain::review::FlagContent::Lexical(_)))
        .unwrap();
    json(&invoke(
        &sandbox,
        &[
            "review",
            id.as_str(),
            "--resolve",
            &flag.identifier.hex(),
            "--confirm",
            "--json",
        ],
    ));
    let source = ledger.note(&id, NoteStage::Analysis).unwrap().unwrap();
    let before = fs::read(&source.target).unwrap();
    let result = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--stage", "analysis", "--json"],
    ));
    assert_eq!(result["omitted"], 0);
    assert!(
        fs::read_to_string(result["written"].as_str().unwrap())
            .unwrap()
            .contains("Meet Alice")
    );
    assert_eq!(fs::read(source.target).unwrap(), before);
    let release = ledger
        .release(result["sequence"].as_i64().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(release.provenance.open_lexical_flags, 0);
    assert_eq!(release.provenance.open_flags, 0);
}
