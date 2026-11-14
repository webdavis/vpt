use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;
use vpt_application::engines::{EngineConfig, EngineKind};

fn helper(directory: &Path, version: &str, body: &str) -> AppleEngine {
    let path = directory.join("helper with spaces");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = --version ]; then\n  printf '%s' '{{\"schema\":\"vpt.helper/1\",\"version\":\"{version}\"}}'\n  exit 0\nfi\n{body}\n"
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    AppleEngine::new(path)
}

fn config(name: &str) -> EngineConfig {
    EngineConfig {
        name: name.into(),
        kind: EngineKind::Apple,
    }
}

fn request(engine: &EngineConfig) -> EngineRequest<'_> {
    EngineRequest {
        engine,
        audio: Path::new("/audio with spaces.m4a"),
        out_dir: Path::new("/unused"),
        language: "fr-CA",
        duration: 2.5,
        timeout: Duration::from_secs(3),
    }
}

const VALID: &str = r#"
[ "$1" = transcribe ] && [ "$2" = '/audio with spaces.m4a' ] && [ "$3" = --locale ] && [ "$4" = fr-CA ] || exit 7
printf '%s' '{"schema":"vpt.engine/1","engine":{"name":"apple","tool":"vpt-macos","version":"1.0.0","model":"SpeechTranscriber","family":"apple"},"language":"fr-CA","local":true,"segments":[{"start":0,"end":2.5,"text":"private-canary","words":[{"text":"private-canary","start":0,"end":2.5,"confidence":0.9}]}]}'
"#;

#[test]
fn calls_version_then_transcribe_with_exact_audio_and_regional_locale() {
    let directory = crate::test_temp::tempdir().unwrap();
    let adapter = helper(directory.path(), "1.0.0", VALID);
    let output = adapter.transcribe(&request(&config("apple"))).unwrap();
    assert_eq!(output.transcript.engine.name, "apple");
    assert_eq!(output.transcript.language, "fr-CA");
    assert_eq!(output.transcript.segments[0].words[0].confidence, Some(0.9));
}

#[test]
fn an_apple_alias_is_named_consistently_in_domain_and_saved_document() {
    let directory = crate::test_temp::tempdir().unwrap();
    let adapter = helper(directory.path(), "1.0.0", VALID);
    let output = adapter
        .transcribe(&request(&config("local-apple")))
        .unwrap();
    assert_eq!(output.transcript.engine.name, "local-apple");
    let raw: serde_json::Value = serde_json::from_slice(&output.raw_document).unwrap();
    assert_eq!(raw["engine"]["name"], "local-apple");
}

#[test]
fn mismatched_helper_major_is_refused_before_speech_is_started() {
    let directory = crate::test_temp::tempdir().unwrap();
    let adapter = helper(directory.path(), "2.0.0", "exit 9");
    assert_eq!(
        adapter.transcribe(&request(&config("apple"))).unwrap_err(),
        EngineFailure::HelperVersion(2)
    );
}

#[test]
fn helper_availability_messages_are_mapped_without_returning_child_text() {
    for (message, expected) in [
        (
            "Apple speech requires macOS 26 or later",
            EngineFailure::UnsupportedPlatform,
        ),
        (
            "Apple SpeechTranscriber is unavailable on this Mac",
            EngineFailure::Unavailable,
        ),
        (
            "Apple speech does not support the requested locale",
            EngineFailure::UnsupportedLanguage,
        ),
        (
            "Apple speech assets are not installed for the requested locale",
            EngineFailure::MissingSpeechAssets,
        ),
        ("private-canary", EngineFailure::Exited(1)),
    ] {
        let directory = crate::test_temp::tempdir().unwrap();
        let adapter = helper(
            directory.path(),
            "1.0.0",
            &format!("printf '%s\\n' 'vpt-macos: {message}' >&2\nexit 1"),
        );
        assert_eq!(
            adapter.transcribe(&request(&config("apple"))).unwrap_err(),
            expected
        );
    }
}
