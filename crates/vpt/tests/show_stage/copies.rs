use super::*;
use std::path::Path;

fn error(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).unwrap()
}

#[test]
fn released_copy_uses_frozen_provenance_and_the_exact_body() {
    let (sandbox, id) = configured("show-copy-release");
    let release = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--title", "Public title", "--json"],
    ));
    let path = Path::new(release["written"].as_str().unwrap());
    let bytes = fs::read_to_string(path).unwrap();
    let body = bytes.split_once("\n---\n").unwrap().1;
    let saved = ledger(&sandbox)
        .release(release["sequence"].as_i64().unwrap())
        .unwrap()
        .unwrap();
    let before = fixture::snapshot(&sandbox);
    let first = json(&show(&sandbox, id.as_str(), "released"));
    super::text::text_fields(&first, "released");
    assert_eq!(first["stage"], "released");
    assert_eq!(first["recording"], id.as_str());
    assert_eq!(first["content_sha256"], saved.version.content.hex());
    assert!(path.ends_with(first["note"].as_str().unwrap()));
    assert_eq!(
        first["provenance"]["open_flags"],
        saved.provenance.open_flags
    );
    assert_eq!(first["provenance"]["reviewed"], false);
    assert_eq!(first["provenance"]["redacted"], true);
    let content = first["content"].as_str().unwrap();
    assert!(content.ends_with(body));
    assert!(content.contains("Open flags: 1"));
    assert!(!content.contains("Public title"));
    assert!(!content.contains("person@sample.test"));
    assert_eq!(fixture::snapshot(&sandbox), before);
    let flag = ledger(&sandbox)
        .review_snapshot(&id)
        .unwrap()
        .unwrap()
        .flags
        .into_iter()
        .find(|f| matches!(&f.content, vpt_domain::review::FlagContent::Lexical(_)))
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
    let second = json(&show(&sandbox, id.as_str(), "released"));
    for key in ["content_sha256", "title", "content", "provenance"] {
        assert_eq!(first[key], second[key], "{key}");
    }
}

#[test]
fn transcript_copy_keeps_operator_prose_and_drops_frontmatter() {
    let (sandbox, id) = configured("show-copy-prose");
    let note = ledger(&sandbox)
        .note(&id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let edited = note.text.replace("tags:", "privateHeader: secret\ntags:")
        + "\nOperator prose must survive.\n";
    fs::write(&note.target, &edited).unwrap();
    let before = fixture::snapshot(&sandbox);
    let result = json(&show(&sandbox, id.as_str(), "transcript"));
    let content = result["content"].as_str().unwrap();
    assert!(content.ends_with("\nOperator prose must survive.\n"));
    assert!(!content.contains("privateHeader"));
    assert_ne!(result["content_sha256"], note.digest.hex());
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn released_selection_uses_sequence_skips_missing_files_and_refuses_changed_newest() {
    let (sandbox, id) = configured("show-copy-selection");
    let first = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    let second = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--title", "Another copy", "--json"],
    ));
    let named = |result: &serde_json::Value, release: &serde_json::Value| {
        let file = Path::new(release["written"].as_str().unwrap()).file_name();
        assert!(Path::new(result["note"].as_str().unwrap()).file_name() == file);
    };
    named(&json(&show(&sandbox, id.as_str(), "released")), &second);
    fs::write(second["written"].as_str().unwrap(), "altered").unwrap();
    let before = fixture::snapshot(&sandbox);
    let changed = show(&sandbox, id.as_str(), "released");
    assert_eq!(changed.status.code(), Some(3));
    assert_eq!(error(&changed)["error"]["rule"], "artifact_changed");
    assert_eq!(fixture::snapshot(&sandbox), before);
    fs::remove_file(second["written"].as_str().unwrap()).unwrap();
    named(&json(&show(&sandbox, id.as_str(), "released")), &first);
}

#[test]
fn missing_artifacts_name_only_stages_whose_files_exist() {
    let (sandbox, id) = configured("show-copy-available");
    let before = fixture::snapshot(&sandbox);
    for args in [
        vec!["show", id.as_str(), "--stage", "released", "--json"],
        vec!["show", id.as_str(), "--stage", "audio", "--json"],
        vec!["show", id.as_str(), "--stage", "released", "--dry-run"],
    ] {
        let result = invoke(&sandbox, &args);
        assert_eq!(result.status.code(), Some(2), "{}", stderr(&result));
        assert!(result.stdout.is_empty());
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
    let release = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    fs::remove_file(release["written"].as_str().unwrap()).unwrap();
    let missing = show(&sandbox, id.as_str(), "analysis");
    assert_eq!(missing.status.code(), Some(2));
    let message = error(&missing)["error"]["message"].clone();
    assert!(
        message
            .as_str()
            .unwrap()
            .ends_with("available stages: transcript")
    );
    let note = ledger(&sandbox)
        .note(&id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    fs::remove_file(&note.target).unwrap();
    for stage in ["transcript", "released"] {
        let missing = show(&sandbox, id.as_str(), stage);
        assert_eq!(missing.status.code(), Some(2));
        let message = error(&missing)["error"]["message"].clone();
        assert!(
            message
                .as_str()
                .unwrap()
                .ends_with("available stages: none")
        );
    }
}

#[test]
fn analysis_copy_uses_the_current_owned_body_and_refuses_a_changed_owner() {
    let (sandbox, id) = configured("show-copy-analysis");
    super::text::analyze(
        &sandbox,
        &id,
        r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Meet Alice with Bob via person@sample.test for 1234","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#,
    );
    fs::remove_file(sandbox.path().join("agent.sh")).unwrap();
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
    let result = json(&show(&sandbox, id.as_str(), "analysis"));
    let content = result["content"].as_str().unwrap();
    assert!(content.ends_with("Operator analysis.\n"));
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
    let changed = show(&sandbox, id.as_str(), "analysis");
    assert_eq!(changed.status.code(), Some(3), "{}", stderr(&changed));
}

#[test]
fn the_handoff_verb_and_its_configuration_key_are_unknown() {
    let (sandbox, id) = configured("show-copy-handoff-gone");
    let verb = invoke(
        &sandbox,
        &["handoff", id.as_str(), "--stage", "released", "--json"],
    );
    assert_eq!(verb.status.code(), Some(2));
    assert_eq!(error(&verb)["error"]["message"], "unknown verb handoff");
    fixture::write_config(&sandbox, "[handoff]\nallow_private = true\n");
    let key = show(&sandbox, id.as_str(), "transcript");
    assert_eq!(key.status.code(), Some(2), "{}", stderr(&key));
    assert_eq!(error(&key)["error"]["kind"], "config");
    assert!(stderr(&key).contains("handoff"), "{}", stderr(&key));
    assert!(key.stdout.is_empty());
}
