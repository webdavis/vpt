use super::*;

#[test]
fn wiki_links_use_exact_root_relative_paths_and_keep_readable_labels() {
    for (target, label, expected) in [
        ("/vault/analysis/Call.md", "Call", "[[analysis/Call|Call]]"),
        ("/vault/Call.md", "Call", "[[Call]]"),
        (
            "/vault/Audio/Call.m4a",
            "Call.m4a",
            "[[Audio/Call.m4a|Call.m4a]]",
        ),
        (
            "/vault/Équipe/Call one.md",
            "Call one",
            "[[Équipe/Call one|Call one]]",
        ),
    ] {
        assert_eq!(
            reference(
                LinkStyle::Wiki,
                label,
                Path::new(target),
                Path::new("/vault/transcripts"),
                Some(Path::new("/vault"))
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn outside_root_and_reserved_wiki_characters_use_encoded_markdown_destinations() {
    for (target, expected) in [
        ("/elsewhere/Call.md", "[Call](../../elsewhere/Call.md)"),
        ("/vault-other/Call.md", "[Call](../../vault-other/Call.md)"),
        (
            "/vault/analysis/Call#1.md",
            "[Call](../analysis/Call%231.md)",
        ),
        (
            "/vault/analysis/Call|1.md",
            "[Call](../analysis/Call%7C1.md)",
        ),
        ("/vault/analysis/A&B.md", "[Call](../analysis/A%26B.md)"),
    ] {
        assert_eq!(
            reference(
                LinkStyle::Wiki,
                "Call",
                Path::new(target),
                Path::new("/vault/transcripts"),
                Some(Path::new("/vault"))
            )
            .unwrap(),
            expected
        );
    }
    assert_eq!(
        reference(
            LinkStyle::Markdown,
            "Call",
            Path::new("/vault/analysis/Call one.md"),
            Path::new("/vault/transcripts"),
            Some(Path::new("/vault"))
        )
        .unwrap(),
        "[Call](../analysis/Call%20one.md)"
    );
}

#[test]
fn link_targets_are_bounded_and_cannot_inject_wiki_syntax() {
    for path in ["/vault/Bad]]\n[[Injected.md", "/vault/Bad\u{2028}name.md"] {
        let rendered = reference(
            LinkStyle::Wiki,
            "Display ] | label",
            Path::new(path),
            Path::new("/vault"),
            Some(Path::new("/vault")),
        )
        .unwrap();
        assert!(!rendered.contains("[["));
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{2028}'));
        assert!(rendered.starts_with("[Display &#93; &#124; label]("));
    }
    let oversized = format!("/{}", "a".repeat(65_536));
    for (target, origin) in [
        (oversized.as_str(), "/vault"),
        ("/vault/Call.md", oversized.as_str()),
    ] {
        assert!(matches!(
            reference(
                LinkStyle::Wiki,
                "Call",
                Path::new(target),
                Path::new(origin),
                Some(Path::new("/vault"))
            ),
            Err(NoteWriteError::Render(RenderError::CapacityExceeded))
        ));
    }
    use std::os::unix::ffi::OsStringExt;
    let invalid = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xff]));
    assert!(matches!(
        reference(
            LinkStyle::Wiki,
            "Call",
            &invalid,
            Path::new("/vault"),
            Some(Path::new("/vault"))
        ),
        Err(NoteWriteError::Render(RenderError::InvalidName))
    ));
}
