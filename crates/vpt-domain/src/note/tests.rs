#![allow(clippy::single_range_in_vec_init, reason = "Flags store word ranges")]
use super::*;
use crate::{
    digest::Sha256Digest,
    review::{FlagContent, Resolution},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Word},
    uncertainty::{Class, Diagnostic, LexicalFlag},
};
mod safety;

pub(super) fn transcript(parts: &[(&str, &[&str])]) -> Transcript {
    Transcript {
        engine: EngineMetadata {
            name: "test".into(),
            tool: "test".into(),
            version: "1".into(),
            model: "test".into(),
            family: "test".into(),
        },
        language: "en-US".into(),
        language_source: LanguageSource::Reported,
        local: true,
        segments: parts
            .iter()
            .enumerate()
            .map(|(i, (text, words))| Segment {
                speaker: None,
                start: i as f64 * 7.0,
                end: i as f64 * 7.0 + 6.0,
                text: (*text).into(),
                words: words
                    .iter()
                    .map(|text| Word {
                        text: (*text).into(),
                        start: i as f64 * 7.0,
                        end: i as f64 * 7.0 + 6.0,
                        confidence: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn flag(
    class: Class,
    ranges: Vec<std::ops::Range<usize>>,
    text: &str,
    alt: Option<&str>,
) -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        content: FlagContent::Lexical(LexicalFlag {
            class,
            ranges,
            record_text: text.into(),
            alternative_text: alt.map(str::to_owned),
            confidence: None,
        }),
        resolution: Resolution::Open,
    }
}

#[test]
fn one_timecoded_paragraph_per_segment_preserves_original_spacing_and_punctuation() {
    let mut record = transcript(&[
        ("hello,  world!", &["hello", "world"]),
        ("again", &["again"]),
    ]);
    record.segments[1].start = 3_661.9;
    record.segments[1].end = 3_667.0;
    record.segments[1].words[0].start = 3_661.9;
    record.segments[1].words[0].end = 3_667.0;
    assert_eq!(
        transcript_content(&record, &[], false).unwrap(),
        "## Transcript\n\n[00:00] hello,  world!\n\n[61:01] again\n"
    );
}

#[test]
fn every_repeated_occurrence_is_annotated_and_readability_preserves_flags_of_all_classes() {
    let record = transcript(&[(
        "um, alice then um alice",
        &["um", "alice", "then", "um", "alice"],
    )]);
    let flags = [
        flag(Class::AgreedUnverified, vec![1..2, 4..5], "alice", None),
        flag(Class::Formatting, vec![3..4], "um", Some("the um")),
    ];
    let original = record.clone();
    assert_eq!(
        transcript_content(&record, &flags, true).unwrap(),
        "## Transcript\n\n[00:00] [unverified: alice] then um [unverified: alice].\n"
    );
    assert_eq!(record, original);
}

#[test]
fn disagreement_and_each_resolution_have_distinct_inline_rendering() {
    let record = transcript(&[("met Alice today", &["met", "Alice", "today"])]);
    let mut flags = [flag(Class::ProperNoun, vec![1..2], "Alice", Some("Alix"))];
    for (resolution, expected) in [
        (Resolution::Open, "[unverified: Alice | Alix]"),
        (
            Resolution::Confirmed {
                at: UtcInstant { secs: 1 },
            },
            "Alice",
        ),
        (
            Resolution::Dismissed {
                at: UtcInstant { secs: 1 },
            },
            "Alice",
        ),
        (
            Resolution::Corrected {
                text: "Alicia".into(),
                at: UtcInstant { secs: 1 },
            },
            "[corrected: Alicia]",
        ),
    ] {
        flags[0].resolution = resolution;
        assert_eq!(
            transcript_content(&record, &flags, false).unwrap(),
            format!("## Transcript\n\n[00:00] met {expected} today\n")
        );
    }
}

#[test]
fn a_flag_spanning_segments_keeps_each_timecode_and_shows_a_correction_once() {
    let record = transcript(&[
        ("met Alice", &["met", "Alice"]),
        ("Smith today", &["Smith", "today"]),
    ]);
    let mut flags = [flag(
        Class::ProperNoun,
        vec![1..3],
        "Alice Smith",
        Some("Alix Smyth"),
    )];
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] met [unverified: Alice | Alix Smyth]\n\n[00:07] [unverified: Smith] today\n"
    );
    flags[0].resolution = Resolution::Corrected {
        text: "Alicia Smith".into(),
        at: UtcInstant { secs: 1 },
    };
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] met [corrected: Alicia Smith]\n\n[00:07]  today\n"
    );
}

#[test]
fn inconsistent_segment_text_uses_accepted_words_without_losing_a_flag() {
    let record = transcript(&[("met a different person", &["met", "Alice"])]);
    let flags = [flag(Class::AgreedUnverified, vec![1..2], "Alice", None)];
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] met [unverified: Alice]\n"
    );
}

#[test]
fn insertion_flags_anchor_before_the_next_word_or_after_the_last_word() {
    let record = transcript(&[("hello", &["hello"]), ("world", &["world"])]);
    let flags = [
        flag(Class::Other, vec![1..1], "", Some("dear")),
        flag(Class::Other, vec![2..2], "", Some("again")),
    ];
    assert_eq!(
        transcript_content(&record, &flags, false).unwrap(),
        "## Transcript\n\n[00:00] hello\n\n[00:07] [unverified:  | dear] world [unverified:  | again]\n"
    );
}

#[test]
fn diagnostics_do_not_change_body_or_hold_back_readability() {
    let record = transcript(&[("um hello", &["um", "hello"])]);
    let flags = [ReviewFlag {
        identifier: Sha256Digest([2; 32]),
        content: FlagContent::Diagnostic(Diagnostic::SingleEngine),
        resolution: Resolution::Open,
    }];
    assert_eq!(
        transcript_content(&record, &flags, true).unwrap(),
        "## Transcript\n\n[00:00] Hello.\n"
    );
}
