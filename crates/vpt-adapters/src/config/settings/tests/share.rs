use super::*;
use vpt_application::ShareSettings;
use vpt_domain::redaction::{Class, FlagPolicy};
#[test]
fn all_share_defaults_and_overrides_reach_runtime_settings_privately() {
    assert_eq!(settings("").unwrap().share, ShareSettings::default());
    let s = settings("[share]\nautomatic=true\nsource_line='Private source'\ninclude_date=true\nkeep_timecodes=true\ndeny_sections=['Private section']\n[share.redact]\nclasses=['money','email']\nmask='Private {class}:{n}'\nflagged_spans='mark'").unwrap();
    assert!(s.share.automatic && s.share.include_date && s.share.keep_timecodes);
    assert_eq!(s.share.source_line, "Private source");
    assert_eq!(s.share.deny_sections, ["Private section"]);
    assert_eq!(s.share.redact.classes, [Class::Money, Class::Email]);
    assert_eq!(s.share.redact.mask, "Private {class}:{n}");
    assert_eq!(s.share.flagged_spans, FlagPolicy::Mark);
    assert!(!format!("{s:?}").contains("Private"));
    let empty = settings("[share]\nsource_line=''\n[share.redact]\nclasses=[]").unwrap();
    assert!(empty.share.source_line.is_empty());
    assert!(empty.share.redact.classes.is_empty());
}
#[test]
fn unknown_classes_and_invalid_masks_refuse_with_the_key_without_the_value() {
    for (key, value) in [
        ("classes", "['Private unknown']"),
        ("classes", "['term']"),
        ("classes", "['note']"),
        ("mask", "'Private missing number'"),
        ("mask", "'{n}{n}'"),
        ("flagged_spans", "'Private unknown'"),
    ] {
        let error = load_text(&format!("config_version=1\n[share.redact]\n{key}={value}"))
            .and_then(|t| from_table(&t, Path::new("/u")))
            .unwrap_err();
        assert_eq!(error.key(), Some(format!("share.redact.{key}").as_str()));
        assert!(!format!("{error:?}").contains("Private"));
    }
}
#[test]
fn share_text_and_collection_limits_fail_during_configuration_loading() {
    for (key, config) in [
        (
            "share.source_line",
            format!("[share]\nsource_line='{}'", "x".repeat(65_537)),
        ),
        (
            "share.source_line",
            "[share]\nsource_line=\"Private\\u0000\"".into(),
        ),
        (
            "share.deny_sections",
            format!(
                "[share]\ndeny_sections=[{}]",
                vec!["'Private'"; 257].join(",")
            ),
        ),
        (
            "share.deny_sections",
            format!(
                "[share]\ndeny_sections=['{}','{}']",
                "x".repeat(32_769),
                "x".repeat(32_769)
            ),
        ),
        (
            "share.redact.classes",
            format!("[share.redact]\nclasses=[{}]", ["'email'"; 8].join(",")),
        ),
    ] {
        let error = load_text(&format!("config_version=1\n{config}"))
            .and_then(|t| from_table(&t, Path::new("/u")))
            .unwrap_err();
        assert_eq!(error.key(), Some(key));
        assert!(!format!("{error:?}").contains("Private"));
    }
}
