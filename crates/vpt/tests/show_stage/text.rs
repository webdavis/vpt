use super::*;

fn text_fields(result: &serde_json::Value, stage: &str) {
    let content = result["content"].as_str().unwrap();
    assert!(
        content.starts_with(&format!("# vpt {stage} copy, ")),
        "{content}"
    );
    assert!(!content.contains("vptStage"), "{content}");
    let stem = result["note"].as_str().unwrap().rsplit('/').next().unwrap();
    assert_eq!(
        result["title"],
        format!("vpt {stage} {}", stem.trim_end_matches(".md"))
    );
    assert_eq!(result["content_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(result["provenance"]["produced_by"], "vpt");
    assert!(result["generated_at"].is_string());
}

pub(super) fn synthesize(
    sandbox: &Sandbox,
    id: &vpt_domain::identity::RecordingId,
    proposal: &str,
) {
    let script = sandbox.path().join("agent.sh");
    let path = sandbox.path().join("proposal.json");
    fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    fs::write(&path, proposal).unwrap();
    fixture::write_config(
        sandbox,
        &format!("[synthesis]\ncommand=['/bin/sh', {script:?}, {path:?}]\n"),
    );
    json(&invoke(sandbox, &["synthesize", id.as_str(), "--json"]));
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
    synthesize(
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

#[test]
fn released_returns_the_saved_redacted_copy() {
    let (sandbox, id) = configured("show-text-released");
    let release = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, id.as_str(), "released"));
    text_fields(&result, "released");
    assert_eq!(result["recording"], id.as_str());
    assert_eq!(result["stage"], "released");
    assert_eq!(result["provenance"]["redacted"], true);
    let written = release["written"].as_str().unwrap();
    assert!(written.ends_with(result["note"].as_str().unwrap()));
    assert!(
        !result["content"]
            .as_str()
            .unwrap()
            .contains("person@sample.test")
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn brief_returns_the_occasion_text_and_its_source_recordings() {
    let (sandbox, id) = configured("show-text-brief");
    json(&invoke(
        &sandbox,
        &["confirm", id.as_str(), "--tag", "planning", "--json"],
    ));
    let created = json(&invoke(
        &sandbox,
        &[
            "brief",
            "--title",
            "Planning",
            "--at",
            "2026-08-25T10:00:00-06:00",
            "--tag",
            "planning",
            "--json",
        ],
    ));
    let occasion = created["occasion"]["id"].as_str().unwrap();
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, occasion, "brief"));
    text_fields(&result, "brief");
    assert_eq!(result["occasion"], occasion);
    assert_eq!(result["recordings"], json!([id.as_str()]));
    assert!(result.get("recording").is_none());
    assert_eq!(
        result["provenance"]["occasion_at"],
        "2026-08-25T10:00:00-06:00"
    );
    assert!(!result["content"].as_str().unwrap().contains("vptOccasion:"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
