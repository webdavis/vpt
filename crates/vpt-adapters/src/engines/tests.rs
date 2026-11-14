use super::*;
use std::path::Path;
use std::time::Duration;
use vpt_application::engines::{EngineConfig, EngineKind};

fn config(command: Vec<String>) -> EngineConfig {
    EngineConfig {
        name: "custom".into(),
        kind: EngineKind::Command {
            command,
            family: "custom-family".into(),
            local: false,
        },
    }
}

fn request(engine: &EngineConfig) -> EngineRequest<'_> {
    EngineRequest {
        engine,
        audio: Path::new("/private/audio {language}.m4a"),
        out_dir: Path::new("/private/output {audio}"),
        language: "en-US",
        duration: 2.5,
        timeout: Duration::from_secs(3),
    }
}

fn document() -> String {
    serde_json::json!({ "schema": "vpt.engine/1",
        "engine": {"name":"custom", "family":"custom-family", "tool":"fixture", "version":"1", "model":"fixture"},
        "language":"en-US", "local":false,
        "segments":[{"start":0.0,"end":2.5,"text":"private-canary", "words":[
            {"start":0.0,"end":2.5,"text":"private-canary"}]}]
    }).to_string()
}

#[test]
fn accepts_a_valid_command_document_without_fabricating_confidence() {
    let document = document();
    let engine = config(vec![
        "/usr/bin/printf".into(),
        "%s".into(),
        document.clone(),
    ]);
    let output = CommandEngine.transcribe(&request(&engine)).unwrap();
    assert_eq!(output.raw_document, document.as_bytes());
    assert_eq!(output.transcript.language, "en-US");
    assert!(!output.transcript.local);
    assert_eq!(
        output.transcript.segments[0].words[0].text,
        "private-canary"
    );
    assert_eq!(output.transcript.segments[0].words[0].end, 2.5);
    assert_eq!(output.transcript.segments[0].words[0].confidence, None);
    assert!(!format!("{output:?}").contains("private-canary"));
    assert!(!format!("{:?}", output.transcript).contains("private-canary"));
}

#[test]
fn placeholders_are_single_pass_and_preserve_argument_boundaries() {
    let directory = crate::test_temp::tempdir().unwrap();
    let audio = directory
        .path()
        .join("with spaces {language} {out_dir}.m4a");
    std::fs::write(&audio, document()).unwrap();
    let engine = config(vec!["/bin/cat".into(), "{audio}".into()]);
    let mut request = request(&engine);
    request.audio = &audio;
    assert!(CommandEngine.transcribe(&request).is_ok());
}

#[test]
fn rejects_invalid_documents_and_unknown_versions() {
    for (text, expected) in [
        ("private-canary".into(), EngineFailure::InvalidDocument),
        (
            document().replace("vpt.engine/1", "vpt.engine/2"),
            EngineFailure::UnsupportedVersion(2),
        ),
        (
            document().replace("custom-family", "other"),
            EngineFailure::InvalidDocument,
        ),
    ] {
        let engine = config(vec!["/usr/bin/printf".into(), "%s".into(), text]);
        assert_eq!(
            CommandEngine.transcribe(&request(&engine)).unwrap_err(),
            expected
        );
    }
}

#[test]
fn command_status_and_deadline_are_typed_failures() {
    let engine = config(vec!["/usr/bin/false".into()]);
    assert_eq!(
        CommandEngine.transcribe(&request(&engine)).unwrap_err(),
        EngineFailure::Exited(1)
    );
    let engine = config(vec!["/bin/sleep".into(), "5".into()]);
    let mut request = request(&engine);
    request.timeout = Duration::from_millis(20);
    assert_eq!(
        CommandEngine.transcribe(&request).unwrap_err(),
        EngineFailure::DeadlineExceeded
    );
}

#[test]
fn output_past_the_limit_is_refused_even_after_a_valid_document() {
    let directory = crate::test_temp::tempdir().unwrap();
    let audio = directory.path().join("oversized");
    let mut content = document().into_bytes();
    content.resize(16_777_217, b' ');
    std::fs::write(&audio, content).unwrap();
    let engine = config(vec!["/bin/cat".into(), "{audio}".into()]);
    let mut request = request(&engine);
    request.audio = &audio;
    assert_eq!(
        CommandEngine.transcribe(&request).unwrap_err(),
        EngineFailure::OutputLimit
    );
}
