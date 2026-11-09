use super::*;

#[test]
fn a_complete_proposal_at_the_output_limit_is_accepted_but_overflow_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("proposal");
    let mut bytes = document().into_bytes();
    bytes.resize(1_048_576, b' ');
    fs::write(&path, &bytes).unwrap();
    let config = settings(&[
        "/bin/sh",
        "-c",
        "cat >/dev/null; cat \"$1\"",
        "fixture",
        path.to_str().unwrap(),
    ]);
    let id = id();
    assert!(CommandAgent.synthesize(&request(&config, &id)).is_ok());
    bytes.push(b' ');
    fs::write(path, bytes).unwrap();
    assert_eq!(
        CommandAgent.synthesize(&request(&config, &id)).unwrap_err(),
        SynthesisFailure::OutputLimit
    );
}

#[test]
fn invalid_documents_and_versions_are_typed_without_exposing_child_output() {
    let id = id();
    for (doc, expected) in [
        ("Private invalid".into(), SynthesisFailure::InvalidDocument),
        (
            document().replace("vpt.proposal/1", "vpt.proposal/2"),
            SynthesisFailure::UnsupportedVersion(2),
        ),
    ] {
        let config = settings(&[
            "/bin/sh",
            "-c",
            "cat >/dev/null; printf '%s' \"$1\"",
            "fixture",
            &doc,
        ]);
        assert_eq!(
            CommandAgent.synthesize(&request(&config, &id)).unwrap_err(),
            expected
        );
    }
}

#[test]
fn disabled_invalid_and_oversized_requests_never_start_the_command() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("must-not-exist");
    let id = id();
    assert_eq!(
        CommandAgent
            .synthesize(&request(&SynthesisSettings::default(), &id))
            .unwrap_err(),
        SynthesisFailure::Disabled
    );
    let config = settings(&["/usr/bin/touch", marker.to_str().unwrap()]);
    let mut bad = request(&config, &id);
    bad.language = "Private\0language";
    assert_eq!(
        CommandAgent.synthesize(&bad).unwrap_err(),
        SynthesisFailure::InvalidArguments
    );
    bad = request(&config, &id);
    bad.transcript = Path::new("relative.md");
    assert_eq!(
        CommandAgent.synthesize(&bad).unwrap_err(),
        SynthesisFailure::InvalidArguments
    );
    let content = "x".repeat(vpt_domain::note::MAX_RENDERED + 1);
    bad = request(&config, &id);
    bad.content = &content;
    assert_eq!(
        CommandAgent.synthesize(&bad).unwrap_err(),
        SynthesisFailure::InputLimit
    );
    let mut oversized = config.clone();
    oversized.command.extend(vec![String::new(); 256]);
    assert_eq!(
        CommandAgent
            .synthesize(&request(&oversized, &id))
            .unwrap_err(),
        SynthesisFailure::InvalidArguments
    );
    oversized = config;
    oversized.command.push("{transcript}".repeat(65_536));
    assert_eq!(
        CommandAgent
            .synthesize(&request(&oversized, &id))
            .unwrap_err(),
        SynthesisFailure::InvalidArguments
    );
    assert!(!marker.exists());
}

#[test]
fn the_shared_deadline_covers_blocked_stdin_and_early_input_closure_is_a_failure() {
    let id = id();
    let content = "Private ".repeat(524_288);
    let mut config = settings(&["/bin/sleep", "5"]);
    config.timeout = Duration::from_millis(20);
    let mut input = request(&config, &id);
    input.content = &content;
    assert_eq!(
        CommandAgent.synthesize(&input).unwrap_err(),
        SynthesisFailure::DeadlineExceeded
    );
    let doc = document();
    let config = settings(&["/usr/bin/printf", "%s", &doc]);
    let mut input = request(&config, &id);
    input.content = &content;
    assert_eq!(
        CommandAgent.synthesize(&input).unwrap_err(),
        SynthesisFailure::Io
    );
}
