#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared command fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "installation/runtime.rs"]
mod runtime;
mod support;

use runtime::{command, configured, installed_bin, json};
use support::stderr;

#[test]
#[ignore = "requires VPT_TEST_INSTALL_ROOT containing installed release binaries"]
fn installed_pair_reports_versions_and_missing_helper_has_install_instructions() {
    let sandbox = configured("command");
    let before = fixture::snapshot(&sandbox);
    let absent = json(&support::run(
        command(&sandbox).args(["--version", "--json"]),
    ));
    assert_eq!(absent["version"], env!("CARGO_PKG_VERSION"));
    assert!(absent["helper_version"].is_null());
    let with_helper = std::env::join_paths([sandbox.path().join("bin"), installed_bin()]).unwrap();
    let present = json(&support::run(
        command(&sandbox)
            .env("PATH", &with_helper)
            .args(["--version", "--json"]),
    ));
    assert_eq!(present["helper_version"], "1.0.0");
    let healthy = json(&support::run(
        command(&sandbox)
            .env("PATH", &with_helper)
            .args(["doctor", "--json"]),
    ));
    assert!(
        healthy["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| check["ok"] == true)
    );
    let missing = support::run(command(&sandbox).args(["doctor", "--json"]));
    assert_eq!(missing.status.code(), Some(3));
    assert!(missing.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert_eq!(error["schema"], "vpt.error/1");
    let checks = error["error"]["checks"].as_array().unwrap();
    let helper = checks
        .iter()
        .find(|check| check["name"] == "helper")
        .unwrap();
    assert_eq!(helper["ok"], false);
    for hint in [
        "cargo install --git https://github.com/webdavis/vpt vpt",
        "swift build -c release",
        "copy .build/release/vpt-macos beside vpt",
    ] {
        assert!(helper["detail"].as_str().unwrap().contains(hint));
    }
    assert_eq!(
        checks
            .iter()
            .find(|check| check["name"] == "engines")
            .unwrap()["ok"],
        true
    );
    let human = support::run(command(&sandbox).arg("doctor"));
    assert_eq!(human.status.code(), Some(3));
    assert!(human.stdout.is_empty());
    assert!(stderr(&human).contains("FAIL helper"));
    assert!(stderr(&human).contains("swift build -c release"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
#[ignore = "requires VPT_TEST_INSTALL_ROOT containing installed release binaries"]
fn installed_command_and_whisply_engines_work_without_helper_and_scheduled_runs_finish() {
    for engine in ["command", "whisply"] {
        let sandbox = configured(engine);
        let source = runtime::add_audio(&sandbox);
        let original = std::fs::read(&source).unwrap();
        let before = fixture::snapshot(&sandbox);
        let plan = json(&runtime::invoke(&sandbox, &["run", "--dry-run", "--json"]));
        assert_eq!(plan["would_ingest"].as_array().unwrap().len(), 1);
        assert_eq!(fixture::snapshot(&sandbox), before);
        let first = json(&runtime::invoke(&sandbox, &["run", "--json"]));
        assert_eq!(first["ingested"].as_array().unwrap().len(), 1);
        assert_eq!(first["transcribed"].as_array().unwrap().len(), 1);
        assert_eq!(first["notes"].as_array().unwrap().len(), 1);
        let id = first["transcribed"][0].as_str().unwrap();
        assert_eq!(std::fs::read(source).unwrap(), original);
        let second = json(&runtime::invoke(&sandbox, &["run", "--json"]));
        for field in ["ingested", "transcribed", "notes"] {
            assert_eq!(second[field], serde_json::json!([]));
        }
        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("calls")).unwrap(),
            "called\n"
        );
        let brief = json(&runtime::invoke(
            &sandbox,
            &[
                "brief",
                "--title",
                "Installation check",
                "--at",
                "2026-10-01T10:00:00-06:00",
                "--json",
            ],
        ));
        assert_eq!(brief["schema"], "vpt.brief/1");
        let release = json(&runtime::invoke(&sandbox, &["redact", id, "--json"]));
        assert!(std::path::Path::new(release["written"].as_str().unwrap()).is_file());
        let before = fixture::snapshot(&sandbox);
        let handoff = json(&runtime::invoke(
            &sandbox,
            &["handoff", id, "--stage", "released", "--json"],
        ));
        assert_eq!(handoff["schema"], "vpt.handoff/1");
        assert_eq!(handoff["kind"], "released");
        assert_eq!(fixture::snapshot(&sandbox), before);
        let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        assert!(!log.contains("Private words"));
        assert!(!log.contains("Installation check"));
        assert!(
            json(&support::run(
                command(&sandbox).args(["--version", "--json"])
            ))["helper_version"]
                .is_null()
        );
        runtime::assert_retention_requires_helper(&sandbox);
    }
}
