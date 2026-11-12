#![allow(
    clippy::single_range_in_vec_init,
    reason = "Flags and markers use ranges"
)]
use super::*;
use sha2::{Digest, Sha256};
use vpt_domain::{
    digest::Sha256Digest,
    grounding::{Finding, SourceRange},
    note::NoteStage,
    review::{FlagContent, Resolution, VerificationFlag},
    transcript::{EngineMetadata, LanguageSource, Segment, Word},
    uncertainty::{Class, LexicalFlag},
};
fn transcript() -> Transcript {
    Transcript {
        engine: EngineMetadata {
            name: "fixture".into(),
            tool: "fixture".into(),
            version: "1".into(),
            model: "fixture".into(),
            family: "fixture".into(),
        },
        language: "en-US".into(),
        language_source: LanguageSource::Reported,
        local: true,
        segments: vec![Segment {
            speaker: None,
            start: 0.0,
            end: 3.0,
            text: "Before Alice after".into(),
            words: ["Before", "Alice", "after"]
                .into_iter()
                .enumerate()
                .map(|(i, text)| Word {
                    text: text.into(),
                    start: i as f64,
                    end: i as f64 + 1.0,
                    confidence: None,
                })
                .collect(),
        }],
    }
}
fn flag() -> ReviewFlag {
    ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Lexical(LexicalFlag {
            ranges: vec![1..2],
            class: Class::ProperNoun,
            record_text: "Alice".into(),
            alternative_text: Some("PRIVATE alternative".into()),
            confidence: None,
        }),
    }
}
fn redactor() -> Redactor {
    Redactor::new(&["Alice"], &[], vpt_domain::redaction::Options::default()).unwrap()
}
fn visible(text: &str) -> String {
    super::super::tests::visible(text)
}
#[test]
fn transcript_omission_removes_only_open_flagged_words_and_marking_keeps_accepted_text() {
    let transcript = transcript();
    let flags = [flag()];
    let source = vpt_domain::note::transcript_content(&transcript, &flags, false).unwrap();
    let omitted = redact_reviewed(
        &source,
        ReviewSource::Transcript {
            transcript: &transcript,
            flags: &flags,
        },
        FlagPolicy::Omit,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(omitted.omitted, 1);
    assert!(omitted.mappings.is_empty());
    assert!(!visible(&omitted.text).contains("Alice"));
    assert!(!visible(&omitted.text).contains("alternative"));
    assert!(visible(&omitted.text).contains("Before  after"));
    let marked = redact_reviewed(
        &source,
        ReviewSource::Transcript {
            transcript: &transcript,
            flags: &flags,
        },
        FlagPolicy::Mark,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(marked.marked, 1);
    assert!(visible(&marked.text).contains("Before [term 1] [unverified] after"));
    assert!(!visible(&marked.text).contains("alternative"));
    let line = marked.lines.last().unwrap();
    assert!(source[line.source.clone()].contains("PRIVATE alternative"));
}
#[test]
fn stale_transcript_source_refuses_instead_of_applying_word_offsets_to_changed_text() {
    let transcript = transcript();
    let flags = [flag()];
    let source = vpt_domain::note::transcript_content(&transcript, &flags, false)
        .unwrap()
        .replace("Before", "PRIVATE changed");
    let error = redact_reviewed(
        &source,
        ReviewSource::Transcript {
            transcript: &transcript,
            flags: &flags,
        },
        FlagPolicy::Omit,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error, Error::InvalidText);
    assert!(!format!("{error:?}").contains("PRIVATE"));
}
fn analysis_flag(source: &str) -> ReviewFlag {
    let parsed = crate::markdown_claims::extract_content(source).unwrap();
    let claim = &parsed.claims()[0];
    ReviewFlag {
        identifier: Sha256Digest([2; 32]),
        resolution: Resolution::Open,
        content: FlagContent::Verification(VerificationFlag {
            artifact: NoteStage::Analysis,
            claim_index: 0,
            claim_digest: Sha256Digest(Sha256::digest(claim.identity_text.as_bytes()).into()),
            record_text: claim.identity_text.clone(),
            finding: Finding::BuiltOnFlaggedText,
            sources: vec![SourceRange {
                start_secs: 0,
                end_secs: 2,
            }],
        }),
    }
}
#[test]
fn analysis_policy_applies_to_the_whole_saved_claim_and_never_uses_transcript_word_offsets() {
    let source = "## Summary\n\n- Private claim [00:00-00:02] [built-on-flagged-text]\n- Other claim [unsourced]\n";
    let mut lexical = flag();
    if let FlagContent::Lexical(value) = &mut lexical.content {
        value.ranges = vec![90..99];
    }
    let flags = [analysis_flag(source), lexical];
    let omitted = redact_reviewed(
        source,
        ReviewSource::Analysis { flags: &flags },
        FlagPolicy::Omit,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(omitted.omitted, 1);
    assert!(!omitted.text.contains("Private"));
    assert!(omitted.text.contains("Other claim"));
    let marked = redact_reviewed(
        source,
        ReviewSource::Analysis { flags: &flags },
        FlagPolicy::Mark,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(marked.marked, 1);
    assert!(visible(&marked.text).contains("Private claim  [unverified]"));
    assert!(!marked.text.contains("built-on-flagged-text"));
    let stale = source.replace("Private claim", "Changed claim");
    assert!(
        redact_reviewed(
            &stale,
            ReviewSource::Analysis { flags: &flags },
            FlagPolicy::Omit,
            &mut redactor(),
            &ContentOptions::default()
        )
        .is_err()
    );
}

#[test]
fn one_flag_spanning_segments_is_counted_once_and_each_retained_piece_is_marked() {
    let mut transcript = transcript();
    transcript.segments = vec![
        Segment {
            speaker: None,
            start: 0.0,
            end: 1.0,
            text: "Alice".into(),
            words: vec![Word {
                text: "Alice".into(),
                start: 0.0,
                end: 1.0,
                confidence: None,
            }],
        },
        Segment {
            speaker: None,
            start: 2.0,
            end: 3.0,
            text: "Smith".into(),
            words: vec![Word {
                text: "Smith".into(),
                start: 2.0,
                end: 3.0,
                confidence: None,
            }],
        },
    ];
    let mut flag = flag();
    if let FlagContent::Lexical(value) = &mut flag.content {
        value.ranges = vec![0..2];
        value.record_text = "Alice Smith".into();
    }
    let source =
        vpt_domain::note::transcript_content(&transcript, std::slice::from_ref(&flag), true)
            .unwrap();
    for policy in [FlagPolicy::Omit, FlagPolicy::Mark] {
        let result = redact_reviewed(
            &source,
            ReviewSource::Transcript {
                transcript: &transcript,
                flags: std::slice::from_ref(&flag),
            },
            policy,
            &mut redactor(),
            &ContentOptions::default(),
        )
        .unwrap();
        if policy == FlagPolicy::Omit {
            assert_eq!(result.omitted, 1);
            assert!(!result.text.contains("Smith"));
        } else {
            assert_eq!(result.marked, 1);
            assert_eq!(visible(&result.text).matches("[unverified]").count(), 2);
        }
    }
}

#[test]
fn empty_insertions_drop_alternatives_and_keep_warning_without_inventing_accepted_words() {
    let transcript = transcript();
    let mut flag = flag();
    if let FlagContent::Lexical(value) = &mut flag.content {
        value.ranges = vec![1..1];
        value.record_text = String::new();
    }
    let source =
        vpt_domain::note::transcript_content(&transcript, std::slice::from_ref(&flag), false)
            .unwrap();
    let result = redact_reviewed(
        &source,
        ReviewSource::Transcript {
            transcript: &transcript,
            flags: &[flag],
        },
        FlagPolicy::Mark,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(result.marked, 1);
    assert!(visible(&result.text).contains("[unverified]"));
    assert!(!result.text.contains("alternative"));
}

#[test]
fn warning_markers_survive_timecode_removal_and_cannot_be_swallowed_by_a_longer_mask() {
    let transcript = transcript();
    let flags = [flag()];
    let source = vpt_domain::note::transcript_content(&transcript, &flags, false).unwrap();
    for keep in [true, false] {
        let mut redactor = Redactor::new(
            &["Alice [unverified]"],
            &[],
            vpt_domain::redaction::Options::default(),
        )
        .unwrap();
        let result = redact_reviewed(
            &source,
            ReviewSource::Transcript {
                transcript: &transcript,
                flags: &flags,
            },
            FlagPolicy::Mark,
            &mut redactor,
            &ContentOptions {
                keep_timecodes: keep,
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(Error::Residue { .. })));
    }
}

#[test]
fn transcript_source_binding_accounts_for_verification_annotations_and_readability() {
    let mut transcript = transcript();
    transcript.segments[0].text = "um Before Alice after".into();
    transcript.segments[0].words.insert(
        0,
        Word {
            text: "um".into(),
            start: 0.0,
            end: 0.0,
            confidence: None,
        },
    );
    let mut flag = flag();
    if let FlagContent::Lexical(value) = &mut flag.content {
        value.ranges = vec![2..3];
    }
    let flags = [flag];
    let source = vpt_domain::note::transcript_content(&transcript, &flags, true).unwrap();
    let parsed =
        crate::markdown_claims::extract_owned(&source, NoteStage::Transcript, &flags).unwrap();
    let annotated = parsed.annotate(&[Some(Finding::Unsourced)]).unwrap();
    let result = redact_reviewed(
        &annotated,
        ReviewSource::Transcript {
            transcript: &transcript,
            flags: &flags,
        },
        FlagPolicy::Omit,
        &mut redactor(),
        &ContentOptions::default(),
    )
    .unwrap();
    assert_eq!(result.omitted, 1);
    assert!(!result.text.contains("alternative"));
    assert!(visible(&result.text).contains("Before  after"));
    assert!(!visible(&result.text).contains("um "));
}
