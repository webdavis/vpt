#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
mod support;

use fixture::write_config;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, UNIX_EPOCH};
use support::{Sandbox, stderr, stdout};
use vpt_domain::fixtures::m4a;

const MPEG4_ZERO: i64 = -2_082_844_800;

fn run(command: &mut std::process::Command) -> Output {
    support::run_with_budget(command, Duration::from_secs(3))
}

fn configured(extra: &str) -> Sandbox {
    let sandbox = Sandbox::new("import-command");
    write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\nquiet_period_secs = 3600\n{extra}",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    sandbox
}

fn file(sandbox: &Sandbox, name: &str, bytes: &[u8], modified: Option<i64>) -> PathBuf {
    let path = sandbox.path().join("downloads").join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    if let Some(secs) = modified {
        std::fs::File::open(&path)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(secs.try_into().unwrap()))
            .unwrap();
    }
    path
}

fn fake_extractor(sandbox: &Sandbox, output: &[u8]) -> PathBuf {
    let produced = sandbox.path().join("bin/produced.m4a");
    std::fs::write(&produced, output).unwrap();
    let script = sandbox.path().join("bin/fake-ffmpeg");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> {:?}\nfor last; do :; done\n/bin/cp {produced:?} \"$last\"\n",
            sandbox.path().join("extract.log")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    script
}

fn import(sandbox: &Sandbox, args: &[&str]) -> Output {
    run(sandbox.vpt().arg("--json").arg("import").args(args))
}

fn json(output: &Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    serde_json::from_str(&stdout(output)).unwrap()
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn m4a_is_archived_as_is_with_stem_title_and_container_capture_time() {
    let sandbox = configured("");
    let source = file(
        &sandbox,
        "Team meeting.m4a",
        &m4a(1_787_604_456, 12, b"x"),
        None,
    );
    let doc = json(&import(&sandbox, &[path(&source)]));
    assert_eq!(doc["command"], "import");
    let record = &doc["ingested"][0];
    assert!(record["id"].as_str().unwrap().len() > 12);
    assert_eq!(record["title"], "Team meeting");
    assert_eq!(record["title_source"], "import");
    assert_eq!(record["source_path"], path(&source));
    assert_eq!(record["captured_at_utc"], "2026-08-24T20:47:36Z");
    let audio = record["audio_path"].as_str().unwrap();
    assert_eq!(
        std::fs::read(audio).unwrap(),
        std::fs::read(&source).unwrap()
    );
}

#[test]
fn title_and_capture_time_flags_override_the_file() {
    let sandbox = configured("");
    let source = file(&sandbox, "a.m4a", &m4a(1_787_604_456, 12, b"x"), None);
    let doc = json(&import(
        &sandbox,
        &[
            path(&source),
            "--title",
            "Planning",
            "--captured-at",
            "2026-01-02T03:04:05Z",
        ],
    ));
    assert_eq!(doc["ingested"][0]["title"], "Planning");
    assert_eq!(
        doc["ingested"][0]["captured_at_utc"],
        "2026-01-02T03:04:05Z"
    );
}

#[test]
fn zero_container_creation_time_falls_back_to_modification_time() {
    let sandbox = configured("");
    let source = file(
        &sandbox,
        "a.m4a",
        &m4a(MPEG4_ZERO, 12, b"x"),
        Some(1_780_000_000),
    );
    let doc = json(&import(&sandbox, &[path(&source)]));
    assert_eq!(
        doc["ingested"][0]["captured_at_utc"],
        "2026-05-28T20:26:40Z"
    );
}

#[test]
fn other_files_are_converted_and_importing_twice_keeps_one_recording() {
    let sandbox = configured("");
    let script = fake_extractor(&sandbox, &m4a(MPEG4_ZERO, 5, b"converted"));
    let mut config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    config.push_str(&format!("[import]\nextract_command = [{script:?}]\n"));
    std::fs::write(sandbox.config_path(), config).unwrap();
    let source = file(&sandbox, "call.wav", b"RIFF wave", Some(1_780_000_000));
    let first = json(&import(&sandbox, &[path(&source)]));
    let record = &first["ingested"][0];
    assert_eq!(record["source_path"], path(&source));
    assert_eq!(record["title"], "call");
    assert_eq!(record["captured_at_utc"], "2026-05-28T20:26:40Z");
    let log = std::fs::read_to_string(sandbox.path().join("extract.log")).unwrap();
    assert!(log.contains(&format!("-i\n{}\n", path(&source))), "{log}");
    let second = json(&import(&sandbox, &[path(&source)]));
    assert_eq!(second["ingested"], serde_json::json!([]));
    assert_eq!(second["already_ingested"][0], record["id"]);
}

#[test]
fn conversion_succeeds_when_the_temp_directory_sits_behind_a_symlink() {
    let sandbox = configured("");
    let script = fake_extractor(&sandbox, &m4a(MPEG4_ZERO, 5, b"converted"));
    let mut config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    config.push_str(&format!("[import]\nextract_command = [{script:?}]\n"));
    std::fs::write(sandbox.config_path(), config).unwrap();
    let linked_temp = sandbox.path().join("linked-tmp");
    std::os::unix::fs::symlink(sandbox.path().join("tmp"), &linked_temp).unwrap();
    let source = file(&sandbox, "call.wav", b"RIFF wave", None);
    let output =
        run(sandbox
            .vpt()
            .env("TMPDIR", &linked_temp)
            .args(["--json", "import", path(&source)]));
    assert_eq!(json(&output)["ingested"][0]["title"], "call");
}

#[test]
fn a_missing_or_failing_extractor_is_refused() {
    let sandbox = configured("");
    let source = file(&sandbox, "call.wav", b"RIFF wave", None);
    let missing = import(&sandbox, &[path(&source)]);
    assert_eq!(missing.status.code(), Some(3), "{}", stderr(&missing));
    assert!(stderr(&missing).contains("extract"), "{}", stderr(&missing));
    let mut config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    config.push_str("[import]\nextract_command = [\"/usr/bin/false\"]\n");
    std::fs::write(sandbox.config_path(), config).unwrap();
    let failing = import(&sandbox, &[path(&source)]);
    assert_eq!(failing.status.code(), Some(3), "{}", stderr(&failing));
    assert!(stderr(&failing).contains("extract"), "{}", stderr(&failing));
}

#[test]
fn a_fresh_file_skips_the_quiet_period_but_a_deferral_fails() {
    let sandbox = configured("");
    let fresh = file(&sandbox, "fresh.m4a", &m4a(1_787_604_456, 12, b"x"), None);
    assert_eq!(
        json(&import(&sandbox, &[path(&fresh)]))["ingested"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let broken = file(&sandbox, "broken.m4a", b"not a container", None);
    let output = import(&sandbox, &[path(&broken)]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("invalid_container"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_file_is_a_usage_error_and_text_output_names_the_id() {
    let sandbox = configured("");
    let output = import(&sandbox, &[path(&sandbox.path().join("absent.m4a"))]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let source = file(&sandbox, "a.m4a", &m4a(1_787_604_456, 12, b"x"), None);
    let text = run(sandbox.vpt().args(["import", path(&source)]));
    assert_eq!(text.status.code(), Some(0), "{}", stderr(&text));
    assert!(
        stdout(&text).starts_with("Ingested 1 recording(s)"),
        "{}",
        stdout(&text)
    );
}
