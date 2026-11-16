mod support;

use support::{Sandbox, run, stderr, stdout};

fn error_document(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("json error on stderr")
}

#[test]
fn a_mistyped_command_suggests_the_closest_one_on_stderr() {
    let sandbox = Sandbox::new("cli-suggest-command");
    let output = run(sandbox.vpt().arg("transcrib"));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let text = stderr(&output);
    assert!(text.starts_with("vpt: "), "{text}");
    assert!(text.contains("'transcribe'"), "{text}");
}

#[test]
fn a_mistyped_flag_suggests_the_closest_one_in_the_error_document() {
    let sandbox = Sandbox::new("cli-suggest-flag");
    let output = run(sandbox.vpt().args(["--json", "list", "--stge", "note"]));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let document = error_document(&stderr(&output));
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], "usage");
    assert!(
        document["error"]["diagnostics"]
            .to_string()
            .contains("--stage"),
        "{document}"
    );
}

#[test]
fn a_value_joined_with_equals_reads_like_a_separate_value() {
    let sandbox = Sandbox::new("cli-equals");
    let joined = run(sandbox
        .vpt()
        .args(["--json", "show", "--stage=audio", "memo"]));
    let separate = run(sandbox
        .vpt()
        .args(["--json", "show", "--stage", "audio", "memo"]));
    assert_eq!(joined.status.code(), Some(2));
    assert_eq!(stderr(&joined), stderr(&separate));
}

#[test]
fn private_text_never_appears_in_a_usage_error() {
    let sandbox = Sandbox::new("cli-private-text");
    let canary = "Private canary";
    for args in [
        vec!["confirm", "term", canary, "--term", "other"],
        vec!["confirm", "term", canary, "--preview"],
        vec!["confirm", "--tag", canary],
        vec!["confirm", "tag", "id", canary, "--reject-tag", "other"],
        vec!["confirm", "relation", "id", canary, "--term", "other"],
        vec!["confirm", "term", canary, "other"],
        vec!["confirm", "--term", canary],
        vec!["confirm", "tag", canary],
        vec!["confirm", "tag", "id", canary, "other"],
        vec!["confirm", "reject-tag", "id", canary, "extra"],
        vec!["confirm", "relation", "id", canary, "extra"],
        vec!["confirm", canary],
        vec!["show", "id", "--tag", canary],
        vec!["review", "memo", "--correct", canary],
        vec!["review", "list", "memo", "--correct", canary],
        vec!["review", "correct", "memo", "f", canary, "--dismiss"],
        vec!["review", "correct", "memo", "f", canary, "extra"],
        vec!["review", "correct", "memo", canary],
        vec!["review", "confirm", "memo", "f", canary],
        vec!["review", "dismiss", "memo", "f", canary],
        vec!["review", "list", "memo", canary],
        vec!["import", "--title", canary],
        vec!["redact", "--title", canary],
        vec!["brief", "--title", canary, "--upcoming"],
        vec!["brief", "upcoming", canary],
        vec!["brief", "upcoming", "--title", canary],
        vec!["brief", "--participant", canary],
        vec!["tag", "rename", canary],
        vec!["tag", "rename", "--Private canary", "new"],
        vec!["import", "--Private canary.m4a"],
        vec!["note", "verify", "--Private canary.md", "--recording", "r"],
        vec!["review", "confirm", "memo", "f", "--json=Private canary"],
    ] {
        for format in [None, Some("--json")] {
            let output = run(sandbox.vpt().args(format).args(&args));
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert_eq!(stdout(&output), "", "{args:?}");
            assert!(
                !stderr(&output).contains(canary),
                "{args:?} {}",
                stderr(&output)
            );
        }
    }
}

#[test]
fn a_rejected_value_is_named_by_its_flag_and_never_echoed() {
    let sandbox = Sandbox::new("cli-value-not-echoed");
    let output = run(sandbox
        .vpt()
        .args(["analyze", "--all", "--since", "Private time"]));
    assert_eq!(output.status.code(), Some(2));
    let text = stderr(&output);
    assert!(!text.contains("Private"), "{text}");
    assert!(
        text.starts_with("vpt: invalid value for '--since <SINCE>'"),
        "{text}"
    );
}
