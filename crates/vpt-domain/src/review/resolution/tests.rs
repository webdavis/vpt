use super::*;
use crate::uncertainty::{Class, Diagnostic, LexicalFlag};

fn flag() -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            class: Class::AgreedUnverified,
            ranges: std::iter::once(0..1).collect(),
            record_text: "Alice".into(),
            alternative_text: None,
            confidence: None,
        }),
    }
}

#[test]
fn actions_close_flags_and_repeated_actions_preserve_the_first_resolution_time() {
    let at = UtcInstant { secs: 60 };
    for (action, expected) in [
        (ReviewAction::Confirm, Resolution::Confirmed { at }),
        (
            ReviewAction::Correct("Alicia".into()),
            Resolution::Corrected {
                text: "Alicia".into(),
                at,
            },
        ),
        (ReviewAction::Dismiss, Resolution::Dismissed { at }),
    ] {
        let mut flag = flag();
        flag.resolution = flag.resolve(&action, at).unwrap();
        assert_eq!(flag.resolution, expected);
        assert_eq!(
            flag.resolve(&action, UtcInstant { secs: 70 }).unwrap(),
            expected
        );
    }
}

#[test]
fn correction_validates_text_and_cannot_replace_a_whole_recording_diagnostic() {
    for text in [
        "".into(),
        "  ".into(),
        "a\u{7f}b".into(),
        "private\0".into(),
        "x".repeat(65_537),
    ] {
        assert_eq!(
            flag().resolve(&ReviewAction::Correct(text), UtcInstant { secs: 1 }),
            Err(ResolveError::InvalidText)
        );
    }
    let mut flag = flag();
    flag.content = FlagContent::Diagnostic(Diagnostic::SingleEngine);
    assert_eq!(
        flag.resolve(&ReviewAction::Correct("x".into()), UtcInstant { secs: 1 }),
        Err(ResolveError::DiagnosticCorrection)
    );
    assert!(matches!(
        flag.resolve(&ReviewAction::Dismiss, UtcInstant { secs: 1 }),
        Ok(Resolution::Dismissed { .. })
    ));
}

#[test]
fn corrections_preserve_newlines_and_tabs_for_safe_rendering() {
    let text = "Alice\r\n\t# Heading";
    let at = UtcInstant { secs: 1 };
    assert_eq!(
        flag().resolve(&ReviewAction::Correct(text.into()), at),
        Ok(Resolution::Corrected {
            text: text.into(),
            at
        })
    );
}

#[test]
fn hidden_formatting_flags_cannot_be_resolved() {
    let mut flag = flag();
    let FlagContent::Lexical(lexical) = &mut flag.content else {
        unreachable!()
    };
    lexical.class = Class::Formatting;
    assert_eq!(
        flag.resolve(&ReviewAction::Confirm, UtcInstant { secs: 1 }),
        Err(ResolveError::HiddenFlag)
    );
}
