#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::process::Output;
use support::{Sandbox, run, stderr, stdout};

const EVERY_VERB: &[&[&str]] = &[
    &["--version"],
    &["list"],
    &["storage"],
    &["occasions"],
    &["doctor"],
    &["setup"],
    &["show", "x"],
    &["search", "x"],
    &["show", "x", "--stage", "transcript"],
    &["show", "x", "--stage", "analysis"],
    &["show", "x", "--stage", "released"],
    &["show", "x", "--stage", "brief"],
    &["show", "x", "--stage", "transcript", "--redact"],
    &["show", "x", "--stage", "analysis", "--redact"],
    &["path", "x", "--stage", "transcript"],
    &["path", "occasion", "x", "--stage", "brief"],
    &["review", "list", "x"],
    &["actions", "list"],
    &["review", "dismiss", "x", "f"],
    &["confirm", "tag", "x", "t"],
    &["tag", "rename", "old", "new"],
    &["tag", "rename", "old", "new", "--preview"],
    &["transcribe", "x"],
    &["analyze", "x"],
    &["analyze", "all"],
    &[
        "analyze",
        "all",
        "--since",
        "2026-01-01T00:00:00Z",
        "--preview",
    ],
    &["analyze", "-"],
    &["note", "write", "x"],
    &["note", "verify", "/absent.md", "--recording", "x"],
    &["redact", "x"],
    &["brief", "x"],
    &["import", "/absent.mp3"],
    &["brief", "upcoming"],
    &["ingest", "--preview"],
    &["run", "--preview"],
    &["retention", "run", "--preview"],
    &["symlink", "verify"],
    &["symlink", "deploy"],
    &["transcode"],
];

fn configured(name: &str) -> Sandbox {
    let sandbox = Sandbox::new(name);
    let config = sandbox.config_path();
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::set_permissions(
        config.parent().unwrap(),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::write(
        &config,
        format!(
            "config_version = 1\n[home]\nstate_dir = {:?}\n",
            sandbox.path().join("state/vpt")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
    sandbox
}

const JSON: &[&str] = &["-o", "json"];
const TOON: &[&str] = &["-o", "toon"];
const YAML: &[&str] = &["--output", "yaml"];
const SPELLINGS: [&[&str]; 6] = [
    JSON,
    TOON,
    YAML,
    &["--output=json"],
    &["--output=toon"],
    &["-oyaml"],
];

fn decoded(flag: &[&str], text: &str) -> Value {
    match flag.concat().as_str() {
        "-ojson" | "--output=json" => serde_json::from_str(text).map_err(|error| error.to_string()),
        "-otoon" | "--output=toon" => {
            toon_format::decode_default(text).map_err(|error| error.to_string())
        }
        _ => serde_yaml_ng::from_str(text).map_err(|error| error.to_string()),
    }
    .unwrap_or_else(|error| panic!("{flag:?} did not decode: {error}\n{text}"))
}

fn document(flag: &[&str], output: &Output) -> (Option<i32>, bool, Value) {
    let on_stdout = !output.stdout.is_empty();
    let text = if on_stdout {
        stdout(output)
    } else {
        stderr(output)
    };
    assert!(
        text.ends_with('\n'),
        "{flag:?} ends with a newline: {text:?}"
    );
    (output.status.code(), on_stdout, decoded(flag, &text))
}

#[test]
fn every_verb_prints_the_same_document_in_each_format() {
    let sandbox = configured("formats-every-verb");
    for args in EVERY_VERB {
        let json = document(JSON, &run(sandbox.vpt().args(*args).args(JSON)));
        for flag in &SPELLINGS[1..] {
            let other = document(flag, &run(sandbox.vpt().args(*args).args(*flag)));
            assert_eq!(json, other, "{args:?} with {flag:?}");
        }
    }
}

#[test]
fn a_usage_error_is_one_document_in_the_chosen_format() {
    let sandbox = Sandbox::new("formats-usage-error");
    for flag in SPELLINGS {
        let (code, on_stdout, error) = document(flag, &run(sandbox.vpt().args(flag).arg("show")));
        assert_eq!((code, on_stdout), (Some(2), false), "{flag:?}");
        assert_eq!(error["schema"], "vpt.error/1", "{flag:?}");
        assert_eq!(error["error"]["kind"], "usage", "{flag:?}");
        assert_eq!(error["error"]["message"], "<ID> is required", "{flag:?}");
    }
}

#[test]
fn two_format_flags_are_refused_in_plain_text_with_the_usage() {
    let sandbox = configured("formats-two-flags");
    for pair in [[JSON, TOON], [TOON, YAML], [YAML, JSON], [JSON, JSON]] {
        let output = run(sandbox.vpt().arg("list").args(pair.concat()));
        assert_eq!(output.status.code(), Some(2), "{pair:?}");
        assert_eq!(stdout(&output), "", "{pair:?}");
        let text = stderr(&output);
        let first = text.lines().next().unwrap_or_default();
        assert!(
            first.contains("'--output <FORMAT>' is given more than once"),
            "{text}"
        );
        assert!(text.contains("vpt help"), "{text}");
    }
}

#[test]
fn toon_and_yaml_are_not_json() {
    let sandbox = configured("formats-not-json");
    for flag in [TOON, YAML] {
        let text = stdout(&run(sandbox.vpt().arg("list").args(flag)));
        assert!(!text.starts_with('{'), "{flag:?}: {text}");
        assert!(text.contains("command: "), "{flag:?}: {text}");
    }
}

#[test]
fn the_removed_yaml_flag_and_an_unknown_format_are_usage_errors() {
    let sandbox = configured("formats-removed-yaml");
    for words in [&["list", "--yaml"][..], &["list", "--output", "xml"]] {
        let output = run(sandbox.vpt().args(words));
        assert_eq!(output.status.code(), Some(2), "{words:?}");
        assert_eq!(stdout(&output), "", "{words:?}");
    }
}

#[test]
fn the_removed_json_and_toon_flags_are_usage_errors_naming_the_output_flag() {
    let sandbox = configured("formats-removed-json-toon");
    for (flag, fix) in [("--json", "-o json"), ("--toon", "-o toon")] {
        for words in [&["list", flag][..], &[flag, "list"]] {
            let output = run(sandbox.vpt().args(words));
            assert_eq!(output.status.code(), Some(2), "{words:?}");
            assert_eq!(stdout(&output), "", "{words:?}");
            let text = stderr(&output);
            assert!(
                text.lines().next().unwrap_or_default().contains(fix),
                "{words:?}: {text}"
            );
        }
    }
}
