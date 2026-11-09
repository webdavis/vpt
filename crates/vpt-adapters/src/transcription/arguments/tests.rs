use super::*;
use vpt_application::engines::{EngineKind, EngineSettings};

fn build(kind: EngineKind, language: &str) -> Result<Invocation, InvocationError> {
    invocation(
        &EngineConfig {
            name: "test".into(),
            kind,
        },
        Path::new("/helper tool"),
        Path::new("/archive/audio file.m4a"),
        Path::new("/private/output dir"),
        language,
    )
}

#[test]
fn apple_receives_the_regional_tag_and_original_audio_path() {
    let actual = build(EngineKind::Apple, "en-US").unwrap();
    assert_eq!(
        actual.argv,
        [
            "/helper tool",
            "transcribe",
            "/archive/audio file.m4a",
            "--locale",
            "en-US"
        ]
    );
    assert_eq!(actual.requested_language, "en-US");
    assert_eq!(actual.backend_language, "en-US");
}

#[test]
fn whisply_receives_supported_base_codes_with_configured_argv_device_and_model() {
    for (tag, base) in [
        ("en-US", "en"),
        ("en-GB", "en"),
        ("pt-BR", "pt"),
        ("zh-Hant-TW", "zh"),
        ("HAW-US", "haw"),
    ] {
        let kind = EngineKind::Whisply {
            command: vec!["uv".into(), "run".into(), "whisply".into()],
            device: "cpu".into(),
            model: "large-v3".into(),
        };
        let actual = build(kind, tag).unwrap();
        assert_eq!(
            actual.argv,
            [
                "uv",
                "run",
                "whisply",
                "run",
                "-f",
                "/archive/audio file.m4a",
                "-o",
                "/private/output dir",
                "-d",
                "cpu",
                "-m",
                "large-v3",
                "-l",
                base,
                "-e",
                "json"
            ]
        );
        assert_eq!(actual.requested_language, tag);
        assert_eq!(actual.backend_language, base);
    }
}

#[test]
fn whisply_defaults_and_language_refusal_are_applied_before_spawning() {
    let settings = EngineSettings::default();
    let kind = settings.definitions["whisply"].kind.clone();
    let actual = build(kind.clone(), "en-US").unwrap();
    assert!(actual.argv.windows(2).any(|pair| pair == ["-d", "mlx"]));
    assert!(
        actual
            .argv
            .windows(2)
            .any(|pair| pair == ["-m", "large-v3-turbo"])
    );
    assert_eq!(
        build(kind, "zz-ZZ"),
        Err(InvocationError::UnsupportedLanguage)
    );
    let english = EngineKind::Whisply {
        command: vec!["whisply".into()],
        device: "cpu".into(),
        model: "small.en".into(),
    };
    assert_eq!(
        build(english.clone(), "fr-FR"),
        Err(InvocationError::UnsupportedLanguage)
    );
    assert!(build(english, "en-GB").is_ok());
}

#[test]
fn command_substitution_preserves_regional_tags_and_never_reprocesses_inserted_text() {
    let kind = EngineKind::Command {
        command: vec![
            "/my engine".into(),
            "--audio={audio}".into(),
            "{language}".into(),
            "{out_dir}".into(),
        ],
        family: "custom".into(),
        local: false,
    };
    let engine = EngineConfig {
        name: "custom".into(),
        kind,
    };
    let actual = invocation(
        &engine,
        Path::new("/unused"),
        Path::new("/archive/{language} $(secret).m4a"),
        Path::new("/out"),
        "en-GB",
    )
    .unwrap();
    assert_eq!(
        actual.argv,
        [
            "/my engine",
            "--audio=/archive/{language} $(secret).m4a",
            "en-GB",
            "/out"
        ]
    );
    assert_eq!(actual.backend_language, "en-GB");
}

#[test]
fn invalid_inputs_are_refused_with_typed_errors() {
    for tag in [
        "", " en-US", "en_US", "en--US", "en/US", "en-US\n", "--help",
    ] {
        assert_eq!(
            build(EngineKind::Apple, tag),
            Err(InvocationError::InvalidLanguage)
        );
    }
    let empty = EngineKind::Command {
        command: vec![],
        family: "custom".into(),
        local: true,
    };
    assert_eq!(build(empty, "en-US"), Err(InvocationError::EmptyCommand));
    assert_eq!(
        invocation(
            &EngineConfig {
                name: "apple".into(),
                kind: EngineKind::Apple
            },
            Path::new("/helper"),
            Path::new("/audio\0path"),
            Path::new("/out"),
            "en-US"
        ),
        Err(InvocationError::InvalidPath)
    );
}
