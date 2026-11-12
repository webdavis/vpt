#![allow(dead_code, reason = "shared command fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "configuration/privacy.rs"]
mod privacy;
mod support;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};
use support::{Sandbox, stderr};
use vpt_application::ports::StageStates;
use vpt_domain::{
    digest::Sha256Digest,
    occasion::{OccasionId, time::OccasionTime},
};

fn commands(sandbox: &Sandbox) -> Vec<Vec<String>> {
    let id = fixture::record(sandbox, 1, StageStates::fresh())
        .id
        .to_string();
    let at = "2026-10-01T10:00:00-06:00";
    let occasion =
        OccasionId::derive(&OccasionTime::parse(at).unwrap(), &Sha256Digest([1; 32])).to_string();
    let note = sandbox.path().join("note.md");
    vec![
        vec!["ingest"],
        vec!["run"],
        vec!["transcribe", &id],
        vec!["review", &id],
        vec!["review", &id, "--resolve", &"0".repeat(64), "--confirm"],
        vec!["confirm", "--term", "Private term"],
        vec!["confirm", &id, "--tag", "planning"],
        vec!["confirm", &id, "--reject-tag", "planning"],
        vec!["confirm", &id, "--relation", &format!("related:{id}")],
        vec!["note", "write", &id],
        vec!["analyze", &id],
        vec!["verify-note", note.to_str().unwrap(), "--recording", &id],
        vec!["occasions"],
        vec!["path", &id, "--stage", "transcript"],
        vec!["path", &id, "--stage", "analysis"],
        vec!["path", "--occasion", &occasion, "--stage", "brief"],
        vec!["brief", &occasion],
        vec!["brief", "--title", "Private planning", "--at", at],
        vec!["brief", "--upcoming"],
        vec!["redact", &id, "--stage", "transcript"],
        vec!["redact", &id, "--stage", "analysis"],
        vec!["show", &id, "--stage", "released"],
        vec!["show", &id, "--stage", "transcript"],
        vec!["show", &id, "--stage", "transcript", "--redact"],
        vec!["show", &id, "--stage", "analysis"],
        vec!["show", &occasion, "--stage", "brief"],
        vec!["retention", "run"],
    ]
    .into_iter()
    .map(|args| args.into_iter().map(str::to_owned).collect())
    .collect()
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn refusal(command: &mut std::process::Command, key: &str, structured: bool) {
    let output = support::run_with_budget(command, Duration::from_secs(5));
    assert_eq!(
        output.status.code(),
        Some(2),
        "{command:?}: {}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains(key),
        "{command:?}: {}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("Private configuration value"));
    if structured {
        let document: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(document["schema"], "vpt.error/1");
        assert_eq!(document["error"]["kind"], "config");
        assert_eq!(document["error"]["completed"], serde_json::json!([]));
    }
}

#[test]
fn configuration_precedence_reaches_every_new_command_without_mutation() {
    let sandbox = fixture::configured("configuration-precedence");
    let parent = sandbox.config_path().parent().unwrap().to_path_buf();
    let environment = parent.join("environment.toml");
    let explicit = parent.join("explicit.toml");
    for (path, key) in [
        (&sandbox.config_path(), "default_unknown"),
        (&environment, "environment_unknown"),
        (&explicit, "explicit_unknown"),
    ] {
        write(
            path,
            &format!("config_version=1\n{key}='Private configuration value'\n"),
        );
    }
    let before = fixture::snapshot(&sandbox);
    for args in commands(&sandbox) {
        for structured in [false, true] {
            for selected in 0..3 {
                let mut command = sandbox.vpt();
                command.env_remove("VPT_CONFIG");
                if structured {
                    command.arg("--json");
                }
                command.args(&args);
                let key = match selected {
                    0 => "default_unknown",
                    1 => {
                        command.env("VPT_CONFIG", &environment);
                        "environment_unknown"
                    }
                    _ => {
                        command
                            .env("VPT_CONFIG", &environment)
                            .arg("--config")
                            .arg(&explicit);
                        "explicit_unknown"
                    }
                };
                refusal(&mut command, key, structured);
                assert_eq!(fixture::snapshot(&sandbox), before, "{args:?}");
            }
        }
    }
}

#[test]
fn configuration_versions_and_invalid_values_refuse_before_new_command_work() {
    let sandbox = fixture::configured("configuration-refusals");
    for (text, key) in [
        ("[context]\ntype='none'", "config_version"),
        ("config_version=2", "version 2"),
        (
            "config_version=1\n[context]\ntype='Private configuration value'",
            "context.type",
        ),
        (
            "config_version=1\n[brief]\nmax_notes=4294967296",
            "brief.max_notes",
        ),
    ] {
        write(&sandbox.config_path(), text);
        let before = fixture::snapshot(&sandbox);
        for args in commands(&sandbox) {
            refusal(sandbox.vpt().args(&args).arg("--json"), key, true);
            assert_eq!(fixture::snapshot(&sandbox), before, "{args:?}");
        }
    }
}
