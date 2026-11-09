use super::*;
use std::fs;
use vpt_domain::uncertainty::Diagnostic;

#[test]
fn checker_policy_executes_sequentially_and_preserves_the_correct_transcript() {
    for mode in ["always", "on_failure"] {
        for (main_ok, checker_ok) in [(true, true), (true, false), (false, true), (false, false)] {
            let (sandbox, row) = configured();
            json(&run(&sandbox, row.id.as_str(), &[]));
            let previous = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
            let audio = fs::read(&row.audio_path).unwrap();
            pair_fixture::configure(&sandbox, mode, false, "");
            for (name, succeeds) in [("primary", main_ok), ("secondary", checker_ok)] {
                fs::write(
                    sandbox.path().join(format!("{name}.exit")),
                    if succeeds { "0" } else { "17" },
                )
                .unwrap();
            }
            let output = run(&sandbox, row.id.as_str(), &[]);
            let checker_ran = mode == "always" || !main_ok;
            assert_eq!(
                pair_fixture::calls(&sandbox),
                if checker_ran {
                    "primary:start\nprimary:end\nsecondary:start\nsecondary:end\n"
                } else {
                    "primary:start\nprimary:end\n"
                }
            );
            assert_eq!(fs::read(&row.audio_path).unwrap(), audio);
            for name in if checker_ran {
                vec!["primary", "secondary"]
            } else {
                vec!["primary"]
            } {
                assert_eq!(
                    fs::read(sandbox.path().join(format!("{name}.input"))).unwrap(),
                    audio
                );
                let args = fs::read_to_string(sandbox.path().join(format!("{name}.args"))).unwrap();
                assert_ne!(args.lines().next(), Some(row.audio_path.to_str().unwrap()));
                assert_eq!(args.lines().nth(1), Some("fr-CA"));
                assert!(!std::path::Path::new(args.lines().next().unwrap()).exists());
            }
            let accepted = ledger(&sandbox).accepted_transcript(&row.id).unwrap();
            if !main_ok && !checker_ok {
                assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
                assert_eq!(accepted, previous);
                assert_eq!(
                    ledger(&sandbox)
                        .by_id(&row.id)
                        .unwrap()
                        .unwrap()
                        .stages
                        .transcribe,
                    StageState::Failed
                );
            } else {
                json(&output);
                let accepted = accepted.unwrap();
                let name = if main_ok { "primary" } else { "secondary" };
                assert_eq!(accepted.transcript.engine.name, name);
                assert_eq!(accepted.transcript.local, main_ok);
                assert_eq!(accepted.from_checker, !main_ok);
                assert_eq!(
                    accepted.transcript.segments[0].words[0].text,
                    format!("Private {name} 42")
                );
                assert_eq!(
                    accepted.transcript.segments[0].words[0].confidence,
                    Some(0.01)
                );
                assert!(!accepted.lexical_classification);
                assert!(accepted.uncertainty.lexical.is_empty());
                assert_eq!(
                    accepted.uncertainty.diagnostics,
                    if main_ok && checker_ran && checker_ok {
                        vec![]
                    } else {
                        vec![Diagnostic::SingleEngine]
                    }
                );
                assert_eq!(
                    accepted.checker_engine.is_some(),
                    main_ok && checker_ran && checker_ok
                );
                assert_eq!(
                    ledger(&sandbox)
                        .by_id(&row.id)
                        .unwrap()
                        .unwrap()
                        .stages
                        .transcribe,
                    StageState::Succeeded
                );
            }
            for (name, saved) in [
                ("primary", main_ok),
                ("secondary", checker_ran && checker_ok),
            ] {
                let path = sandbox
                    .path()
                    .join(format!("home/.vpt/engine-outputs/{}.{name}.json", row.id));
                assert_eq!(path.exists(), saved);
                if saved {
                    assert_eq!(
                        fs::read(&path).unwrap(),
                        fs::read(sandbox.path().join(format!("{name}.document"))).unwrap()
                    );
                    assert_eq!(
                        fs::metadata(path).unwrap().permissions().mode() & 0o777,
                        0o600
                    );
                }
            }
            let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
            assert_eq!(
                events.contains("transcribe_failed"),
                !main_ok && !checker_ok
            );
            assert!(!events.contains("Private"));
            assert!(!stdout(&output).contains("Private"));
            assert!(!stderr(&output).contains("Private"));
        }
    }
}

#[test]
fn invocation_language_selects_the_override_pair_and_reports_configured_locality() {
    let (sandbox, row) = configured();
    pair_fixture::configure(
        &sandbox,
        "always",
        false,
        "[engines.by_language.ja-JP]\nmain='secondary'\nchecker=''\n",
    );
    pair_fixture::reported_language(&sandbox, "secondary", "fr");
    let before = fixture::snapshot(&sandbox);
    let plan = json(&run(
        &sandbox,
        row.id.as_str(),
        &["--language", "ja-JP", "--dry-run"],
    ));
    assert_eq!(plan["language"], "ja-JP");
    assert_eq!(plan["engines"].as_array().unwrap().len(), 1);
    assert_eq!(plan["engines"][0]["name"], "secondary");
    assert_eq!(plan["engines"][0]["local"], false);
    assert_eq!(fixture::snapshot(&sandbox), before);
    json(&run(&sandbox, row.id.as_str(), &["--language", "ja-JP"]));
    assert_eq!(
        pair_fixture::calls(&sandbox),
        "secondary:start\nsecondary:end\n"
    );
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.requested_language, "ja-JP");
    assert_eq!(accepted.transcript.language, "fr");
    assert!(!accepted.transcript.local);
    assert_eq!(
        accepted.uncertainty.diagnostics,
        [Diagnostic::SingleEngine, Diagnostic::LanguageMismatch]
    );
    assert_eq!(
        fs::read_to_string(sandbox.path().join("secondary.args"))
            .unwrap()
            .lines()
            .nth(1),
        Some("ja-JP")
    );
}

#[test]
fn matching_families_require_explicit_opt_in_before_either_engine_starts() {
    let (sandbox, row) = configured();
    pair_fixture::configure(&sandbox, "always", true, "");
    let before = fixture::snapshot(&sandbox);
    for args in [vec![], vec!["--dry-run"]] {
        let output = run(&sandbox, row.id.as_str(), &args);
        assert_eq!(output.status.code(), Some(3));
        for name in ["primary", "secondary", "shared"] {
            assert!(stderr(&output).contains(name));
        }
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
    pair_fixture::configure(&sandbox, "always", true, "allow_same_family=true");
    json(&run(&sandbox, row.id.as_str(), &[]));
    assert_eq!(
        pair_fixture::calls(&sandbox),
        "primary:start\nprimary:end\nsecondary:start\nsecondary:end\n"
    );
    let accepted = ledger(&sandbox)
        .accepted_transcript(&row.id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.transcript.engine.family, "shared");
    assert_eq!(accepted.checker_engine.unwrap().family, "shared");
}
