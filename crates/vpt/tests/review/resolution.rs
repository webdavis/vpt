use super::*;
use vpt_domain::{
    identity::RecordingId,
    review::{FlagContent, Resolution},
};

pub(super) fn setup() -> (Sandbox, RecordingId, std::path::PathBuf) {
    let (sandbox, id) = configured();
    let terms = sandbox.config_path().with_file_name("known-terms.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path = {terms:?}\n"),
    );
    (sandbox, id, terms)
}

pub(super) fn inputs(sandbox: &Sandbox, id: &RecordingId) -> NoteInputs {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .note_inputs(id)
        .unwrap()
        .unwrap()
}

pub(super) fn identifier(sandbox: &Sandbox, id: &RecordingId, class: &str) -> String {
    inputs(sandbox, id)
        .review
        .flags
        .into_iter()
        .find(|flag| match &flag.content {
            FlagContent::Lexical(flag) => flag.class.as_str() == class,
            FlagContent::Diagnostic(flag) => flag.as_str() == class,
            FlagContent::Verification(flag) => flag.finding.as_str() == class,
        })
        .unwrap()
        .identifier
        .hex()
}

pub(super) fn resolve(
    sandbox: &Sandbox,
    id: &RecordingId,
    flag: &str,
    action: &[&str],
) -> std::process::Output {
    support::run_with_budget(
        sandbox
            .vpt()
            .args(["review", id.as_str(), "--resolve", flag, "--json"])
            .args(action),
        std::time::Duration::from_secs(5),
    )
}

#[test]
fn correction_publishes_note_and_known_term_without_changing_accepted_words() {
    let (sandbox, id, terms) = setup();
    let flag = identifier(&sandbox, &id, "numeric");
    let before = inputs(&sandbox, &id).review.accepted;
    let result = json(&resolve(
        &sandbox,
        &id,
        &flag,
        &["--correct", "Private correction"],
    ));
    assert_eq!(result["open_flags"], 2);
    assert_eq!(result["resolved"]["state"], "corrected");
    assert_eq!(result["resolved"]["changed"], true);
    let after = inputs(&sandbox, &id);
    assert_eq!(after.review.accepted, before);
    assert!(
        matches!(&after.review.flags.iter().find(|f| f.identifier.hex() == flag).unwrap().resolution,
        Resolution::Corrected { text, .. } if text == "Private correction")
    );
    let note = &after.notes[0];
    assert!(note.text.contains("[corrected: Private correction]"));
    assert!(note.text.contains("vpt-open-flags: 2"));
    assert_eq!(std::fs::read_to_string(&note.target).unwrap(), note.text);
    assert_eq!(
        std::fs::read_to_string(&terms).unwrap(),
        "Private correction\n"
    );
    let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(log.contains("flag_resolved"));
    assert!(!log.contains("Private correction"));
    assert!(!log.contains("SecretAlice"));
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn repeated_confirmation_keeps_resolution_time_and_known_term_bytes() {
    let (sandbox, id, terms) = setup();
    let flag = identifier(&sandbox, &id, "agreed-unverified");
    json(&resolve(&sandbox, &id, &flag, &["--confirm"]));
    let first = inputs(&sandbox, &id);
    let modified = std::fs::metadata(&terms).unwrap().modified().unwrap();
    let repeated = json(&resolve(&sandbox, &id, &flag, &["--confirm"]));
    assert_eq!(repeated["resolved"]["changed"], false);
    let second = inputs(&sandbox, &id);
    assert_eq!(first.review.flags, second.review.flags);
    assert_eq!(std::fs::read_to_string(&terms).unwrap(), "SecretAlice\n");
    assert_eq!(
        std::fs::metadata(terms).unwrap().modified().unwrap(),
        modified
    );
    assert!(!second.notes[0].text.contains("[unverified: SecretAlice]"));
}

#[test]
fn dismissal_and_diagnostic_confirmation_close_flags_without_learning_terms() {
    let (sandbox, id, terms) = setup();
    let flag = identifier(&sandbox, &id, "low-confidence");
    assert_eq!(
        json(&resolve(&sandbox, &id, &flag, &["--dismiss"]))["open_flags"],
        2
    );
    let diagnostic = identifier(&sandbox, &id, "single-engine");
    let result = json(&resolve(&sandbox, &id, &diagnostic, &["--confirm"]));
    assert_eq!(result["diagnostic_count"], 0);
    assert!(!terms.exists());
    assert!(
        inputs(&sandbox, &id).notes[0]
            .text
            .contains("vpt-diagnostics: 0")
    );
}

#[test]
fn invalid_actions_are_observational_and_private() {
    let (sandbox, id, _) = setup();
    let numeric = identifier(&sandbox, &id, "numeric");
    let diagnostic = identifier(&sandbox, &id, "single-engine");
    let hidden = identifier(&sandbox, &id, "formatting");
    let before = fixture::snapshot(&sandbox);
    for (flag, action) in [
        ("bad-id", vec!["--confirm"]),
        (
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            vec!["--confirm"],
        ),
        (diagnostic.as_str(), vec!["--correct", "Private canary"]),
        (hidden.as_str(), vec!["--dismiss"]),
        (numeric.as_str(), vec!["--correct", "Private\u{1}canary"]),
        (numeric.as_str(), vec!["--correct", " "]),
        (
            numeric.as_str(),
            vec!["--correct", "Private canary", "--confirm"],
        ),
    ] {
        let output = resolve(&sandbox, &id, flag, &action);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn malformed_note_prevents_both_resolution_and_known_term_commit() {
    let (sandbox, id, terms) = setup();
    json(&run(sandbox.vpt().args([
        "note",
        "write",
        id.as_str(),
        "--json",
    ])));
    let before = inputs(&sandbox, &id);
    let note = &before.notes[0];
    let edited = note.text.replace("<!-- vpt:content end -->", "");
    std::fs::write(&note.target, &edited).unwrap();
    let flag = identifier(&sandbox, &id, "numeric");
    let output = resolve(&sandbox, &id, &flag, &["--correct", "Private canary"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private canary"));
    let after = inputs(&sandbox, &id);
    assert_eq!(after.review.flags, before.review.flags);
    assert_eq!(after.notes, before.notes);
    assert_eq!(std::fs::read_to_string(&note.target).unwrap(), edited);
    assert!(!terms.exists());
}
