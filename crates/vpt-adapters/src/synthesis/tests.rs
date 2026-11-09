use super::*;
use std::{fs, path::Path, time::Duration};
use vpt_application::SynthesisSettings;
use vpt_domain::identity::RecordingId;
mod boundaries;

fn id() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}
fn settings(command: &[&str]) -> SynthesisSettings {
    SynthesisSettings {
        command: command.iter().map(|s| (*s).into()).collect(),
        timeout: Duration::from_secs(3),
        ..Default::default()
    }
}
fn request<'a>(
    settings: &'a SynthesisSettings,
    recording: &'a RecordingId,
) -> SynthesisRequest<'a> {
    SynthesisRequest {
        settings,
        recording,
        transcript: Path::new("/private/with spaces {language} $(Private).md"),
        language: "fr-CA",
        content: "  Private exact content\r\n[00:01] punctuation!\n",
    }
}
fn document() -> String {
    serde_json::json!({"schema":"vpt.proposal/1", "by":"Private agent", "summary":[{"text":"Private claim", "sources":["00:01-00:02"]}], "actions":[], "tags":[], "relations":[], "future":"Private extra"}).to_string()
}

#[test]
fn passes_exact_content_on_stdin_and_literal_arguments_then_returns_a_bound_domain_proposal() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("stdin");
    let args = directory.path().join("arguments");
    let doc = document();
    let settings = settings(&[
        "/bin/sh",
        "-c",
        "cat > \"$1\"; printf '%s' \"$2\"; printf '%s\\n' \"$3\" \"$4\" \"$5\" > \"$6\"; printf Private-stderr >&2",
        "fixture",
        input.to_str().unwrap(),
        &doc,
        "{transcript}",
        "{id}",
        "{language}",
        args.to_str().unwrap(),
    ]);
    let id = id();
    let request = request(&settings, &id);
    let result = CommandAgent.synthesize(&request).unwrap();
    assert_eq!(fs::read(&input).unwrap(), request.content.as_bytes());
    assert_eq!(
        fs::read_to_string(args).unwrap(),
        format!(
            "{}\n{}\n{}\n",
            request.transcript.display(),
            id,
            request.language
        )
    );
    assert_eq!(
        result.input_digest,
        crate::digest_open(&mut fs::File::open(input).unwrap()).unwrap()
    );
    assert_eq!(result.proposal.summary[0].text, "Private claim");
    assert_eq!(
        result.diagnostics,
        ["unknown additive proposal field: future"]
    );
    assert!(!format!("{result:?}").contains("Private"));
}

#[test]
fn command_failure_missing_executable_and_timeout_are_private_typed_errors() {
    let id = id();
    for (command, expected) in [
        (
            vec!["/bin/sh", "-c", "cat >/dev/null; echo Private >&2; exit 17"],
            SynthesisFailure::Exited(17),
        ),
        (
            vec!["/Private-no-such-command"],
            SynthesisFailure::Unavailable,
        ),
    ] {
        let config = settings(&command);
        let error = CommandAgent.synthesize(&request(&config, &id)).unwrap_err();
        assert_eq!(error, expected);
        assert!(!format!("{error:?}").contains("Private"));
    }
    let mut config = settings(&["/bin/sleep", "5"]);
    config.timeout = Duration::from_millis(20);
    assert_eq!(
        CommandAgent.synthesize(&request(&config, &id)).unwrap_err(),
        SynthesisFailure::DeadlineExceeded
    );
}
