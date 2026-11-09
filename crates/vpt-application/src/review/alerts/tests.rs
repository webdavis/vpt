use super::*;
mod findings;
use crate::review::{FlagView, Occurrence};
use vpt_domain::{
    digest::Sha256Digest,
    uncertainty::{Diagnostic, LexicalFlag},
};

fn review() -> Review {
    Review {
        findings: vec![],
        recording: RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap(),
        transcribed: true,
        flags: vec![],
        diagnostics: vec![],
    }
}

#[test]
fn resolved_review_needs_no_notice_but_open_diagnostics_do() {
    let mut review = review();
    assert!(ReviewNotice::from_review(&review, "/notes/private.md".into()).is_none());
    review
        .diagnostics
        .push((Sha256Digest([1; 32]), Diagnostic::SingleEngine));
    let notice = ReviewNotice::from_review(&review, "/notes/private.md".into()).unwrap();
    assert_eq!(notice.counts.diagnostics, 1);
    let event = notice.event(UtcInstant { secs: 10 });
    assert_eq!(event.kind, EventKind::ReviewNeeded);
    assert_eq!(event.paths[0].key, EventPathKey::Note);
}

#[test]
fn notice_counts_each_occurrence_and_retains_no_transcript_text() {
    let mut review = review();
    review.flags.push(FlagView {
        identifier: Sha256Digest([2; 32]),
        flag: LexicalFlag {
            class: Class::AgreedUnverified,
            ranges: vec![2..3, 4..5],
            record_text: "Private name".into(),
            alternative_text: None,
            confidence: None,
        },
        occurrences: vec![
            Occurrence {
                words: 2..3,
                start: 2.0,
                end: 3.0,
            },
            Occurrence {
                words: 4..5,
                start: 4.0,
                end: 5.0,
            },
        ],
    });
    let notice = ReviewNotice::from_review(&review, "/notes/memo.md".into()).unwrap();
    assert_eq!(notice.counts.agreed_unverified, 2);
    assert!(!format!("{notice:?}").contains("Private name"));
    assert_eq!(
        notice.event(UtcInstant { secs: 10 }).detail,
        EventDetail::Review {
            spans: 0,
            agreed_unverified: 2
        }
    );
}

#[test]
fn threshold_exceeded_produces_one_event_with_recording_count_and_combined_classes() {
    let notices: Vec<_> = (0..4)
        .map(|seed| ReviewNotice {
            recording: RecordingId::derive(
                UtcInstant { secs: 100 },
                vpt_domain::time::UtcOffset { secs: 0 },
                &Sha256Digest([seed; 32]),
            )
            .unwrap(),
            path: format!("/notes/{seed}.md").into(),
            counts: EventCounts {
                low_confidence: 1,
                agreed_unverified: 2,
                diagnostics: 1,
                ..Default::default()
            },
        })
        .collect();
    let at = UtcInstant { secs: 20 };
    assert_eq!(events(&notices[..3], 3, at).unwrap().len(), 3);
    let aggregate = events(&notices, 3, at).unwrap();
    assert_eq!(aggregate.len(), 1);
    assert!(aggregate[0].recording.is_none());
    assert!(aggregate[0].paths.is_empty());
    assert_eq!(aggregate[0].counts.low_confidence, 4);
    assert_eq!(aggregate[0].counts.agreed_unverified, 8);
    assert_eq!(aggregate[0].detail_text(), "4 recordings need review");
    assert!(events(&[], 0, at).unwrap().is_empty());
}

#[test]
fn duplicate_recordings_overflow_and_excessive_batches_refuse() {
    let base = ReviewNotice {
        recording: review().recording,
        path: "/notes/a.md".into(),
        counts: EventCounts::default(),
    };
    let at = UtcInstant { secs: 20 };
    assert_eq!(
        events(&[base.clone(), base.clone()], 3, at),
        Err(AlertError::DuplicateRecording)
    );
    let mut other = base.clone();
    other.recording = RecordingId::parse("2026-08-24T144736-aaaaaaaaaaaa").unwrap();
    let mut maximum = base.clone();
    maximum.counts.numeric = u32::MAX;
    other.counts.numeric = 1;
    assert_eq!(
        events(&[maximum, other], 0, at),
        Err(AlertError::CapacityExceeded)
    );
    assert_eq!(
        events(&vec![base; 65_537], 3, at),
        Err(AlertError::CapacityExceeded)
    );
}

#[test]
fn rejected_queue_addition_does_not_change_the_accumulated_events() {
    let mut queue = ReviewQueue::default();
    let notice = ReviewNotice {
        recording: review().recording,
        path: "/notes/a.md".into(),
        counts: EventCounts {
            diagnostics: 1,
            ..Default::default()
        },
    };
    queue.push(notice.clone()).unwrap();
    assert_eq!(queue.push(notice), Err(AlertError::DuplicateRecording));
    let events = queue.events(3, UtcInstant { secs: 20 }).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].counts.diagnostics, 1);
}
