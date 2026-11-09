mod resolve;
use super::*;
use crate::ports::{AcceptedTranscript, ReviewSnapshot};
use vpt_domain::{
    review::{FlagContent, Resolution, ReviewFlag},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::Class,
};

struct Fixed(Option<ReviewSnapshot>);
impl ReviewLedger for Fixed {
    fn review_snapshot(&self, _: &RecordingId) -> Result<Option<ReviewSnapshot>, LedgerError> {
        Ok(self.0.clone())
    }
}

fn id() -> RecordingId {
    RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()
}
fn snapshot() -> ReviewSnapshot {
    ReviewSnapshot {
        verified_claims: vec![],
        stale_findings: vec![],
        stale_artifacts: vec![],
        accepted: Some(AcceptedTranscript {
            checker_engine: None,
            recording: id(),
            requested_language: "en-US".into(),
            duration: 4.0,
            accepted_at: UtcInstant { secs: 1 },
            lexical_classification: true,
            from_checker: false,
            uncertainty: Default::default(),
            transcript: Transcript {
                engine: EngineMetadata {
                    name: "apple".into(),
                    family: "apple".into(),
                    model: "test".into(),
                    tool: "test".into(),
                    version: "1".into(),
                },
                language: "en-US".into(),
                language_source: LanguageSource::Reported,
                local: true,
                segments: vec![Segment {
                    start: 0.0,
                    end: 4.0,
                    text: "private words".into(),
                    words: vec![
                        Word {
                            text: "private".into(),
                            start: 0.25,
                            end: 1.0,
                            confidence: None,
                        },
                        Word {
                            text: "words".into(),
                            start: 2.0,
                            end: 4.0,
                            confidence: None,
                        },
                    ],
                }],
            },
        }),
        flags: vec![],
    }
}

fn flag(range: Range<usize>, class: Class) -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([2; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            class,
            ranges: vec![range],
            record_text: "private".into(),
            alternative_text: Some("alternative".into()),
            confidence: None,
        }),
    }
}

#[test]
fn insertion_timecodes_use_the_following_word_or_the_final_end() {
    for (range, expected) in [(0..0, 0.25), (1..1, 2.0), (2..2, 4.0)] {
        let mut snapshot = snapshot();
        let mut insertion = flag(range.clone(), Class::Other);
        if let FlagContent::Lexical(flag) = &mut insertion.content {
            flag.record_text.clear()
        }
        snapshot.flags.push(insertion);
        let view = review(&Fixed(Some(snapshot)), &id()).unwrap();
        assert_eq!(
            view.flags[0].occurrences,
            [Occurrence {
                words: range,
                start: expected,
                end: expected
            }]
        );
    }
}

#[test]
fn closed_flags_and_open_formatting_are_omitted_from_review() {
    let mut snapshot = snapshot();
    for resolution in [
        Resolution::Confirmed {
            at: UtcInstant { secs: 2 },
        },
        Resolution::Corrected {
            text: "correction-canary".into(),
            at: UtcInstant { secs: 3 },
        },
        Resolution::Dismissed {
            at: UtcInstant { secs: 4 },
        },
    ] {
        let mut closed = flag(0..1, Class::ProperNoun);
        closed.resolution = resolution;
        snapshot.flags.push(closed);
    }
    snapshot.flags.push(flag(0..1, Class::Formatting));
    snapshot.flags.push(flag(0..1, Class::Other));
    let view = review(&Fixed(Some(snapshot)), &id()).unwrap();
    assert_eq!(view.flags.len(), 1);
    assert_eq!(view.flags[0].flag.class, Class::Other);
    assert!(!format!("{view:?}").contains("canary"));
    assert!(view.transcribed);
}

#[test]
fn absent_and_untranscribed_recordings_are_distinct() {
    assert_eq!(review(&Fixed(None), &id()), Err(ReviewError::NotFound));
    let report = review(
        &Fixed(Some(ReviewSnapshot {
            verified_claims: vec![],
            stale_findings: vec![],
            stale_artifacts: vec![],
            accepted: None,
            flags: vec![],
        })),
        &id(),
    )
    .unwrap();
    assert!(!report.transcribed);
    assert!(report.flags.is_empty());
}

#[test]
fn invalid_ranges_or_timings_refuse_without_flag_text() {
    for case in 0..3 {
        let mut snapshot = snapshot();
        snapshot
            .flags
            .push(flag(if case == 0 { 0..3 } else { 0..1 }, Class::Other));
        if case == 1 {
            snapshot.accepted = None
        }
        if case == 2 {
            snapshot.accepted.as_mut().unwrap().transcript.segments[0].words[0].start = f64::NAN
        }
        assert_eq!(
            review(&Fixed(Some(snapshot)), &id()),
            Err(ReviewError::InvalidFlags)
        );
    }
}
