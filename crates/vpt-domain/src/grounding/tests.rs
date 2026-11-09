use super::*;
mod boundaries;
mod corrections;
mod facts;
use crate::{
    review::{FlagContent, Resolution},
    time::UtcInstant,
    uncertainty::{Class, LexicalFlag},
};

fn words(values: &[(&str, f64, f64)]) -> Vec<Word> {
    values
        .iter()
        .map(|(text, start, end)| Word {
            text: (*text).into(),
            start: *start,
            end: *end,
            confidence: None,
        })
        .collect()
}
fn claim(text: &str, sources: &[&str]) -> Claim {
    Claim {
        text: text.into(),
        source_references: sources.iter().map(|s| (*s).into()).collect(),
    }
}
fn flag(class: Class, resolution: Resolution) -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([7; 32]),
        resolution,
        content: FlagContent::Lexical(LexicalFlag {
            ranges: std::iter::once(1..2).collect(),
            class,
            record_text: "Private".into(),
            alternative_text: None,
            confidence: None,
        }),
    }
}

#[test]
fn missing_and_bad_references_precede_other_findings() {
    let words = words(&[("Alice", 0.0, 1.0)]);
    let mut checker = Grounder::new(words.iter(), &[], 10.0, 15).unwrap();
    assert_eq!(
        checker.check(&claim("Bob", &[])).unwrap().finding,
        Some(Finding::Unsourced)
    );
    for reference in [
        "",
        "bad",
        "00:03-00:01",
        "00:01-00:01",
        "00:00-00:11",
        "00:60-01:01",
        "0:1-00:02",
        "999999999999999999999:00-999999999999999999999:01",
    ] {
        let result = checker.check(&claim("Bob", &[reference])).unwrap();
        assert_eq!(result.finding, Some(Finding::BadReference), "{reference}");
        assert!(result.sources.is_empty());
    }
}

#[test]
fn supported_claims_keep_sources_and_context_window_does_not_extend_inherited_uncertainty() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 5.0, 6.0)]);
    let flags = vec![flag(Class::Numeric, Resolution::Open)];
    let mut checker = Grounder::new(words.iter(), &flags, 10.0, 5).unwrap();
    let result = checker
        .check(&claim("Alice paid 42", &["00:00-00:01"]))
        .unwrap();
    assert_eq!(result.finding, None);
    assert_eq!(
        result.sources,
        [SourceRange {
            start_secs: 0,
            end_secs: 1
        }]
    );
    assert!(result.inherited_flags.is_empty());
    let result = checker
        .check(&claim("Alice paid 42", &["00:05-00:06"]))
        .unwrap();
    assert_eq!(result.finding, Some(Finding::BuiltOnFlaggedText));
    assert_eq!(result.inherited_flags, [Sha256Digest([7; 32])]);
}

#[test]
fn unsupported_facts_precede_annotations_but_preserve_inherited_flags() {
    let words = words(&[("alice", 0.0, 1.0), ("42", 1.0, 2.0)]);
    let flags = vec![flag(Class::Numeric, Resolution::Open)];
    let mut checker = Grounder::new(words.iter(), &flags, 5.0, 0).unwrap();
    let result = checker
        .check(&claim("PrivateBob paid 42", &["00:00-00:02"]))
        .unwrap();
    assert_eq!(
        result.finding,
        Some(Finding::Unsupported("PrivateBob".into()))
    );
    assert_eq!(result.inherited_flags, [Sha256Digest([7; 32])]);
    assert!(!format!("{result:?}").contains("PrivateBob"));
}

#[test]
fn only_open_surfaced_lexical_flags_are_inherited() {
    let words = words(&[("alice", 0.0, 1.0), ("42", 1.0, 2.0)]);
    for (class, resolution, inherited) in [
        (Class::Formatting, Resolution::Open, false),
        (
            Class::Numeric,
            Resolution::Confirmed {
                at: UtcInstant { secs: 1 },
            },
            false,
        ),
        (
            Class::ProperNoun,
            Resolution::Dismissed {
                at: UtcInstant { secs: 1 },
            },
            false,
        ),
        (Class::LowConfidence, Resolution::Open, true),
        (Class::AgreedUnverified, Resolution::Open, true),
        (Class::Other, Resolution::Open, true),
    ] {
        let flags = vec![flag(class, resolution)];
        let mut checker = Grounder::new(words.iter(), &flags, 5.0, 0).unwrap();
        let result = checker
            .check(&claim("Alice paid 42", &["00:00-00:02"]))
            .unwrap();
        assert_eq!(!result.inherited_flags.is_empty(), inherited);
    }
}

#[test]
fn inherited_flag_order_is_independent_of_the_ledger_read_order() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 1.0, 2.0)]);
    let first = flag(Class::Numeric, Resolution::Open);
    let mut second = first.clone();
    second.identifier = Sha256Digest([3; 32]);
    if let FlagContent::Lexical(content) = &mut second.content {
        content.class = Class::ProperNoun;
        content.ranges = std::iter::once(0..1).collect();
        content.record_text = "Alice".into();
    }
    let flags = [first.clone(), second.clone()];
    let reversed = [second, first];
    let claim = claim("Alice 42", &["00:00-00:02"]);
    let a = Grounder::new(words.iter(), &flags, 2.0, 0)
        .unwrap()
        .check(&claim)
        .unwrap();
    let b = Grounder::new(words.iter(), &reversed, 2.0, 0)
        .unwrap()
        .check(&claim)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(
        a.inherited_flags,
        vec![Sha256Digest([3; 32]), Sha256Digest([7; 32])]
    );
}
