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
    &["path", "x", "--stage", "transcript"],
    &["path", "--occasion", "x", "--stage", "brief"],
    &["review", "x"],
    &["review", "x", "--resolve", "f", "--dismiss"],
    &["confirm", "x", "--tag", "t"],
    &["tag", "rename", "old", "new"],
    &["tag", "rename", "old", "new", "--dry-run"],
    &["transcribe", "x"],
    &["synthesize", "x"],
    &["note", "write", "x"],
    &["verify-note", "/absent.md", "--recording", "x"],
    &["handoff", "x", "--stage", "transcript"],
    &["redact", "x"],
    &["brief", "x"],
    &["import", "/absent.mp3"],
    &["brief", "--upcoming"],
    &["ingest", "--dry-run"],
    &["run", "--dry-run"],
    &["retention", "run", "--dry-run"],
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

fn decoded(flag: &str, text: &str) -> Value {
    match flag {
        "--json" => serde_json::from_str(text).map_err(|error| error.to_string()),
        "--toon" => toon_format::decode_default(text).map_err(|error| error.to_string()),
        _ => serde_yaml_ng::from_str(text).map_err(|error| error.to_string()),
    }
    .unwrap_or_else(|error| panic!("{flag} did not decode: {error}\n{text}"))
}

fn document(flag: &str, output: &Output) -> (Option<i32>, bool, Value) {
    let on_stdout = !output.stdout.is_empty();
    let text = if on_stdout {
        stdout(output)
    } else {
        stderr(output)
    };
    assert!(text.ends_with('\n'), "{flag} ends with a newline: {text:?}");
    (output.status.code(), on_stdout, decoded(flag, &text))
}

#[test]
fn every_verb_prints_the_same_document_in_each_format() {
    let sandbox = configured("formats-every-verb");
    for args in EVERY_VERB {
        let [json, toon, yaml] = ["--json", "--toon", "--yaml"]
            .map(|flag| document(flag, &run(sandbox.vpt().args(*args).arg(flag))));
        assert_eq!(json, toon, "{args:?} as TOON");
        assert_eq!(json, yaml, "{args:?} as YAML");
    }
}

#[test]
fn a_usage_error_is_one_document_in_the_chosen_format() {
    let sandbox = Sandbox::new("formats-usage-error");
    for flag in ["--json", "--toon", "--yaml"] {
        let (code, on_stdout, error) = document(flag, &run(sandbox.vpt().args([flag, "show"])));
        assert_eq!((code, on_stdout), (Some(2), false), "{flag}");
        assert_eq!(error["schema"], "vpt.error/1", "{flag}");
        assert_eq!(error["error"]["kind"], "usage", "{flag}");
        assert_eq!(error["error"]["message"], "<id> is required", "{flag}");
    }
}

#[test]
fn two_format_flags_are_refused_in_plain_text_with_the_usage() {
    let sandbox = configured("formats-two-flags");
    for pair in [
        ["--json", "--toon"],
        ["--toon", "--yaml"],
        ["--yaml", "--json"],
    ] {
        let output = run(sandbox.vpt().arg("list").args(pair));
        assert_eq!(output.status.code(), Some(2), "{pair:?}");
        assert_eq!(stdout(&output), "", "{pair:?}");
        let text = stderr(&output);
        assert!(
            text.starts_with(&format!("vpt: {} conflicts with {}\n", pair[1], pair[0])),
            "{text}"
        );
        assert!(text.contains("usage: vpt [--json|--toon|--yaml]"), "{text}");
    }
}

#[test]
fn toon_and_yaml_are_not_json() {
    let sandbox = configured("formats-not-json");
    for flag in ["--toon", "--yaml"] {
        let text = stdout(&run(sandbox.vpt().args(["list", flag])));
        assert!(!text.starts_with('{'), "{flag}: {text}");
        assert!(text.contains("command: "), "{flag}: {text}");
    }
}
