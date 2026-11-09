#![allow(clippy::single_range_in_vec_init, reason = "Flags store word ranges")]
use super::*;
use crate::{
    review::{FlagContent, Resolution},
    uncertainty::{Class, LexicalFlag},
};
#[test]
fn rendered_ranges_bind_open_flags_to_exact_escaped_bytes_and_preserve_occurrence_identity() {
    let mut transcript = transcript(&["Alice", "Smith", "um", "hello"]);
    transcript.segments[0].text = "Alice Smith um hello".into();
    let flag = ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            ranges: vec![0..2],
            class: Class::ProperNoun,
            record_text: "Alice Smith".into(),
            alternative_text: Some("PRIVATE alternative".into()),
            confidence: None,
        }),
    };
    let result = transcript_with_spans(&transcript, std::slice::from_ref(&flag), true).unwrap();
    assert_eq!(result.open_spans.len(), 1);
    let span = &result.open_spans[0];
    assert_eq!(
        &result.text[span.bytes.clone()],
        "[unverified: Alice Smith | PRIVATE alternative]"
    );
    assert_eq!(span.accepted, "Alice Smith");
    assert_eq!(span.flag, flag.identifier);
    assert_eq!(span.occurrence, 0);
    assert!(!format!("{result:?}").contains("Alice"));
    assert!(!format!("{span:?}").contains("PRIVATE"));
    assert_eq!(
        result.text,
        transcript_content(&transcript, std::slice::from_ref(&flag), true).unwrap()
    );
    let mut resolved = flag;
    resolved.resolution = Resolution::Dismissed {
        at: crate::time::UtcInstant { secs: 1 },
    };
    assert!(
        transcript_with_spans(&transcript, &[resolved], false)
            .unwrap()
            .open_spans
            .is_empty()
    );
}
#[test]
fn formatting_flags_still_have_source_spans_even_without_visible_annotations() {
    let transcript = transcript(&["um", "hello"]);
    let flag = ReviewFlag {
        identifier: Sha256Digest([2; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            ranges: vec![0..1],
            class: Class::Formatting,
            record_text: "um".into(),
            alternative_text: None,
            confidence: None,
        }),
    };
    let result = transcript_with_spans(&transcript, &[flag], true).unwrap();
    assert_eq!(result.open_spans.len(), 1);
    assert_eq!(&result.text[result.open_spans[0].bytes.clone()], "um");
}

fn transcript(words: &[&str]) -> Transcript {
    crate::note::tests::transcript(&[(&words.join(" "), words)])
}
