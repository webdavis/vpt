use super::*;

pub(super) fn text_fields(result: &serde_json::Value, stage: &str) {
    let content = result["content"].as_str().unwrap();
    assert!(
        content.starts_with(&format!("# vpt {stage} copy, ")),
        "{content}"
    );
    assert!(!content.contains("vpt-stage"), "{content}");
    let stem = result["note"].as_str().unwrap().rsplit('/').next().unwrap();
    assert_eq!(
        result["title"],
        format!("vpt {stage} {}", stem.trim_end_matches(".md"))
    );
    assert_eq!(result["content_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(result["provenance"]["produced_by"], "vpt");
    assert!(result["generated_at"].is_string());
}

pub(super) fn analyze(sandbox: &Sandbox, id: &vpt_domain::identity::RecordingId, proposal: &str) {
    let script = sandbox.path().join("agent.sh");
    let path = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    fs::write(&path, proposal).unwrap();
    fixture::write_config(
        sandbox,
        &format!("[analysis]\ncommand=['/bin/sh', {script:?}, {path:?}]\n"),
    );
    json(&invoke(sandbox, &["analyze", id.as_str(), "--json"]));
}

#[test]
fn transcript_returns_the_text_and_its_provenance_beside_the_segments() {
    let (sandbox, id) = configured("show-text-transcript");
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, id.as_str(), "transcript"));
    text_fields(&result, "transcript");
    assert_eq!(result["segments"].as_array().unwrap().len(), 1);
    let provenance = &result["provenance"];
    assert_eq!(provenance["redacted"], false);
    assert_eq!(provenance["reviewed"], false);
    assert_eq!(provenance["open_flags"], 1);
    assert_eq!(provenance["engines"], json!(["apple:fixture"]));
    assert!(provenance["captured_at"].is_string());
    let human = invoke(&sandbox, &["show", id.as_str(), "--stage", "transcript"]);
    assert_eq!(human.status.code(), Some(0), "{}", stderr(&human));
    assert_eq!(support::stdout(&human), result["content"].as_str().unwrap());
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn analysis_returns_the_text_beside_the_summary_and_actions() {
    let (sandbox, id) = configured("show-text-analysis");
    analyze(
        &sandbox,
        &id,
        r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Alice meets Bob","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#,
    );
    let result = json(&show(&sandbox, id.as_str(), "analysis"));
    text_fields(&result, "analysis");
    assert_eq!(result["summary"][0]["text"], "Alice meets Bob");
    assert!(
        result["content"]
            .as_str()
            .unwrap()
            .contains("Alice meets Bob")
    );
    assert_eq!(result["provenance"]["redacted"], false);
}
