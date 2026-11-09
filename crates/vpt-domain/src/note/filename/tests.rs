use super::*;

fn name(title: Option<&str>, maximum: usize) -> Result<String, RenderError> {
    let id = RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    transcript_name(&id, title, maximum)
}

#[test]
fn name_uses_capture_date_title_and_identity_digest_only() {
    assert_eq!(
        name(Some("Invoice call"), 60).unwrap(),
        "2026-08-24-invoice-call-4f3ab19c.md"
    );
    assert_eq!(name(None, 60).unwrap(), "2026-08-24-untitled-4f3ab19c.md");
}

#[test]
fn unicode_compatibility_casefold_and_unsafe_separators_have_one_policy() {
    for (title, slug) in [
        ("Ｆｏｏ Straße Kelvin", "foo-strasse-kelvin"),
        ("../Hello\\World:_#[]*?", "hello-world"),
        ("  Hello---  World  ", "hello-world"),
        ("\0/.. 😺", "untitled"),
        ("cafe\u{301} 世界", "café-世界"),
    ] {
        assert_eq!(
            name(Some(title), 60).unwrap(),
            format!("2026-08-24-{slug}-4f3ab19c.md")
        );
    }
}

#[test]
fn truncation_keeps_whole_words_when_a_hyphen_boundary_is_available() {
    assert_eq!(
        name(Some("first second third"), 10).unwrap(),
        "2026-08-24-first-4f3ab19c.md"
    );
    assert_eq!(
        name(Some("first second third"), 12).unwrap(),
        "2026-08-24-first-second-4f3ab19c.md"
    );
    assert_eq!(
        name(Some("世界您好再见"), 4).unwrap(),
        "2026-08-24-世界您好-4f3ab19c.md"
    );
}

#[test]
fn a_zero_limit_and_unbounded_title_are_refused() {
    assert_eq!(name(Some("title"), 0), Err(RenderError::InvalidName));
    assert!(matches!(
        name(Some(&"x".repeat(262_145)), 60),
        Err(RenderError::CapacityExceeded)
    ));
}
