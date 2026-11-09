use super::*;
use vpt_domain::uncertainty::{Class, Diagnostic};

fn apple_config(sandbox: &Sandbox, floor: f64) {
    fixture::write_config(
        sandbox,
        &format!(
            "[recording]\nlanguage='en-US'\n[engines]\nmain='apple'\n[helper]\npath={:?}\n[reconcile]\nconfidence_floor={floor}\nknown_terms_path={:?}\n",
            sandbox.path().join("bin/speech"),
            sandbox.path().join("terms.txt")
        ),
    );
}

pub(super) fn apple() -> (Sandbox, vpt_application::ports::RecordingRecord) {
    let (sandbox, row) = configured();
    let words: Vec<_> = [
        "we",
        "met",
        "SecretAlice",
        "then",
        "SecretAlice",
        "paid",
        "42",
    ]
    .iter()
    .enumerate()
    .map(|(index, text)| {
        serde_json::json!({
            "text":text, "start":index as f64, "end":index as f64 + 1.0,
            "confidence": if index == 6 { 0.2 } else { 0.9 },
        })
    })
    .collect();
    let document = serde_json::json!({"schema":"vpt.engine/1", "engine":{
        "name":"apple", "family":"apple", "tool":"vpt-macos", "version":"1.0.0", "model":"SpeechTranscriber"
    }, "language":"en-US", "local":true, "segments":[{"start":0.0,"end":7.0,
        "text":"we met SecretAlice then SecretAlice paid 42", "words":words}]});
    let helper = sandbox.path().join("bin/speech");
    std::fs::write(&helper, format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then\nprintf '%s' '{{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}}'\nelse\n/bin/cat <<'DOCUMENT'\n{document}\nDOCUMENT\nfi\n"
    )).unwrap();
    std::fs::set_permissions(helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    apple_config(&sandbox, 0.35);
    (sandbox, row)
}

#[test]
fn builtin_flags_persist_and_follow_known_terms_and_configured_confidence() {
    let (sandbox, row) = apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.uncertainty.diagnostics, [Diagnostic::SingleEngine]);
    assert_eq!(accepted.uncertainty.lexical.len(), 2);
    assert_eq!(accepted.uncertainty.lexical[0].class, Class::LowConfidence);
    assert_eq!(accepted.uncertainty.lexical[0].record_text, "42");
    assert_eq!(accepted.uncertainty.lexical[1].ranges, [2..3, 4..5]);
    assert!(!sandbox.path().join("terms.txt").exists());
    std::fs::write(sandbox.path().join("terms.txt"), "secretalice\n").unwrap();
    apple_config(&sandbox, 0.1);
    json(&run(&sandbox, row.id.as_str(), &[]));
    let changed = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(changed.transcript, accepted.transcript);
    assert_eq!(changed.uncertainty.lexical.len(), 1);
    assert_eq!(
        changed.uncertainty.lexical[0].class,
        Class::AgreedUnverified
    );
    assert_eq!(changed.uncertainty.lexical[0].record_text, "42");
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("SecretAlice"));
    assert_eq!(events.matches("\"event\":\"review_needed\"").count(), 2);
}

#[test]
fn invalid_known_terms_preserve_accepted_flags_and_do_not_leak_text() {
    let (sandbox, row) = apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let before = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
    std::fs::write(sandbox.path().join("terms.txt"), b"private-term-canary\0").unwrap();
    let failed = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stdout.is_empty());
    assert!(!stderr(&failed).contains("private-term-canary"));
    assert_eq!(
        ledger(&sandbox).accepted_transcript(&row.id).unwrap(),
        before
    );
}

#[test]
fn a_generic_checker_keeps_builtin_lexical_flags_and_only_adds_language_diagnostics() {
    let (sandbox, row) = apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let before = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    let path = sandbox.path().join("comparison.json");
    let document = serde_json::json!({"schema":"vpt.engine/1","engine":{
        "name":"comparison","tool":"fixture","version":"1","model":"generic","family":"independent"},
        "language":"de","local":false,"segments":[{"start":0.0,"end":1.0,
            "text":"PrivateDifferent 999","words":[{"text":"PrivateDifferent 999","start":0.0,"end":1.0,"confidence":0.01}]}]});
    std::fs::write(&path, document.to_string()).unwrap();
    let config = std::fs::read_to_string(sandbox.config_path())
        .unwrap()
        .replace("main='apple'", "main='apple'\nchecker='comparison'");
    std::fs::write(sandbox.config_path(), format!("{config}\n[engines.comparison]\nkind='command'\nfamily='independent'\nlocal=false\ncommand=['/bin/cat', {path:?}]\n")).unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let after = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert!(after.lexical_classification);
    assert_eq!(after.transcript, before.transcript);
    assert_eq!(after.uncertainty.lexical, before.uncertainty.lexical);
    assert_eq!(
        after.uncertainty.diagnostics,
        [Diagnostic::LanguageMismatch]
    );
    assert_eq!(after.checker_engine.unwrap().name, "comparison");
}
