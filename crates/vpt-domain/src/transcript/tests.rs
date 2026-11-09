use super::*;

fn transcript(language: &str, source: LanguageSource) -> Transcript {
    Transcript {
        engine: EngineMetadata {
            name: "fixture".into(),
            tool: "fixture".into(),
            version: "1".into(),
            model: "fixture".into(),
            family: "fixture".into(),
        },
        language: language.into(),
        language_source: source,
        local: true,
        segments: vec![],
    }
}

#[test]
fn reported_languages_compare_base_codes_when_either_tag_is_base_only() {
    for (reported, requested, mismatch) in [
        ("en", "en-US", false),
        ("en-US", "en", false),
        ("EN-us", "en-US", false),
        ("en-US", "en-GB", true),
        ("fr", "en-US", true),
        ("fr-CA", "fr-FR", true),
    ] {
        assert_eq!(
            transcript(reported, LanguageSource::Reported).language_mismatch(requested),
            mismatch
        );
    }
}

#[test]
fn an_echoed_language_option_is_not_evidence_of_a_mismatch() {
    assert!(!transcript("fr", LanguageSource::Requested).language_mismatch("en-US"));
}
