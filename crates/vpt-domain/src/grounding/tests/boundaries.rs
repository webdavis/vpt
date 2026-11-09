use super::*;

#[test]
fn invalid_transcripts_and_flag_ranges_are_refused_without_panics() {
    for values in [
        vec![],
        vec![("", 0.0, 1.0)],
        vec![("a", -1.0, 1.0)],
        vec![("a", 2.0, 1.0)],
        vec![("a", 0.0, 3.0)],
        vec![("a", f64::NAN, 1.0)],
        vec![("a", 0.0, f64::INFINITY)],
        vec![("a", 1.0, 2.0), ("b", 0.0, 1.0)],
    ] {
        let words = words(&values);
        assert!(matches!(
            Grounder::new(words.iter(), &[], 2.0, 0),
            Err(GroundingError::InvalidTranscript)
        ));
    }
    let words = words(&[("a", 0.0, 1.0)]);
    for duration in [-1.0, f64::NAN, f64::INFINITY, 9_007_199_254_740_994.0] {
        assert!(matches!(
            Grounder::new(words.iter(), &[], duration, 0),
            Err(GroundingError::InvalidTranscript)
        ));
    }
    for range in [2..2, 0..2, std::ops::Range { start: 1, end: 0 }] {
        let mut flag = flag(Class::Numeric, Resolution::Open);
        let FlagContent::Lexical(content) = &mut flag.content else {
            unreachable!()
        };
        content.ranges = vec![range];
        assert!(matches!(
            Grounder::new(words.iter(), &[flag], 2.0, 0),
            Err(GroundingError::InvalidFlags)
        ));
    }
}

#[test]
fn source_boundaries_exclude_touching_words_but_include_instant_words() {
    let words = words(&[("Alice", 0.0, 1.0), ("Bob", 1.0, 1.0), ("Carol", 2.0, 3.0)]);
    let mut checker = Grounder::new(words.iter(), &[], 3.0, 0).unwrap();
    assert_eq!(
        checker
            .check(&claim("Bob", &["00:01-00:02"]))
            .unwrap()
            .finding,
        None
    );
    for text in ["Alice", "Carol"] {
        assert!(matches!(
            checker
                .check(&claim(text, &["00:01-00:02"]))
                .unwrap()
                .finding,
            Some(Finding::Unsupported(_))
        ));
    }
}

#[test]
fn insertion_flags_use_their_anchor_and_duplicate_ids_are_inherited_once() {
    let words = words(&[("Alice", 0.0, 1.0), ("Bob", 2.0, 3.0)]);
    for (range, source, expected) in [
        (1..1, "00:01-00:02", false),
        (1..1, "00:02-00:03", true),
        (2..2, "00:03-00:04", true),
    ] {
        let mut flag = flag(Class::Other, Resolution::Open);
        let FlagContent::Lexical(content) = &mut flag.content else {
            unreachable!()
        };
        content.ranges = vec![range];
        let flags = [flag.clone(), flag];
        let result = Grounder::new(words.iter(), &flags, 4.0, 0)
            .unwrap()
            .check(&claim("text", &[source]))
            .unwrap();
        assert_eq!(result.inherited_flags.len(), usize::from(expected));
    }
}

#[test]
fn limits_bound_input_and_total_work_across_claims() {
    let words = words(&[("Alice", 0.0, 1.0)]);
    assert!(matches!(
        Grounder::new(std::iter::repeat(&words[0]), &[], 1.0, 0),
        Err(GroundingError::CapacityExceeded)
    ));
    let oversized = super::words(&[("a", 0.0, 1.0)]);
    let mut oversized = oversized;
    oversized[0].text = "a".repeat(MAX_ITEMS + 1);
    assert!(matches!(
        Grounder::new(oversized.iter(), &[], 1.0, 0),
        Err(GroundingError::CapacityExceeded)
    ));
    let mut checker = Grounder::new(words.iter(), &[], 1.0, 0).unwrap();
    assert_eq!(
        checker.check(&claim(&"a".repeat(MAX_ITEMS + 1), &[])),
        Err(GroundingError::CapacityExceeded)
    );
    assert_eq!(
        checker.check(&claim("Alice", &vec!["00:00-00:01"; 257])),
        Err(GroundingError::CapacityExceeded)
    );
    checker.remaining = 2;
    assert_eq!(
        checker.check(&claim("Alice", &["00:00-00:01"])),
        Err(GroundingError::CapacityExceeded)
    );
    assert_eq!(
        checker.check(&claim("a", &[])),
        Err(GroundingError::CapacityExceeded)
    );
}

#[test]
fn evidence_token_expansion_is_bounded() {
    let words = words(&[("a ", 0.0, 1.0)]);
    let mut words = words;
    words[0].text = "a ".repeat(MAX_ITEMS / 2);
    words.push(words[0].clone());
    words.push(words[0].clone());
    let mut checker = Grounder::new(words.iter(), &[], 1.0, 0).unwrap();
    assert_eq!(
        checker.check(&claim("Alice", &["00:00-00:01"])),
        Err(GroundingError::CapacityExceeded)
    );
}

#[test]
fn context_distance_is_inclusive_but_does_not_expand_flag_inheritance() {
    let words = words(&[("Alice", 0.0, 1.0), ("42", 6.0, 7.0), ("99", 7.001, 8.0)]);
    let flags = vec![flag(Class::Numeric, Resolution::Open)];
    let mut checker = Grounder::new(words.iter(), &flags, 8.0, 5).unwrap();
    let result = checker.check(&claim("Alice 42", &["00:01-00:02"])).unwrap();
    assert_eq!(result.finding, None);
    assert!(result.inherited_flags.is_empty());
    assert_eq!(
        checker
            .check(&claim("42", &["00:00-00:01"]))
            .unwrap()
            .finding,
        None
    );
    assert!(matches!(
        checker
            .check(&claim("99", &["00:01-00:02"]))
            .unwrap()
            .finding,
        Some(Finding::Unsupported(_))
    ));
}

#[test]
fn proposal_checks_share_one_budget_across_sections_and_return_no_partial_acceptance() {
    use crate::proposal::Proposal;
    let words = words(&[("Alice", 0.0, 1.0)]);
    let mut checker = Grounder::new(words.iter(), &[], 1.0, 0).unwrap();
    checker.remaining = 5;
    let proposal = Proposal {
        by: "agent".into(),
        summary: vec![claim("a", &[])],
        actions: vec![claim("bbbb", &[])],
        tags: vec![],
        relations: vec![],
    };
    assert_eq!(
        checker.proposal(proposal),
        Err(GroundingError::CapacityExceeded)
    );
}
