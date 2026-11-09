use super::*;
use crate::engines::{EngineKind, EnginePair};

#[test]
fn a_language_override_selects_its_pair_and_preserves_the_requested_tag() {
    let mut settings = EngineSettings::default();
    settings.by_language.insert(
        "fr-CA".into(),
        EnginePair {
            main: "whisply".into(),
            checker: Some("apple".into()),
        },
    );
    let selected = settings.select(Some("fr-ca")).unwrap();
    assert_eq!(selected.language, "fr-ca");
    assert_eq!(selected.main.name, "whisply");
    assert_eq!(selected.checker.unwrap().name, "apple");
    let fallback = settings.select(Some("es-MX")).unwrap();
    assert_eq!(fallback.main.name, "apple");
    assert_eq!(fallback.language, "es-MX");
}

#[test]
fn selected_checker_of_the_same_family_is_refused_unless_enabled() {
    let mut settings = EngineSettings::default();
    settings.definitions.insert(
        "other".into(),
        EngineConfig {
            name: "other".into(),
            kind: EngineKind::Command {
                command: vec!["engine".into()],
                family: "apple".into(),
                local: false,
            },
        },
    );
    settings.pair.checker = Some("other".into());
    assert!(
        matches!(settings.select(None), Err(SelectionError::SameFamily { main, checker, family })
        if main == "apple" && checker == "other" && family == "apple")
    );
    settings.allow_same_family = true;
    assert_eq!(
        settings.select(None).unwrap().checker.unwrap().name,
        "other"
    );
}

#[test]
fn missing_selected_definition_and_empty_language_are_typed_errors() {
    let mut settings = EngineSettings::default();
    assert!(matches!(
        settings.select(Some(" ")),
        Err(SelectionError::InvalidLanguage)
    ));
    settings.pair.checker = Some("missing".into());
    assert!(matches!(
        settings.select(None),
        Err(SelectionError::MissingEngine { slot: "checker" })
    ));
}

#[test]
fn engine_debug_does_not_expose_configured_arguments() {
    let engine = EngineConfig {
        name: "custom".into(),
        kind: EngineKind::Command {
            command: vec!["private-canary".into(), "--token=private-canary".into()],
            family: "custom".into(),
            local: false,
        },
    };
    assert!(!format!("{engine:?}").contains("private-canary"));
}
