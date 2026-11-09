use super::*;
#[test]
fn analysis_handoff_uses_current_owned_body_without_running_an_agent() {
    let (sandbox, id) = configured("handoff-analysis");
    let script = sandbox.path().join("agent.sh");
    let proposal = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    fs::write(&proposal, r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Meet Alice with Bob via person@sample.test for 1234","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[synthesis]\ncommand=['/bin/sh', {script:?}, {proposal:?}]\n[handoff]\nallow_private=true\n"
        ),
    );
    json(&invoke(&sandbox, &["synthesize", id.as_str(), "--json"]));
    fs::remove_file(script).unwrap();
    let saved = ledger(&sandbox)
        .note(&id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    fs::write(
        &saved.target,
        format!("{}\nOperator analysis.\n", saved.text),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let result = json(&handoff(&sandbox, id.as_str(), "analysis"));
    assert_eq!(result["kind"], "analysis");
    assert!(
        result["content"]
            .as_str()
            .unwrap()
            .ends_with("Operator analysis.\n")
    );
    assert_eq!(result["provenance"]["reviewed"], false);
    assert!(result["provenance"]["open_flags"].as_u64().unwrap() >= 1);
    assert_eq!(fixture::snapshot(&sandbox), before);
    fs::write(
        &saved.target,
        saved
            .text
            .replace("vptStage: \"analysis\"", "vptStage: \"transcript\""),
    )
    .unwrap();
    let changed = handoff(&sandbox, id.as_str(), "analysis");
    assert_eq!(changed.status.code(), Some(3), "{}", stderr(&changed));
}
