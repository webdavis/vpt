#![allow(
    clippy::single_range_in_vec_init,
    reason = "Protected spans are byte ranges"
)]
use super::*;

#[test]
fn disabled_readability_keeps_every_byte_and_range() {
    let text = "  um, hello  世界!\n";
    let ranges = [6..11];
    assert_eq!(
        apply(text, &ranges, false).unwrap(),
        Readable {
            text: text.into(),
            protected: ranges.to_vec()
        }
    );
}

#[test]
fn removes_only_closed_fillers_and_like_followed_by_a_comma() {
    for (input, expected) in [
        ("um, hello uh there erm", "Hello there."),
        ("like, hello", "Hello."),
        ("like , hello", "Hello."),
        ("I like this", "I like this."),
        ("like. this stays", "Like. This stays."),
        ("umbrella uh-oh ermine", "Umbrella uh-oh ermine."),
        ("um1 umami", "Um1 umami."),
        ("UM Uh ErM", ""),
    ] {
        assert_eq!(apply(input, &[], true).unwrap().text, expected, "{input}");
    }
}

#[test]
fn capitalization_and_terminal_punctuation_preserve_existing_sentence_marks() {
    for (input, expected) in [
        ("hello. world! again? yes", "Hello. World! Again? Yes."),
        ("hello!", "Hello!"),
        ("hello?\n", "Hello?\n"),
        ("hello  ", "Hello.  "),
        ("3.14 dollars", "3.14 dollars."),
        ("version 1.2 works", "Version 1.2 works."),
        ("ßeta ends", "SSeta ends."),
    ] {
        assert_eq!(apply(input, &[], true).unwrap().text, expected, "{input}");
    }
}

#[test]
fn all_protected_bytes_survive_and_positions_follow_surrounding_edits() {
    let input = "um, secret  name then uh Alice";
    let ranges = [4..16, 25..30];
    let result = apply(input, &ranges, true).unwrap();
    assert_eq!(result.text, "secret  name then Alice.");
    for (before, after) in ranges.iter().zip(&result.protected) {
        assert_eq!(&input[before.clone()], &result.text[after.clone()]);
    }
    assert_eq!(result.protected, [0..12, 18..23]);
}

#[test]
fn flagged_fillers_capitals_and_inline_markers_are_copied_verbatim() {
    for text in [
        "um",
        "like,",
        "[unverified: um | erm]",
        "[corrected: lower case]",
    ] {
        let result = apply(text, &[0..text.len()], true).unwrap();
        assert_eq!(result.text, format!("{text}."));
        assert_eq!(result.protected, [0..text.len()]);
    }
}

#[test]
fn protected_punctuation_and_whitespace_are_not_deleted_with_a_filler() {
    let result = apply("like , hello", &[5..6], true).unwrap();
    assert_eq!(&result.text[result.protected[0].clone()], ",");
    let result = apply("um  hello", &[2..4], true).unwrap();
    assert_eq!(&result.text[result.protected[0].clone()], "  ");
}

#[test]
fn empty_protected_ranges_keep_an_anchor_when_adjacent_words_are_removed() {
    let result = apply("um hello", &[3..3], true).unwrap();
    assert_eq!(result.text, "Hello.");
    assert_eq!(result.protected, [0..0]);
}

#[test]
fn invalid_byte_boundaries_overlaps_and_excessive_input_are_refused() {
    for ranges in [vec![1..2], vec![0..99], vec![0..3, 2..4], vec![4..4, 0..0]] {
        assert_eq!(
            apply("é hello", &ranges, true),
            Err(ReadabilityError::InvalidRanges)
        );
    }
    assert_eq!(
        apply(&"x".repeat(16 * 1024 * 1024 + 1), &[], true),
        Err(ReadabilityError::CapacityExceeded)
    );
    assert_eq!(
        apply("x", &vec![0..0; 65_537], true),
        Err(ReadabilityError::CapacityExceeded)
    );
}

#[test]
fn every_protected_substring_survives_unicode_expansion_deletions_and_punctuation() {
    for text in [
        "um, ΐ secret",
        "like , hello  ",
        "um uh erm",
        "hello. world",
        "\"hello\" ",
        "é um, like, secret",
    ] {
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(index, _)| index)
            .chain([text.len()])
            .collect();
        for (index, start) in boundaries.iter().enumerate() {
            for end in &boundaries[index..] {
                let protected = *start..*end;
                let result = apply(text, std::slice::from_ref(&protected), true).unwrap();
                let after = result.protected[0].clone();
                assert_eq!(
                    &text[protected.clone()],
                    &result.text[after],
                    "{text:?} {protected:?}"
                );
            }
        }
    }
}
