#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "confirm/boundaries.rs"]
mod boundaries;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
mod support;
#[path = "confirm/tag_publication.rs"]
mod tag_publication;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{run::*, time::UtcInstant};

fn configured(name: &str) -> (Sandbox, PathBuf) {
    let sandbox = fixture::configured(name);
    let path = sandbox.config_path().with_file_name("known-terms.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path = {path:?}\n"),
    );
    (sandbox, path)
}
fn invoke(sandbox: &Sandbox, term: &str) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["confirm", "--term", term, "--json"]),
        std::time::Duration::from_secs(5),
    )
}
fn success(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}
fn pending(sandbox: &Sandbox, path: &std::path::Path, term: &str) -> VocabularyPublication {
    pending_vocabulary(sandbox, path, term, VocabularyKind::Terms)
}
fn pending_vocabulary(
    sandbox: &Sandbox,
    path: &std::path::Path,
    term: &str,
    kind: VocabularyKind,
) -> VocabularyPublication {
    fixture::seed(sandbox, vec![]);
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let at = UtcInstant { secs: 20 };
    let update = match kind {
        VocabularyKind::Terms => vpt_adapters::known_terms::prepare(path, term),
        VocabularyKind::Tags => vpt_adapters::known_tags::prepare(path, term),
    }
    .unwrap();
    let value = VocabularyPublication {
        kind,
        entry: update.publication(at),
        bytes: update.content().to_vec(),
    };
    let HistoryWrite::Committed(receipt) = ledger.begin_run(RunKind::Publish, at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.commit_step(
            receipt.run,
            &WorkDelta {
                vocabulary: Some(value.clone()),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::Publication,
                    reference: EffectReference::PublicationTarget(path.into())
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    value
}
fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[test]
fn confirms_terms_privately_and_idempotently_without_engines_or_recordings() {
    let (sandbox, path) = configured("confirm-term");
    let first = success(&invoke(&sandbox, "  Straße\n  Private Name  "));
    assert_eq!(first["confirmed"]["added"], true);
    assert_eq!(fs::read_to_string(&path).unwrap(), "Straße Private Name\n");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let second = success(&invoke(&sandbox, "STRASSE private name"));
    assert_eq!(second["confirmed"]["added"], false);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert!(
        ledger(&sandbox)
            .vocabulary_publication(&path)
            .unwrap()
            .is_none()
    );
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("Private Name"));
    assert!(events.contains("confirm"));
    assert!(events.contains("effect_observed"));
}

#[test]
fn invalid_confirmation_input_is_observational() {
    let (sandbox, _) = configured("confirm-invalid");
    let before = fixture::snapshot(&sandbox);
    for term in ["", " \t\n", "Private\u{7f}canary"] {
        let output = invoke(&sandbox, term);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("canary"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn startup_publishes_saved_terms_before_appending_new_work() {
    let (sandbox, path) = configured("confirm-recovery");
    let saved = pending(&sandbox, &path, "Recovered");
    success(&invoke(&sandbox, "New term"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Recovered\nNew term\n");
    assert!(
        ledger(&sandbox)
            .vocabulary_publication(&saved.entry.target)
            .unwrap()
            .is_none()
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn recovery_acknowledges_already_published_terms_without_duplication() {
    let (sandbox, path) = configured("confirm-recovery-published");
    let saved = pending(&sandbox, &path, "Recovered");
    vpt_adapters::known_terms::Update::restore(&saved.entry, saved.bytes)
        .unwrap()
        .publish()
        .unwrap();
    success(&invoke(&sandbox, "Recovered"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Recovered\n");
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn recovery_preserves_operator_changes_and_keeps_the_pending_intent() {
    let (sandbox, path) = configured("confirm-recovery-modified");
    let saved = pending(&sandbox, &path, "Recovered");
    fs::write(&path, "Operator edit\n").unwrap();
    let output = invoke(&sandbox, "New term");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), "Operator edit\n");
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&path).unwrap(),
        Some(saved)
    );
}

#[test]
fn changing_the_configured_terms_path_does_not_redirect_pending_work() {
    let (sandbox, path) = configured("confirm-recovery-path");
    let saved = pending(&sandbox, &path, "Recovered");
    let other = path.with_file_name("other.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path = {other:?}\n"),
    );
    let output = invoke(&sandbox, "New term");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(!path.exists());
    assert!(!other.exists());
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&path).unwrap(),
        Some(saved)
    );
}
