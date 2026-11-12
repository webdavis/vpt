use super::*;

const PRIVATE: [&str; 5] = ["Alice", "Bob", "person@sample.test", "303-555-0123", "1234"];

fn private(sandbox: &Sandbox) {
    json(&invoke(sandbox, &["confirm", "--term", "Alice", "--json"]));
    fs::write(
        sandbox.path().join("home/.vpt/transcripts/Bob.md"),
        "# Bob\n",
    )
    .unwrap();
}

fn redacted(sandbox: &Sandbox, id: &str, stage: &str) -> serde_json::Value {
    let plain = json(&show(sandbox, id, stage));
    let before = fixture::snapshot(sandbox);
    let result = json(&invoke(
        sandbox,
        &["show", id, "--stage", stage, "--redact", "--json"],
    ));
    assert_eq!(fixture::snapshot(sandbox), before, "--redact wrote nothing");
    let mut fields = result.clone();
    for key in ["content_sha256", "generated_at"] {
        fields.as_object_mut().unwrap().remove(key);
    }
    let document = fields.to_string();
    let path = plain["note"].as_str().unwrap();
    let stem = path.rsplit('/').next().unwrap().trim_end_matches(".md");
    for word in PRIVATE.iter().chain([&path, &stem]) {
        assert!(!document.contains(word), "{word}: {document}");
    }
    assert!(document.contains("redacted extract, not a verbatim record"));
    assert_eq!(result["note"], json!(null));
    assert_eq!(result["title"], format!("vpt {stage} {id}"));
    assert_eq!(result["provenance"]["redacted"], true);
    for key in ["segments", "summary", "actions"] {
        assert!(result.get(key).is_none(), "{key}");
    }
    let stores = vpt_adapters::FilesystemStores::open(&[sandbox.path().to_path_buf()]).unwrap();
    let emitted = result["content"].as_str().unwrap().as_bytes();
    assert_eq!(result["content_sha256"], stores.digest_bytes(emitted).hex());
    result
}

fn mark(sandbox: &Sandbox, extra: &str) {
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(sandbox.config_path())
        .unwrap()
        .write_all(format!("\n{extra}[share.redact]\nflagged_spans = \"mark\"\n").as_bytes())
        .unwrap();
}

fn content(result: &serde_json::Value) -> &str {
    result["content"].as_str().unwrap()
}

#[test]
fn redacted_transcript_is_the_masked_text_only() {
    let (sandbox, id) = configured("show-redact-transcript");
    private(&sandbox);
    let plain = json(&show(&sandbox, id.as_str(), "transcript"));
    assert!(content(&plain).contains("Alice"));
    assert!(plain["segments"].is_array());
    let omitted = redacted(&sandbox, id.as_str(), "transcript");
    assert!(content(&omitted).contains("&#91;term 1&#93;"));
    assert!(!content(&omitted).contains("00&#58;00"));
    mark(&sandbox, "[share]\nkeep_timecodes = true\n");
    let marked = redacted(&sandbox, id.as_str(), "transcript");
    assert!(content(&marked).contains("&#91;unverified&#93;"));
    assert!(content(&marked).contains("00&#58;00"));
}

#[test]
fn redacted_analysis_is_the_masked_text_only() {
    let (sandbox, id) = configured("show-redact-analysis");
    private(&sandbox);
    super::text::analyze(
        &sandbox,
        &id,
        r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Meet Alice via person@sample.test","sources":["00:00-00:01"]}],"actions":[{"text":"Call 303-555-0123","sources":[]}],"tags":[],"relations":[]}"#,
    );
    let plain = json(&show(&sandbox, id.as_str(), "analysis"));
    assert_eq!(plain["summary"][0]["finding"], "built-on-flagged-text");
    let omitted = redacted(&sandbox, id.as_str(), "analysis");
    assert!(!content(&omitted).contains("Meet"));
    assert!(content(&omitted).contains("Call &#91;phone"));
    mark(&sandbox, "");
    let marked = redacted(&sandbox, id.as_str(), "analysis");
    assert!(content(&marked).contains("Meet &#91;term"));
    assert!(content(&marked).contains("&#91;unverified&#93;"));
}

#[test]
fn redact_on_a_brief_a_release_or_no_stage_is_a_usage_error() {
    let (sandbox, id) = configured("show-redact-usage");
    let before = fixture::snapshot(&sandbox);
    for args in [
        vec!["show", id.as_str(), "--redact", "--json"],
        vec![
            "show",
            id.as_str(),
            "--stage",
            "released",
            "--redact",
            "--json",
        ],
        vec![
            "show",
            "2026-08-25T100000-cd3e938bdd0d",
            "--stage",
            "brief",
            "--redact",
            "--json",
        ],
    ] {
        let output = invoke(&sandbox, &args);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(stderr(&output).contains("--redact"), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
}

fn split(name: &str, term: Option<&str>, texts: &[&str], halves: &[(usize, &str)]) {
    for mark in [false, true] {
        let sandbox = fixture::configured(&format!("{name}-{mark}"));
        let flags = halves
            .iter()
            .filter(|_| mark)
            .map(|(index, text)| super::recording::flag_word(*index, text))
            .collect();
        let id = super::recording::seed_segments(&sandbox, flags, texts);
        if let Some(term) = term {
            json(&invoke(&sandbox, &["confirm", "--term", term, "--json"]));
        }
        if mark {
            self::mark(&sandbox, "");
        }
        let output = invoke(
            &sandbox,
            &[
                "show",
                id.as_str(),
                "--stage",
                "transcript",
                "--redact",
                "--json",
            ],
        );
        let emitted = format!("{}{}", support::stdout(&output), stderr(&output));
        for (_, half) in halves {
            assert!(!emitted.contains(half), "{mark} {half}: {emitted}");
        }
        assert_eq!(output.status.code(), Some(3), "{mark} {emitted}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["rule"], "residue");
    }
}

#[test]
fn a_term_split_across_segments_is_refused_rather_than_shown_in_halves() {
    split(
        "show-redact-split-term",
        Some("Acme Corp"),
        &["Send it to Acme", "Corp today"],
        &[(3, "Acme"), (4, "Corp")],
    );
}

#[test]
fn a_phone_number_split_across_segments_is_refused_rather_than_shown_in_halves() {
    split(
        "show-redact-split-phone",
        None,
        &["Please call 303-555", "0123 today"],
        &[(2, "303-555"), (3, "0123")],
    );
}

#[test]
fn a_pending_note_publication_stops_both_views() {
    let (sandbox, id) = configured("show-redact-pending");
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection
        .execute_batch("CREATE TRIGGER hold BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT,'held'); END")
        .unwrap();
    fixture::preserve_wal(&connection);
    let write = invoke(&sandbox, &["note", "write", id.as_str(), "--json"]);
    assert_eq!(write.status.code(), Some(1), "{}", stderr(&write));
    assert_eq!(ledger(&sandbox).pending_publications().unwrap().len(), 1);
    let before = fixture::snapshot(&sandbox);
    for redact in [false, true] {
        let mut args = vec!["show", id.as_str(), "--stage", "transcript", "--json"];
        if redact {
            args.push("--redact");
        }
        let output = invoke(&sandbox, &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{redact}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty());
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
}
