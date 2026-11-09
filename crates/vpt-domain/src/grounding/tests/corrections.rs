use super::*;

fn correction(ranges: Vec<std::ops::Range<usize>>, text: &str) -> ReviewFlag {
    let mut flag = flag(
        Class::Other,
        Resolution::Corrected {
            text: text.into(),
            at: UtcInstant { secs: 1 },
        },
    );
    let FlagContent::Lexical(lexical) = &mut flag.content else {
        unreachable!()
    };
    lexical.ranges = ranges;
    flag
}

#[test]
fn corrected_text_supports_claims_without_mutating_accepted_words_or_inheriting_closed_flags() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 1.0, 2.0)]);
    let original = words.clone();
    let flags = [correction(std::iter::once(1..2).collect(), "43")];
    let mut checker = Grounder::new(words.iter(), &flags, 3.0, 0).unwrap();
    let result = checker
        .check(&claim("Alice paid 43", &["00:00-00:02"]))
        .unwrap();
    assert_eq!(result.finding, None);
    assert!(result.inherited_flags.is_empty());
    assert_eq!(
        checker
            .check(&claim("42", &["00:00-00:02"]))
            .unwrap()
            .finding,
        Some(Finding::Unsupported("42".into()))
    );
    assert!(words == original);
}

#[test]
fn repeated_corrections_and_insertions_keep_their_own_time_anchors() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 1.0, 2.0), ("Alice", 2.0, 3.0)]);
    for (ranges, sources) in [
        (vec![0..1, 2..3], vec!["00:00-00:01", "00:02-00:03"]),
        (std::iter::once(1..1).collect(), vec!["00:01-00:02"]),
        (std::iter::once(3..3).collect(), vec!["00:03-00:04"]),
    ] {
        let flags = [correction(ranges, "Bob")];
        let mut checker = Grounder::new(words.iter(), &flags, 4.0, 0).unwrap();
        for source in sources {
            assert_eq!(
                checker.check(&claim("Bob", &[source])).unwrap().finding,
                None
            );
        }
    }
}

#[test]
fn contradictory_or_invalid_corrections_are_refused() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 1.0, 2.0)]);
    for flags in [
        vec![correction(vec![], "Bob")],
        vec![correction(std::iter::once(0..1).collect(), "")],
        vec![
            correction(std::iter::once(0..2).collect(), "Bob"),
            correction(std::iter::once(1..2).collect(), "Carol"),
        ],
        vec![
            correction(std::iter::once(1..1).collect(), "Bob"),
            correction(std::iter::once(1..1).collect(), "Carol"),
        ],
    ] {
        assert!(matches!(
            Grounder::new(words.iter(), &flags, 3.0, 0),
            Err(GroundingError::InvalidFlags)
        ));
    }
}

#[test]
fn corrected_verification_claims_never_replace_transcript_evidence() {
    let words = words(&[("Alice", 0.0, 1.0)]);
    let flags = vec![ReviewFlag {
        identifier: Sha256Digest([9; 32]),
        content: FlagContent::Verification(crate::review::VerificationFlag {
            artifact: crate::note::NoteStage::Analysis,
            claim_index: 0,
            claim_digest: Sha256Digest([8; 32]),
            record_text: "private claim".into(),
            finding: Finding::Unsourced,
            sources: vec![],
        }),
        resolution: Resolution::Corrected {
            text: "Bob".into(),
            at: UtcInstant { secs: 1 },
        },
    }];
    let mut checker = Grounder::new(words.iter(), &flags, 1.0, 0).unwrap();
    assert_eq!(
        checker
            .check(&claim("Alice", &["00:00-00:01"]))
            .unwrap()
            .finding,
        None
    );
    assert_eq!(
        checker
            .check(&claim("Bob", &["00:00-00:01"]))
            .unwrap()
            .finding,
        Some(Finding::Unsupported("Bob".into()))
    );
}
