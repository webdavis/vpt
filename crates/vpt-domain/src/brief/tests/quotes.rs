use super::*;
use crate::{
    review::{FlagContent, Resolution, ReviewFlag},
    uncertainty::{Class, LexicalFlag},
};

fn segments() -> Vec<Segment> {
    [
        "Private Person said 17.",
        "Private filler.",
        "Private Person said 18.",
    ]
    .into_iter()
    .enumerate()
    .map(|(n, text)| {
        let start = n as f64 * 10.0;
        Segment {
            speaker: None,
            start,
            end: start + 5.0,
            text: text.into(),
            words: text
                .split_whitespace()
                .enumerate()
                .map(|(i, text)| Word {
                    text: text.into(),
                    start: start + i as f64,
                    end: start + i as f64 + 0.5,
                    confidence: None,
                })
                .collect(),
        }
    })
    .collect()
}
fn flag() -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([90; 32]),
        content: FlagContent::Lexical(LexicalFlag {
            ranges: std::iter::once(3..4).collect(),
            class: Class::Numeric,
            record_text: "17.".into(),
            alternative_text: Some("70.".into()),
            confidence: None,
        }),
        resolution: Resolution::Open,
    }
}
fn rows() -> Vec<Candidate> {
    let mut row = candidate(1, 100);
    row.relations.push(mention("/index/person.md"));
    let source = row.source.as_mut().unwrap();
    source.transcript.segments = segments();
    source.flags = vec![flag()];
    vec![row]
}

#[test]
fn participant_quotes_are_verbatim_matching_segments_with_sources_and_inline_certainty_data() {
    let occasion = occasion();
    let terms = [term("Private Person", "/index/person.md")];
    let rows = rows();
    let result = assemble(&request(&occasion, &terms, &rows)).unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].text, "Private Person said 17.");
    assert_eq!(result.items[0].certainty, Certainty::Unverified);
    assert_eq!(result.items[1].text, "Private Person said 18.");
    assert_eq!(result.items[1].certainty, Certainty::Confirmed);
    assert_eq!(result.items[1].source.at_secs, 20.0);
    assert_eq!(result.items[0].source.recording, rows[0].recording);
    assert_eq!(
        result.items[0].source.digest,
        rows[0].source.as_ref().unwrap().digest
    );
    assert_eq!(
        result.items[0].source.note,
        rows[0].source.as_ref().unwrap().note
    );
    assert_eq!(
        result.unresolved,
        Unresolved {
            flags: 1,
            notes_with_flags: 1,
            reviewed: true
        }
    );
    assert!(!format!("{:?}", result.items).contains("Private"));
}

#[test]
fn span_deduplication_precedes_limit_and_equal_text_at_different_times_is_distinct() {
    let mut occasion = occasion();
    occasion.participants.clear();
    let mut rows = rows();
    let row = &mut rows[0];
    row.tags.push(tag("planning", tags::State::Confirmed));
    let source = row.source.as_mut().unwrap();
    source.flags.clear();
    source.transcript.segments = vec![
        segments()[0].clone(),
        segments()[0].clone(),
        segments()[2].clone(),
    ];
    source.transcript.segments[2].text = source.transcript.segments[0].text.clone();
    let mut request = request(&occasion, &[], &rows);
    request.options.max_spans_per_note = 1;
    let result = assemble(&request).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.omitted_spans, 1);
    request.options.max_spans_per_note = 5;
    let result = assemble(&request).unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].text, result.items[1].text);
    assert_ne!(
        result.items[0].source.at_secs,
        result.items[1].source.at_secs
    );
}

#[test]
fn corrected_original_quotes_remain_unverified_and_disabled_review_is_explicit() {
    let occasion = occasion();
    let terms = [term("Private Person", "/index/person.md")];
    let mut rows = rows();
    rows[0].source.as_mut().unwrap().flags[0].resolution = Resolution::Corrected {
        text: "70".into(),
        at: UtcInstant { secs: 1 },
    };
    let result = assemble(&request(&occasion, &terms, &rows)).unwrap();
    assert_eq!(result.items[0].text, "Private Person said 17.");
    assert_eq!(result.items[0].certainty, Certainty::Unverified);
    assert_eq!(result.unresolved.flags, 0);
    for resolution in [
        Resolution::Confirmed {
            at: UtcInstant { secs: 1 },
        },
        Resolution::Dismissed {
            at: UtcInstant { secs: 1 },
        },
    ] {
        rows[0].source.as_mut().unwrap().flags[0].resolution = resolution;
        assert_eq!(
            assemble(&request(&occasion, &terms, &rows)).unwrap().items[0].certainty,
            Certainty::Confirmed
        );
    }
    rows[0].source.as_mut().unwrap().reviewed = false;
    let result = assemble(&request(&occasion, &terms, &rows)).unwrap();
    assert!(!result.unresolved.reviewed);
    assert!(
        result
            .items
            .iter()
            .all(|item| item.certainty == Certainty::Unverified)
    );
}

#[test]
fn an_open_divergent_diagnostic_marks_the_whole_pack_unreviewed() {
    let occasion = occasion();
    let terms = [term("Private Person", "/index/person.md")];
    let mut rows = rows();
    rows[0].source.as_mut().unwrap().flags = vec![ReviewFlag {
        identifier: Sha256Digest([98; 32]),
        content: FlagContent::Diagnostic(crate::uncertainty::Diagnostic::Divergent),
        resolution: Resolution::Open,
    }];
    let result = assemble(&request(&occasion, &terms, &rows)).unwrap();
    assert!(!result.unresolved.reviewed);
    assert!(
        result
            .items
            .iter()
            .all(|item| item.certainty == Certainty::Unverified)
    );
    assert_eq!(result.unresolved.flags, 1);
}
