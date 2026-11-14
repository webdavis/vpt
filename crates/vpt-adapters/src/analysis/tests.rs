use super::*;
use std::{fs, path::Path, time::Duration};
use vpt_application::{
    AnalysisSettings,
    tag_hints::{KnownTag, RefusedTag, TagHints},
};
use vpt_domain::identity::RecordingId;
mod boundaries;

static NO_TAGS: TagHints = TagHints {
    known: Vec::new(),
    refused: Vec::new(),
};

fn id() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}
fn settings(command: &[&str]) -> AnalysisSettings {
    AnalysisSettings {
        command: command.iter().map(|s| (*s).into()).collect(),
        timeout: Duration::from_secs(3),
        ..Default::default()
    }
}
fn request<'a>(settings: &'a AnalysisSettings, recording: &'a RecordingId) -> AnalysisRequest<'a> {
    AnalysisRequest {
        settings,
        recording,
        transcript: Path::new("/private/with spaces {language} $(Private).md"),
        language: "fr-CA",
        captured: (
            vpt_domain::time::UtcInstant {
                secs: 1_787_000_000,
            },
            vpt_domain::time::UtcOffset { secs: -21_600 },
        ),
        content: "  Private exact content\r\n[00:01] punctuation!\n",
        tags: &NO_TAGS,
    }
}
fn document() -> String {
    serde_json::json!({"schema":"vpt.proposal/1", "by":"Private agent", "summary":[{"text":"Private claim", "sources":["00:01-00:02"]}], "actions":[], "tags":[], "relations":[], "future":"Private extra"}).to_string()
}

#[test]
fn passes_exact_content_on_stdin_and_literal_arguments_then_returns_a_bound_domain_proposal() {
    let directory = crate::test_temp::tempdir().unwrap();
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
    let result = CommandAgent.analyze(&request).unwrap();
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
            AnalysisFailure::Exited(17),
        ),
        (
            vec!["/Private-no-such-command"],
            AnalysisFailure::Unavailable,
        ),
    ] {
        let config = settings(&command);
        let error = CommandAgent.analyze(&request(&config, &id)).unwrap_err();
        assert_eq!(error, expected);
        assert!(!format!("{error:?}").contains("Private"));
    }
    let mut config = settings(&["/bin/sleep", "5"]);
    config.timeout = Duration::from_millis(20);
    assert_eq!(
        CommandAgent.analyze(&request(&config, &id)).unwrap_err(),
        AnalysisFailure::DeadlineExceeded
    );
}

#[test]
fn tag_lists_are_cut_at_whole_entries_to_keep_the_command_inside_its_limit() {
    let name = |index: usize| format!("tag-{index:03}-{}", "x".repeat(200));
    let tags = TagHints {
        known: (0..400)
            .map(|index| KnownTag {
                name: name(index),
                notes: 400 - index,
            })
            .collect(),
        refused: vec![
            RefusedTag {
                name: "billing".into(),
                renamed_to: Some("invoice".into()),
            },
            RefusedTag {
                name: "spam".into(),
                renamed_to: None,
            },
        ],
    };
    let id = id();
    for (filler, shown) in [(0, 300), (1_048_576 - 20_000, 30)] {
        let config = settings(&[
            "/bin/sh",
            &"f".repeat(filler),
            "Known: {known_tags}. Refused: {refused_tags}.",
            "{known_tags}",
        ]);
        let argv = arguments::argv(&AnalysisRequest {
            tags: &tags,
            ..request(&config, &id)
        })
        .unwrap();
        assert!(argv.iter().map(String::len).sum::<usize>() <= 1_048_576);
        let known: Vec<_> = argv[3].split(", ").collect();
        assert_eq!(known.len(), shown);
        for (index, entry) in known.iter().enumerate() {
            assert_eq!(*entry, format!("{} ({})", name(index), 400 - index));
        }
        assert!(argv[2].ends_with(". Refused: billing -> invoice, spam."));
    }
}
